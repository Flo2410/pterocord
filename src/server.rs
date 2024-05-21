use crate::server_config::ServerConfig;
use anyhow::Result;
use async_tungstenite::tungstenite::client::IntoClientRequest;
use async_tungstenite::tungstenite::http::header::ORIGIN;
use async_tungstenite::tungstenite::http::HeaderValue;
use local_ip_address::local_ip;
use log::{debug, info};
use pterodactyl_api::client::websocket::{PteroWebSocketHandle, PteroWebSocketListener, ServerStats};
use pterodactyl_api::client::{Client, PowerSignal, ServerState};
use serenity::all::{
  ButtonStyle, CacheHttp, ChannelId, Color, CreateActionRow, CreateButton, CreateEmbed, CreateMessage, EditChannel,
  EditMessage, GetMessages,
};
use serenity::all::{Http, Message};
use serenity::async_trait;
use std::str::FromStr;
use std::sync::Arc;
use strum_macros::{Display, EnumString};
use tokio::sync::RwLock;
use url::Url;

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
  pub server_state: ServerState,
}

impl Server {
  pub fn new(config: Arc<RwLock<ServerConfig>>, ptero_client: Arc<RwLock<Client>>) -> Self {
    Self {
      config,
      discord_msg: RwLock::new(Message::default()),
      ptero_client,
      server_state: ServerState::Offline,
    }
  }

