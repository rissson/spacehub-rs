use crate::config;
use crate::ldap::LdapClient;
use crate::matrix::MatrixClient;
use async_recursion::async_recursion;
use color_eyre::eyre::{eyre, Result};
use matrix_sdk::ruma::UserId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::convert::TryFrom;
use std::fs;
use std::path::Path;
use tracing::*;

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone)]
pub struct ExternalId {
    /// The authentication provider to which the user is associated.
    pub auth_provider: String,
    /// The ID known to the auth provider associated with this user.
    pub external_id: String,
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Clone)]
pub struct UserMetadata {
    pub mxid: String,
    pub power_level: i32,
    pub external_ids: Vec<ExternalId>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
struct LdapGroupMetadata {
    dn: String,
    power_level: i32,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct RoomMetadata {
    pub id: Option<String>,
    pub alias: Option<String>,
    extra_aliases: Vec<String>,
    visibility: String,
    ldap_groups: Vec<LdapGroupMetadata>,
    admins: Vec<String>,
    users: HashSet<UserMetadata>,
}

#[derive(Debug)]
pub struct SpaceFolder {
    metadata: Option<RoomMetadata>,
    pub rooms: Vec<RoomMetadata>,
    children: Vec<Box<SpaceFolder>>,
}

fn walkdir(path: &Path) -> Result<Box<dyn Iterator<Item = std::fs::DirEntry>>> {
    Ok(Box::new(fs::read_dir(path)?.filter_map(|e| e.ok()).filter(
        |e| !e.file_name().to_str().unwrap_or(".").starts_with('.'),
    )))
}

impl LdapGroupMetadata {
    async fn get_users_metadatas_for_group(
        &self,
        ldap_client: &mut LdapClient,
        localpart_template: &str,
        mx_server_name: &str,
        synapse_external_ids: Option<&Vec<config::ExternalId>>,
    ) -> Result<HashSet<UserMetadata>> {
        let mut users_metadatas = HashSet::new();

        let users = ldap_client.get_users_in_group(&self.dn).await?;
        for user in users {
            let mut env = minijinja::Environment::new();
            let template = format!("@{}:{}", localpart_template, mx_server_name);
            env.add_template("mxid", &template)?;
            let mut ctx = BTreeMap::new();
            ctx.insert("user", user);
            let mxid = env.get_template("mxid").unwrap().render(&ctx).unwrap();

            let mut external_ids = vec![];
            if synapse_external_ids.is_some() {
                for external_id in synapse_external_ids.unwrap() {
                    env.add_template("external_id", &external_id.external_id_template)?;
                    let ext_id = env
                        .get_template("external_id")
                        .unwrap()
                        .render(&ctx)
                        .unwrap();
                    external_ids.push(ExternalId {
                        auth_provider: external_id.auth_provider.clone(),
                        external_id: ext_id,
                    });
                }
            }

            users_metadatas.insert(UserMetadata {
                mxid,
                power_level: self.power_level,
                external_ids,
            });
        }

        Ok(users_metadatas)
    }
}

impl RoomMetadata {
    // It is actually used to get default values for metadatas
    #[allow(dead_code)]
    fn default() -> Self {
        let default = std::default::Default::default();
        Self {
            visibility: "private".to_string(),
            ..default
        }
    }
}

impl SpaceFolder {
    fn new_rec(root: &Path) -> Result<Box<Self>> {
        debug!("Starting to process folder at {}", root.display());
        let mut space_folder = Box::new(Self {
            metadata: None,
            rooms: vec![],
            children: vec![],
        });

        for entry in walkdir(root)? {
            if entry.file_type()?.is_file() {
                if let Some(file_name) = entry.file_name().to_str() {
                    if file_name == "metadata.yml" || file_name == "metadata.yaml" {
                        let contents = std::fs::read_to_string(entry.path())?;
                        let metadata: RoomMetadata = serde_yaml::from_str(&contents)?;
                        space_folder.metadata = Some(metadata);
                    } else if file_name.starts_with('!') || file_name.starts_with('#') {
                        let contents = std::fs::read_to_string(entry.path())?;
                        let mut metadata: RoomMetadata = serde_yaml::from_str(&contents)?;
                        if file_name.starts_with('!') {
                            metadata.id = Some(file_name.to_string());
                        } else {
                            metadata.alias = Some(file_name.to_string());
                        }
                        space_folder.rooms.push(metadata);
                    } else {
                        info!("Unsupported file found at {}", entry.path().display());
                    }
                }
            } else if entry.file_type()?.is_dir() {
                space_folder
                    .children
                    .push(SpaceFolder::new_rec(entry.path().as_path())?);
            } else if entry.file_type()?.is_symlink() {
                info!(
                    "Symlinks are not supported (yet), found at {}",
                    entry.path().display()
                );
            }
        }

        Ok(space_folder)
    }

