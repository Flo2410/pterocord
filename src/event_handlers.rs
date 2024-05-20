use crate::{
  server::{Server, ServerActionButton},
  types::{Data, Error},
};
use pterodactyl_api::client::PowerSignal;
use serenity::all::{Context, CreateInteractionResponse, FullEvent};
use std::str::FromStr;

pub async fn event_handler(
  ctx: &Context,
  event: &FullEvent,
  _framework: poise::FrameworkContext<'_, Data, Error>,
  data: &Data,
) -> Result<(), Error> {
  match event {
    FullEvent::Ready { data_about_bot, .. } => {
      println!("Logged in as {}", data_about_bot.user.name);

      for server in data.servers.read().await.iter() {
        println!("calling init");
        server.init(&ctx).await?;
      }
    }

    FullEvent::ReactionAdd { add_reaction } => {
      println!("Added reaction {} in {}", add_reaction.emoji, add_reaction.channel_id);
    }

    FullEvent::InteractionCreate { interaction } => {
      let component = interaction.clone().message_component().unwrap();
      let custom_id = ServerActionButton::from_str(&component.data.custom_id)?;

      let servers = data.servers.read().await;
      let server = Server::find_by_discord_channel_id(&servers, &component.channel_id).await?;

      println!(
        "Button '{}' was pressed for '{}'",
        custom_id,
        server.config.read().await.discord_channle_name
      );

      match custom_id {
        ServerActionButton::Start => server.send_power_signal(PowerSignal::Start).await?,
        ServerActionButton::Stop => server.send_power_signal(PowerSignal::Stop).await?,
        ServerActionButton::Restart => server.send_power_signal(PowerSignal::Restart).await?,
        ServerActionButton::Kill => server.send_power_signal(PowerSignal::Kill).await?,
      };

      println!("Responde");
      component
        .create_response(&ctx, CreateInteractionResponse::Acknowledge)
        .await?;
    }

    _ => {}
  }
  Ok(())
}
