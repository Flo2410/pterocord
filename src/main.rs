mod commands;
mod config;
mod event_handlers;
mod server;
mod server_config;
mod types;

use anyhow::Result;
use commands::server;
use config::Config;
use event_handlers::event_handler;
use pterodactyl_api::client as ptero_client;
use serenity::all::{ClientBuilder, GatewayIntents, GuildId};
use std::{env, str::FromStr, sync::Arc, time::Duration};
use tokio::{sync::RwLock, time::sleep};
use types::Data;

use crate::server::Server;

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
  let servers = Arc::new(RwLock::new(
    config
      .servers
      .iter()
      .map(|server| Server::new(Arc::new(RwLock::new(server.clone())), ptero_client.clone()))
      .collect(),
  ));

  // Discord
  let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::GUILD_MESSAGE_REACTIONS;
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

  // Start update loop
  let http = client.http.clone();
  let task = tokio::spawn(async move {
    loop {
      for server in servers.read().await.iter() {
        let _ = server.update_msg(&http).await;
      }

      sleep(Duration::from_secs(10)).await;
    }
  });

  // Start discord client
  client.start().await.unwrap();

  task.abort();

  Ok(())
}
