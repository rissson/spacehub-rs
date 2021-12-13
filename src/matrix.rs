use crate::folders::UserMetadata;
use crate::config::MatrixConfig;
use color_eyre::eyre::{ErrReport, Result};
use std::convert::TryFrom;
use tracing::*;
use matrix_sdk::{
    ruma::{
        api::{
            client::{error as ruma_api_client_error, r0::profile},
            error as ruma_api_error,
        },
        UserId,
    },
    Client, ClientConfig, HttpError, RequestConfig,
};
use synapse_admin_api::users as synapse_users;

pub struct MatrixClient {
    client: Client,
}

impl MatrixClient {
    #[instrument(skip(config))]
    pub async fn new(config: &MatrixConfig) -> Result<Self> {
        info!("Beginning Matrix setup");

        let client_config = ClientConfig::new().user_agent("spacehub")?;
        let client = Client::new_with_config(config.homeserver_url.parse()?, client_config)?;

        info!("Logging in to Matrix");
        let _response = client
            .login(&config.mxid, &config.password, None, Some("spacehub"))
            .await?;

        info!("Finished setting up Matrix");
        Ok(Self { client })
    }

    async fn user_exists(&self, user_id: &UserId) -> Result<bool> {
        let profile_request = profile::get_profile::Request::new(&user_id);

        let profile = match self
            .client
            .send(profile_request, Some(RequestConfig::new().force_auth()))
            .await
        {
            Err(HttpError::ClientApi(ruma_api_error::FromHttpResponseError::Http(
                ruma_api_error::ServerError::Known(e),
            ))) if e.kind == ruma_api_client_error::ErrorKind::NotFound => None,
            Err(e) => return Err(ErrReport::try_from(e)?),
            Ok(_) => Some(0),
        };

        Ok(profile.is_some())
    }

    pub async fn create_user_if_missing(&self, user: &UserMetadata) -> Result<()> {
        let user_id = UserId::try_from(user.mxid.clone())?;
        if self.user_exists(&user_id).await? {
            return Ok(())
        }

        info!("Creating user {}", user.mxid);
        let mut register_request = synapse_users::create_or_modify::v2::Request::new(&user_id, None);
        register_request.external_ids = Some(vec![]);
        for external_id in &user.external_ids {
            register_request.external_ids.as_mut().unwrap().push(synapse_admin_api::users::create_or_modify::v2::ExternalId {
                auth_provider: external_id.auth_provider.clone(),
                external_id: external_id.external_id.clone(),
            })
        }
        let _created_user = self.client.send(register_request, None).await?;

        Ok(())
    }

    /* pub async fn create_space(
        &self,
        name: String,
        description: Option<String>,
        avatar_url: Option<String>,
    ) {
        let space = create_room::Request {
            creation_content: create_room::CreationContent {
                federate: true,
                predecessor: None,
                room_type: Some("".to_string()),
            },
            initial_state: &vec![
                AnyInitialStateEvent::RoomHistoryVisibility(InitialStateEvent {
                    state_key: "".to_string(),
                    content: HistoryVisibilityEventContent {
                        history_visibility: HistoryVisibility::Invited,
                    },
                }),
                AnyInitialStateEvent::RoomAvatar(InitialStateEvent {
                    state_key: "".to_string(),
                    content: AvatarEventContent {
                        info: None,
                        url: avatar_url.unwrap_or_default(),
                    },
                }),
            ],
            name: Some(&name),
            preset: Some(create_room::RoomPreset::PrivateChat),
            invite: &vec![],
            invite_3pid: &vec![],
            is_direct: false,
            room_alias_name: None,
            room_version: None,
            topic: description.as_ref().map(|x| &**x),
            visibility: Visibility::Private,
            power_level_content_override: Some(
                PowerLevelsEventContent {
                    events_default: int!(100),
                    ..Default::default()
                }
                .into(),
            ),
        };

        let room = match self.client.request(space).await {
            Ok(a) => a,
            _ => {
                error!("Failed to create space");
                return;
            }
        };
    } */
    // pub async fn set_child(parent_id: RoomId, child_id: RoomId) {}
}
