use poise::CreateReply;
use serenity::all::{Colour, CreateEmbed};

use crate::types::{Context, Error};

/// Manage servers for the bot.
#[poise::command(slash_command, subcommands("list"))]
pub async fn server(_ctx: Context<'_>, _arg: String) -> Result<(), Error> {
  Ok(())
}

/// List all available servers.
#[poise::command(slash_command)]
pub async fn list(ctx: Context<'_>) -> Result<(), Error> {
  let client = &ctx.data().ptero_client;

  let servers = client.list_servers().await?;

  let mut fields: Vec<(String, String, bool)> = vec![];

  for server_struct in servers.iter() {
    let server_resources = client
      .get_server(&server_struct.identifier)
      .get_resources()
      .await
      .unwrap();

    let server_desc_default = server_struct.description.clone().unwrap_or_default();
    let server_desc = if server_desc_default.is_empty() {
      String::from(" - ")
    } else {
      server_desc_default
    };

    fields.push((
      server_struct.name.clone(),
      format!(
        "ID: {}\nDescription: {}\nStatus: {:#?}",
        server_struct.identifier, server_desc, server_resources.current_state,
      ),
      false,
    ));
  }

  let embed = CreateEmbed::new()
    .title("Pterocord")
    .description("Servers:")
    .thumbnail("https://i.imgur.com/aBDbmTu.png")
    .color(Colour::new(0x0000BB))
    .fields(fields);

  let reply = CreateReply::default().embed(embed);

  ctx.send(reply).await?;

  Ok(())
}
