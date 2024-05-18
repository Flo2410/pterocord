use serenity::{
  all::{Context, EventHandler, Ready},
  async_trait,
};

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
  async fn ready(&self, _: Context, ready: Ready) {
    println!("Bot is ready: {} is connected!", ready.user.name);
  }
}