  pub async fn find_by_discord_channel_id<'a>(
    servers: &'a Vec<Arc<RwLock<Self>>>,
    discord_channel_id: &'a ChannelId,
  ) -> Result<&'a Arc<RwLock<Self>>> {
    for server_arc in servers.iter() {
      let server = server_arc.read().await;
      if server.config.read().await.discord_channel_id == discord_channel_id.to_string() {
        return Ok(&server_arc);
      }
    }

    Err(anyhow::Error::msg(format!(
      "Server not found with channel id: {}",
      discord_channel_id.to_string(),
    )))
  }

  pub async fn init(&self, cache_http: impl CacheHttp) -> Result<()> {
    let channel = ChannelId::new(self.config.read().await.discord_channel_id.parse()?);
    let channel_name = &self.config.read().await.discord_channle_name;
    // Clear channel
    let msgs = channel.messages(&cache_http, GetMessages::default()).await?;
    if !msgs.is_empty() {
      channel.delete_messages(&cache_http.http(), msgs).await?;
      debug!("Cleared the channel '{}'", channel_name);
    } else {
      debug!("No messages to clear in '{}'", channel_name);
    }

    // Set name
    // let new_channel_name = self.build_channel().await?;
    // channel.edit(&cache_http.http(), new_channel_name).await?;
    // debug!("Updated the name for '{}'", channel_name);

    let (embed, buttons) = self.build_msg().await?;

    let new_msg = CreateMessage::new().embed(embed).components(buttons);

    let mut dc_msg = self.discord_msg.write().await;
    *dc_msg = channel.send_message(cache_http, new_msg).await?;
    debug!("Sent new message to '{}'", channel_name);

    Ok(())
  }

  pub async fn update_channel(&self, cache_http: impl CacheHttp) -> Result<()> {
    let channel = ChannelId::new(self.config.read().await.discord_channel_id.parse()?);
    let channel_name = &self.config.read().await.discord_channle_name;
    let edit_channel = self.build_channel().await?;

    if self.server_state == ServerState::Starting || self.server_state == ServerState::Stopping {
      return Ok(());
    }

    let a = channel.edit(cache_http.http(), edit_channel);
    debug!("Sent channel update for '{}'", channel_name);
    let res = a.await;
    if res.is_ok() {
      debug!("Updated channel '{}'", channel_name);
    } else {
      debug!("Error updating channel '{}'", channel_name);
    }

    Ok(())
  }

  pub async fn update_msg(&self, cache_http: impl CacheHttp) -> Result<()> {
    let mut msg = self.discord_msg.write().await;
    let (embed, buttons) = self.build_msg().await?;

    let edit_msg = EditMessage::new().embed(embed).components(buttons);
    (*msg).edit(cache_http, edit_msg).await?;

    debug!(
      "Updated message in channel '{}'",
      self.config.read().await.discord_channle_name
    );

    Ok(())
  }

  async fn build_channel(&self) -> Result<EditChannel> {
    Ok(EditChannel::new().name(format!(
      "{}-{}",
      chose_emoji(self.server_state),
      self.config.read().await.discord_channle_name
    )))
  }

  async fn build_msg(&self) -> Result<(CreateEmbed, Vec<CreateActionRow>)> {
    // Get infos from Pterodactyl
    let ptero_server_lock = self.ptero_client.read().await;
    let ptero_server = ptero_server_lock.get_server(&self.config.read().await.ptero_server_id);
    let server_struct = ptero_server.get_details().await?;

    let server_desc_default = server_struct.description.clone().unwrap_or_default();
    let server_desc = if server_desc_default.is_empty() {
      String::from(" - ")
    } else {
      server_desc_default
    };

    let fields = vec![
      (String::from("Description:"), server_desc, false),
      (String::from("Status:"), format!("{:#?}", self.server_state), false),
    ];

    let embed = CreateEmbed::new()
      .title("Pterocord")
      .description(format!("{} {}", chose_emoji(self.server_state), &server_struct.name))
      .thumbnail("https://i.imgur.com/aBDbmTu.png")
      .color(chose_color(self.server_state))
      .fields(fields);

    let buttons = vec![CreateActionRow::Buttons(match self.server_state {
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

    debug!(
      "Sent power signal '{}' to server '{}'",
      signal.to_string(),
      ptero_server.get_details().await?.name
    );

    Ok(())
  }

  pub async fn start_websocket_client(&self, server_arc: Arc<RwLock<Server>>, http: Arc<Http>) {
    let ptero_client = self.ptero_client.clone();
    let config = self.config.clone();

    // Start WebSocket client
    tokio::spawn(async move {
      let ptero_client = ptero_client.read().await;
      let ptero_server = ptero_client.get_server(&config.read().await.ptero_server_id);
      info!(
        "Staring websocket client for server '{}'",
        config.read().await.discord_channle_name
      );
      let _ = ptero_server
        .run_websocket_loop(
          |url| async move {
            let local_ip = local_ip().expect("Failed to get local IP address").to_string();

            // Convert the URL into a request
            let mut request = Url::from_str(url.as_str())
              .unwrap()
              .into_client_request()
              .expect("Failed to create request");

            // Add the custom "Origin" header
            request.headers_mut().insert(ORIGIN, HeaderValue::from_str(&local_ip)?);

            Ok(async_tungstenite::tokio::connect_async(request).await?.0)
          },
          WebsocketListener { server_arc, http },
        )
        .await;
    });
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

struct WebsocketListener {
  server_arc: Arc<RwLock<Server>>,
  http: Arc<Http>,
}

#[async_trait]
impl<H: PteroWebSocketHandle> PteroWebSocketListener<H> for WebsocketListener {
  async fn on_ready(&mut self, _handle: &mut H) -> pterodactyl_api::Result<()> {
    info!(
      "WebSocket is ready for '{}'",
      self.server_arc.read().await.config.read().await.discord_channle_name
    );
    pterodactyl_api::Result::Ok(())
  }

  async fn on_status(&mut self, _handle: &mut H, status: ServerState) -> pterodactyl_api::Result<()> {
    info!(
      "Received status for '{}': {:?}",
      self.server_arc.read().await.config.read().await.discord_channle_name,
      status
    );
    self.server_arc.write().await.server_state = status;
    let _ = self.server_arc.read().await.update_msg(&self.http).await;

    pterodactyl_api::Result::Ok(())
  }

  async fn on_console_output(&mut self, _handle: &mut H, _output: &str) -> pterodactyl_api::Result<()> {
    // debug!("Console output for {}: {}", self.server_arc.read().await.config.read().await.discord_channle_name, output);
    pterodactyl_api::Result::Ok(())
  }

  async fn on_stats(&mut self, _handle: &mut H, _stats: ServerStats) -> pterodactyl_api::Result<()> {
    // debug!("Received stats for {}: {:?}", self.server_arc.read().await.config.read().await.discord_channle_name, stats);
    pterodactyl_api::Result::Ok(())
  }
}
