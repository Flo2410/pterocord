use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct ServerConfig {
  pub ptero_server_id: String,
  pub discord_channel_id: String,
  pub discord_channle_name: String,
}