    pub fn new(root: &Path) -> Result<Vec<Box<Self>>> {
        info!("Starting to process folder at {}", root.display());

        let mut space_folders = vec![];

        for entry in walkdir(root)? {
            if entry.file_type()?.is_dir() {
                let folder = SpaceFolder::new_rec(entry.path().as_path())?;
                space_folders.push(folder);
            } else {
                info!(
                    "File {} has been ignored as it's not a directory.",
                    entry.path().display()
                );
            }
        }

        info!("Finished processing folder at {}", root.display());

        Ok(space_folders)
    }

    pub fn check(&self) -> Result<()> {
        if self.metadata.is_none() {
            return Err(eyre!("Folder should contain a metadata.yml"));
        }

        if self.metadata.as_ref().unwrap().id.is_none()
            && self.metadata.as_ref().unwrap().alias.is_none()
        {
            return Err(eyre!(
                "Folder should have a room ID or alias defined in metadata.yml"
            ));
        }

        let check_user = |mxid: &str| {
            let _ = UserId::try_from(mxid).expect(&format!("Couldn't parse MXID for {}", mxid));
        };

        for user in &self.metadata.as_ref().unwrap().users {
            check_user(&user.mxid);
        }

        for room in &self.rooms {
            for user in &room.users {
                check_user(&user.mxid);
            }
        }

        for child in &self.children {
            child.check()?;
        }

        Ok(())
    }

    #[async_recursion]
    pub async fn populate_rooms_users(
        &mut self,
        ldap_client: &mut LdapClient,
        localpart_template: &str,
        mx_server_name: &str,
        synapse_external_ids: Option<&'async_recursion Vec<config::ExternalId>>,
    ) -> Result<()> {
        info!(
            "Fetching users for room {} {}",
            self.metadata
                .as_ref()
                .unwrap()
                .id
                .as_ref()
                .unwrap_or(&String::new()),
            self.metadata
                .as_ref()
                .unwrap()
                .alias
                .as_ref()
                .unwrap_or(&String::new())
        );

        let mut users = HashSet::new();
        for group in &self.metadata.as_ref().unwrap().ldap_groups {
            users.extend(
                group.get_users_metadatas_for_group(
                    ldap_client,
                    localpart_template,
                    mx_server_name,
                    synapse_external_ids,
                )
                .await?,
            );
        }
        self.metadata.as_mut().unwrap().users.extend(users);

        for room in &mut self.rooms {
            info!(
                "Fetching users for room {} {}",
                room.id.as_ref().unwrap_or(&String::new()),
                room.alias.as_ref().unwrap_or(&String::new())
            );
            for group in &room.ldap_groups {
                room.users.extend(
                    group.get_users_metadatas_for_group(
                        ldap_client,
                        localpart_template,
                        mx_server_name,
                        synapse_external_ids,
                    )
                    .await?,
                );
            }
        }

        for child in &mut self.children {
            child
                .populate_rooms_users(
                    ldap_client,
                    localpart_template,
                    mx_server_name,
                    synapse_external_ids,
                )
                .await?;
        }

        Ok(())
    }

    pub fn get_all_users(&self) -> HashSet<UserMetadata> {
        let users = self.metadata.as_ref().unwrap().users.clone();

        let users = self.rooms.iter().fold(users, |mut acc, room| {
            acc.extend(room.users.clone());
            acc
        });

        self.children.iter().fold(users, |mut acc, child| {
            acc.extend(child.get_all_users());
            acc
        })
    }

    #[async_recursion]
    pub async fn folders_to_matrix(
        &self,
        matrix_client: &MatrixClient,
        parent: Option<&str>,
    ) -> Result<()> {

        let room_id = "";

        for child in &self.children {
            child.folders_to_matrix(matrix_client, Some(room_id)).await?;
        }
        Ok(())
    }
}
