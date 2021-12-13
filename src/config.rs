use color_eyre::eyre::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ExternalId {
    pub auth_provider: String,
    pub external_id_template: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct MatrixConfig {
    pub server_name: String,
    pub homeserver_url: String,
    pub mxid: String,
    pub password: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct LdapConfig {
    pub uri: String,
    pub no_tls_verify: Option<bool>,
    pub starttls: Option<bool>,
    pub bind_dn: Option<String>,
    pub bind_password: Option<String>,
    pub user_base_dn: String,
    pub user_filter: String,
    pub localpart_template: String,
    pub create_missing_users: bool,
    pub synapse_external_ids: Option<Vec<ExternalId>>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub matrix: MatrixConfig,
    pub ldap: LdapConfig,
    pub git_repository: String,
}

impl Config {
    pub fn load<P: AsRef<std::path::Path> + std::fmt::Debug>(path: P) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: Self = serde_yaml::from_str(&contents)?;
        Ok(config)
    }
}
