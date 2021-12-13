use crate::config::LdapConfig;
use color_eyre::eyre::Result;
use ldap3::{Ldap, LdapConnAsync, LdapConnSettings, Scope, SearchEntry};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::*;
use url::Url;

pub struct LdapClient {
    ldap: Ldap,
    user_base_dn: String,
    user_filter: String,
}

#[derive(Serialize, Deserialize)]
pub struct LdapResult {
    /// Entry DN.
    pub dn: String,
    /// Attributes.
    pub attrs: HashMap<String, Vec<String>>,
    /// Binary-valued attributes.
    pub bin_attrs: HashMap<String, Vec<Vec<u8>>>,
}

impl LdapClient {
    #[instrument(skip(config))]
    pub async fn new(config: &LdapConfig) -> Result<Self> {
        info!("Beginning LDAP setup");

        let settings = LdapConnSettings::new()
            .set_starttls(config.starttls.unwrap_or(false))
            .set_no_tls_verify(config.no_tls_verify.unwrap_or(false));

        let (conn, mut ldap) =
            LdapConnAsync::from_url_with_settings(settings, &Url::parse(&config.uri)?).await?;
        ldap3::drive!(conn);
        info!("Connected to LDAP server");

        if config.bind_dn.is_some() {
            info!("Attempting bind to LDAP");
            let _ = ldap
                .simple_bind(
                    config.bind_dn.as_deref().unwrap(),
                    config.bind_password.as_deref().unwrap(),
                )
                .await?;
        }

        info!("Finished LDAP setup");
        Ok(Self {
            ldap,
            user_base_dn: config.user_base_dn.clone(),
            user_filter: config.user_filter.clone(),
        })
    }

    pub async fn get_users_in_group(
        &mut self,
        group_dn: &str,
    ) -> Result<Box<dyn Iterator<Item = LdapResult>>> {
        info!("Fetching users in group {}", group_dn);
        let (rs, _) = self
            .ldap
            .search(
                &self.user_base_dn,
                Scope::Subtree,
                &format!("(&{}(memberOf={}))", self.user_filter, group_dn),
                vec!["*"],
            )
            .await?
            .success()?;

        Ok(Box::new(rs.into_iter().map(SearchEntry::construct).map(
            |e| LdapResult {
                dn: e.dn,
                attrs: e.attrs,
                bin_attrs: e.bin_attrs,
            },
        )))
    }
}
