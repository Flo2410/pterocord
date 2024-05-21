mod commands;
mod config;
mod event_handlers;
mod server;
mod server_config;
mod types;

use crate::server::Server;
use anyhow::Result;
use commands::server;
use config::Config;
use event_handlers::event_handler;
use pterodactyl_api::client as ptero_client;
use serenity::all::{ClientBuilder, GatewayIntents, GuildId};
use std::{env, str::FromStr, sync::Arc};
use tokio::sync::RwLock;
use types::Data;

#[tokio::main]
async fn main() -> Result<()> {
  let discord_api_token: String = env::var("DISCORD_API_TOKEN").expect("missing DISCORD_API_TOKEN");
  let discord_guild_id = env::var("DISCORD_GUILD_ID").expect("missing DISCORD_GUILD_ID");
  let pterodactyl_url = env::var("PTERODACTYL_URL").expect("missing PTERODACTYL_URL");
  let pterodactyl_client_api_key = env::var("PTERODACTYL_CLIENT_API_KEY").expect("missing PTERODACTYL_CLIENT_API_KEY");

  // Config file
  let config = Config::load();

  // Pterodactyl
  let ptero_client = Arc::new(RwLock::new(
    ptero_client::ClientBuilder::new(pterodactyl_url, pterodactyl_client_api_key).build(),
  ));

  println!(
    "Pterodactyl connected as {}",
    ptero_client.read().await.get_account_details().await?.username
  );

  // Server
  let servers = config
    .servers
    .iter()
    .map(|server_config| {
      Arc::new(RwLock::new(Server::new(
        Arc::new(RwLock::new(server_config.clone())),
        ptero_client.clone(),
      )))
    })
    .collect::<Vec<_>>();

  // Discord
  let intents = GatewayIntents::GUILDS;
  let servers_clone = servers.clone();
  let framework = poise::Framework::builder()
    .setup(|ctx, _ready, framework| {
      Box::pin(async move {
        poise::builtins::register_in_guild(
          ctx,
          &framework.options().commands,
          GuildId::from_str(&discord_guild_id)?,
        )
        .await?;

        Ok(Data {
          servers: servers_clone,
          ptero_client,
        })
      })
    })
    .options(poise::FrameworkOptions {
      commands: vec![server()],
      event_handler: |ctx, event, framework, data| Box::pin(event_handler(ctx, event, framework, data)),
      ..Default::default()
    })
    .build();

  let mut client = ClientBuilder::new(discord_api_token, intents)
    .framework(framework)
    .await?;

  // Start the websocket client for all servers
  for server_arc in servers.iter() {
    server_arc
      .read()
      .await
      .start_websocket_client(server_arc.clone(), client.http.clone())
      .await;
  }

  // Start discord client
  client.start().await.unwrap();

  Ok(())
}
