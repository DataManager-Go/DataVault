#![allow(dead_code)]
use serde::{Deserialize, Serialize};

use async_std::io::prelude::*;
use async_std::path::PathBuf;
use async_std::{
    fs::{self, File},
    path::Path,
};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub preview: Preview,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ServerConfig {
    pub allow_registration: bool,
    pub file_output_path: String,
    pub external_url: String,
    pub raw_file_agents: Option<Vec<String>>,
    pub html_files: Option<String>,
    pub cors_allow: Option<Vec<String>>,
    pub max_preview_filesize: Option<u64>,
    pub listen_address: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Preview {
    pub ace_theme: Option<String>,
}

impl Config {
    /// Create a new config object
    pub async fn new() -> Result<Self, String> {
        let config_file = std::env::var("DVAULT_CONFIG")
            .map(|i| Path::new(&i).to_owned())
            .unwrap_or(Self::get_config_file().await?);

        debug!("Using config {} ", config_file.to_str().unwrap());

        let config = if !config_file.exists().await
            // Check if file is empty
            || fs::metadata(&config_file)
                .await
                .map(|i| i.len())
                .unwrap_or(1)
                == 0
        {
            Self::default().save().await?
        } else {
            let conf_data = fs::read_to_string(&config_file)
                .await
                .map_err(|e| e.to_string())?;

            toml::from_str(&conf_data).map_err(|e| e.to_string())?
        };

        Ok(config)
    }

    // Save the config
    pub async fn save(self) -> Result<Self, String> {
        let config_file = Self::get_config_file().await?;

        let s = toml::to_string_pretty(&self).map_err(|e| e.to_string())?;
        let mut f = File::create(&config_file)
            .await
            .map_err(|e| e.to_string())?;
        f.write_all(&s.as_bytes())
            .await
            .map_err(|e| e.to_string())?;

        debug!("Config saved");
        Ok(self)
    }

    // load a config
    pub async fn load(&mut self) -> Result<(), String> {
        let config_file = Self::get_config_file().await?;

        let conf_data = fs::read_to_string(&config_file)
            .await
            .map_err(|e| e.to_string())?;
        *self = toml::from_str(&conf_data).map_err(|e| e.to_string())?;

        Ok(())
    }

    // Create missing folders and return the config file
    pub async fn get_config_file() -> Result<PathBuf, String> {
        let conf_dir: PathBuf = Path::new("./").join("data");

        if !conf_dir.exists().await {
            fs::create_dir_all(&conf_dir)
                .await
                .map_err(|e| e.to_string())?;
        }

        Ok(conf_dir.join("config.toml"))
    }
}
