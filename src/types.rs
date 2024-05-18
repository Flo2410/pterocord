use pterodactyl_api::client::Client;

// User data, which is stored and accessible in all command invocations
pub struct Data {
  pub ptero_client: Client,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;
