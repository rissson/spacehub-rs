use crate::Config;
use color_eyre::eyre::Result;
use ruma::{
    api::client::r0::room::{create_room, Visibility},
    events::{
        room::{
            avatar::AvatarEventContent,
            history_visibility::{HistoryVisibility, HistoryVisibilityEventContent},
            power_levels::PowerLevelsEventContent,
        },
        AnyInitialStateEvent, InitialStateEvent,
    },
    int, RoomId,
};
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
    }
    pub async fn set_child(parent_id: RoomId, child_id: RoomId) {}
    pub async fn get_spaces_toplevel() {}
    pub async fn get_space_childs(space_id: RoomId) {}
}
