use std::sync::Arc;

use pterodactyl_api::client::Client;
use tokio::sync::RwLock;

use crate::server::Server;

// User data, which is stored and accessible in all command invocations
pub struct Data {
  pub servers: Arc<RwLock<Vec<Server>>>,
  pub ptero_client: Arc<RwLock<Client>>,
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;
