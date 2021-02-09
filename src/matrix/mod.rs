use crate::Config;
use color_eyre::eyre::Result;
use ruma::RoomId;
use ruma_client::Client;
use tracing::*;

pub struct Matrix {
    client: Client,
}

impl Matrix {
    #[instrument(skip(config))]
    pub async fn new(config: Config<'_>) -> Result<Self> {
        info!("Beginning Matrix Setup");
        let homeserver_url = config.homeserver_url.to_string().parse()?;
        let client = Client::new(homeserver_url, None);

        let _ = client
            .clone()
            .log_in(&config.mxid, &config.password, None, Some("spacehub"))
            .await?;
        Ok(Self { client })
    }

    pub async fn create_space(
        name: String,
        description: Option<String>,
        avatar_url: Option<String>,
    ) {
    }
    pub async fn set_child(parent_id: RoomId, child_id: RoomId) {}
    pub async fn get_spaces_toplevel() {}
    pub async fn get_space_childs(space_id: RoomId) {}
}
