use anyhow::{Ok, Result};
use pterodactyl_api::client::{Client, PowerSignal, ServerState};
use serenity::all::Message;
use serenity::all::{
  ButtonStyle, CacheHttp, ChannelId, Color, CreateActionRow, CreateButton, CreateEmbed, CreateMessage, EditChannel,
  EditMessage, GetMessages,
};
use std::sync::Arc;
use strum_macros::{Display, EnumString};
use tokio::sync::RwLock;

use crate::server_config::ServerConfig;

#[derive(Debug, PartialEq, EnumString, Display)]
pub enum ServerActionButton {
  Start,
  Stop,
  Restart,
  Kill,
}

pub struct Server {
  pub config: Arc<RwLock<ServerConfig>>,
  pub ptero_client: Arc<RwLock<Client>>,

  discord_msg: RwLock<Message>,
}

impl Server {
  pub fn new(config: Arc<RwLock<ServerConfig>>, ptero_client: Arc<RwLock<Client>>) -> Self {
    Self {
      config,
      discord_msg: RwLock::new(Message::default()),
      ptero_client: ptero_client,
    }
  }

  pub async fn find_by_discord_channel_id<'a>(
    servers: &'a Vec<Self>,
    discord_channel_id: &'a ChannelId,
  ) -> Result<&'a Self> {
    for server in servers.iter() {
      if server.config.read().await.discord_channel_id == discord_channel_id.to_string() {
        return Ok(server);
      }
    }

    Err(anyhow::Error::msg(format!(
      "Server not found with channel id: {}",
      discord_channel_id.to_string(),
    )))
  }

  pub async fn init(&self, cache_http: impl CacheHttp) -> Result<()> {
    let channel = ChannelId::new(self.config.read().await.discord_channel_id.parse()?);

    // Clear channel
    let msgs = channel.messages(&cache_http, GetMessages::default()).await?;
    if !msgs.is_empty() {
      channel.delete_messages(&cache_http.http(), msgs).await?;
      println!("cleared the channel");
    } else {
      println!("no messages to clear");
    }

    // Set name
    let new_channel_name = self.build_channel().await?;
    channel.edit(&cache_http, new_channel_name).await?;
    println!("updated the name");

    let (embed, buttons) = self.build_msg().await?;

    let new_msg = CreateMessage::new().embed(embed).components(buttons);

    let mut dc_msg = self.discord_msg.write().await;
    *dc_msg = channel.send_message(cache_http, new_msg).await?;
    println!("sent new msg");

    Ok(())
  }

  pub async fn update_channel(&self, cache_http: impl CacheHttp) -> Result<()> {
    let channel = ChannelId::new(self.config.read().await.discord_channel_id.parse()?);
    let channel_name = &self.config.read().await.discord_channle_name;
    let edit_channel = self.build_channel().await?;

    channel.edit(cache_http, edit_channel).await?;

    println!("Updated channel {}", channel_name);

    Ok(())
  }

  pub async fn update_msg(&self, cache_http: impl CacheHttp) -> Result<()> {
    let mut msg = self.discord_msg.write().await;
    let (embed, buttons) = self.build_msg().await?;

    let edit_msg = EditMessage::new().embed(embed).components(buttons);
    (*msg).edit(cache_http, edit_msg).await?;

    println!(
      "Updated message in channel {}",
      self.config.read().await.discord_channle_name
    );

    Ok(())
  }

  async fn build_channel(&self) -> Result<EditChannel> {
    let ptero_server_lock = self.ptero_client.read().await;
    let ptero_server = ptero_server_lock.get_server(&self.config.read().await.ptero_server_id);
    let server_resources = ptero_server.get_resources().await?;

    Ok(EditChannel::new().name(format!(
      "{}-{}",
      chose_emoji(server_resources.current_state),
      self.config.read().await.discord_channle_name
    )))
  }

  async fn build_msg(&self) -> Result<(CreateEmbed, Vec<CreateActionRow>)> {
    // Get infos from Pterodactyl
    let ptero_server_lock = self.ptero_client.read().await;
    let ptero_server = ptero_server_lock.get_server(&self.config.read().await.ptero_server_id);
    let server_struct = ptero_server.get_details().await?;
    let server_resources = ptero_server.get_resources().await?;

    let server_desc_default = server_struct.description.clone().unwrap_or_default();
    let server_desc = if server_desc_default.is_empty() {
      String::from(" - ")
    } else {
      server_desc_default
    };

    let server_status = server_resources.current_state;

    let fields = vec![
      (String::from("Description:"), server_desc, false),
      (String::from("Status:"), format!("{:#?}", server_status), false),
    ];

    let embed = CreateEmbed::new()
      .title("Pterocord")
      .description(format!("{} {}", chose_emoji(server_status), &server_struct.name))
      .thumbnail("https://i.imgur.com/aBDbmTu.png")
      .color(chose_color(server_status))
      .fields(fields);

    let buttons = vec![CreateActionRow::Buttons(match server_status {
      ServerState::Offline => vec![CreateButton::new(ServerActionButton::Start.to_string())
        .label("Start Server")
        .style(ButtonStyle::Success)],

      ServerState::Running | ServerState::Starting => vec![
        CreateButton::new(ServerActionButton::Stop.to_string())
          .label("Stop Server")
          .style(ButtonStyle::Danger),
        CreateButton::new(ServerActionButton::Restart.to_string())
          .label("Restart Server")
          .style(ButtonStyle::Primary),
        CreateButton::new(ServerActionButton::Kill.to_string())
          .label("Kill Server")
          .style(ButtonStyle::Danger),
      ],

      ServerState::Stopping => vec![CreateButton::new(ServerActionButton::Kill.to_string())
        .label("Kill Server")
        .style(ButtonStyle::Danger)],
    })];

    Ok((embed, buttons))
  }

  pub async fn send_power_signal(&self, signal: PowerSignal) -> Result<()> {
    let ptero_server_lock = self.ptero_client.read().await;
    let ptero_server = ptero_server_lock.get_server(&self.config.read().await.ptero_server_id);
    ptero_server.send_power_signal(signal).await?;

    println!(
      "{} server {}",
      signal.to_string(),
      ptero_server.get_details().await?.name
    );

    Ok(())
  }
}

fn chose_emoji(state: ServerState) -> String {
  match state {
    ServerState::Offline => String::from("🔴"),
    ServerState::Running => String::from("🟢"),
    _ => String::from("🟡"),
  }
}

fn chose_color(state: ServerState) -> Color {
  match state {
    ServerState::Offline => Color::new(0x8b1300),
    ServerState::Running => Color::new(0x0B6623),
    _ => Color::new(0xFFCC32),
  }
}
