mod commands;
mod event_handlers;
mod types;

use commands::server;

use event_handlers::Handler;
use pterodactyl_api::client as ptero_client;
use serenity::all::{ClientBuilder, GatewayIntents, GuildId};
use std::{env, str::FromStr};
use types::Data;

#[tokio::main]
async fn main() {
  let discord_api_token: String = env::var("DISCORD_API_TOKEN").expect("missing DISCORD_API_TOKEN");
  let discord_guild_id = env::var("DISCORD_GUILD_ID").expect("missing DISCORD_GUILD_ID");
  let pterodactyl_url = env::var("PTERODACTYL_URL").expect("missing PTERODACTYL_URL");
  let pterodactyl_client_api_key = env::var("PTERODACTYL_CLIENT_API_KEY").expect("missing PTERODACTYL_CLIENT_API_KEY");

  // Pterodactyl

  let ptero_client = ptero_client::ClientBuilder::new(pterodactyl_url, pterodactyl_client_api_key).build();

  // Discord

  let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::GUILD_MESSAGE_REACTIONS;

  let framework = poise::Framework::builder()
    .options(poise::FrameworkOptions {
      commands: vec![server()],
      ..Default::default()
    })
    .setup(|ctx, _ready, framework| {
      Box::pin(async move {
        poise::builtins::register_in_guild(
          ctx,
          &framework.options().commands,
          GuildId::from_str(&discord_guild_id)?,
        )
        .await?;

        Ok(Data { ptero_client })
      })
    })
    .build();

  let client = ClientBuilder::new(discord_api_token, intents)
    .event_handler(Handler)
    .framework(framework)
    .await;

  client.unwrap().start().await.unwrap();
}
