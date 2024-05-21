use crate::{
  server::{Server, ServerActionButton},
  types::{Data, Error},
};
use log::{debug, info};
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
      info!("Logged in as '{}'", data_about_bot.user.name);

      for server_arc in data.servers.iter() {
        let server = server_arc.read().await;
        let channel_name = &server.config.read().await.discord_channle_name;

        debug!("Calling init for '{}'", channel_name);
        server.init(&ctx).await?;
      }
    }

    FullEvent::ReactionAdd { add_reaction } => {
      debug!(
        "Added reaction '{}' in '{}'",
        add_reaction.emoji, add_reaction.channel_id
      );
    }

    FullEvent::InteractionCreate { interaction } => {
      let component = interaction.clone().message_component().unwrap();
      let custom_id = ServerActionButton::from_str(&component.data.custom_id)?;

      let server = Server::find_by_discord_channel_id(&data.servers, &component.channel_id)
        .await?
        .read()
        .await;

      let channel_name = &server.config.read().await.discord_channle_name;
      info!("Button '{}' was pressed for '{}'", custom_id, channel_name);

      match custom_id {
        ServerActionButton::Start => server.send_power_signal(PowerSignal::Start).await?,
        ServerActionButton::Stop => server.send_power_signal(PowerSignal::Stop).await?,
        ServerActionButton::Restart => server.send_power_signal(PowerSignal::Restart).await?,
        ServerActionButton::Kill => server.send_power_signal(PowerSignal::Kill).await?,
      };

      debug!("Respond to button press '{}' for '{}'", custom_id, channel_name);
      component
        .create_response(&ctx, CreateInteractionResponse::Acknowledge)
        .await?;
    }

    _ => {}
  }
  Ok(())
}
