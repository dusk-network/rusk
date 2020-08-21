//! Config structure module.
use super::default_ctants::*;
use super::errors::ConfigError;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::from_str;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub(crate) port: String,
    pub(crate) host_address: String,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            host_address: String::from(HOST_ADDRESS),
            port: String::from(PORT),
        }
    }
}

impl Config {
    // Creates a config based on a JSON configfile.
    pub fn from_configfile(path: &str) -> Result<Config> {
        match File::open(&Path::new(path)) {
            Ok(mut file) => {
                let mut contents = String::new();
                file.read_to_string(&mut contents)?;
                let config: Config = from_str(&contents)?;
                Ok(config)
            }
            Err(e) => Err(ConfigError::Io(e))?,
        }
    }
}
