use anyhow::{Ok, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

use std::{
  fs::{File, OpenOptions},
  io::BufReader,
  path::Path,
};

use crate::server_config::ServerConfig;

const CONFIG_FILE_PATH: &str = "./config.json";

#[derive(Serialize, Deserialize)]
pub struct Config {
  pub servers: Vec<ServerConfig>,
}

impl Config {
  pub fn load() -> Self {
    let file_path = Path::new(CONFIG_FILE_PATH);
    let file = open_json_file(&file_path);
    let config: Config = serde_json::from_reader(BufReader::new(&file)).expect("Could not parse manifest");
    config
  }

  pub fn add_server(&mut self, server_config: ServerConfig) -> Result<()> {
    self.servers.push(server_config);
    self.save()?;
    Ok(())
  }

  fn save(&self) -> Result<()> {
    serde_json::to_writer_pretty(
      OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(Path::new(CONFIG_FILE_PATH))?,
      &self,
    )?;
    Ok(())
  }
}

fn open_json_file(path: &Path) -> File {
  if !path.exists() {
    // let file = File::create(path).unwrap_or_else(|_| panic!("Could not create file: {}", path.display()));
    let file = OpenOptions::new()
      .write(true)
      .create(true)
      .open(path)
      .unwrap_or_else(|_| panic!("Could not create file: {}", path.display()));

    serde_json::to_writer_pretty(&file, &json!({"servers":[]})).expect("Could not write to file");
  }

  OpenOptions::new()
    .read(true)
    .open(path)
    .unwrap_or_else(|_| panic!("Could not open file: {}", path.display()))
}
