use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::config::paths;
use crate::error::DockyardError;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GlobalConfig {
    #[serde(default)]
    pub github: GitHubConfig,
    #[serde(default)]
    pub cloudflare: CloudflareConfig,
    #[serde(default)]
    pub daemon: DaemonConfig,
    #[serde(default)]
    pub caddy: CaddyConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GitHubConfig {
    pub ssh_key_path: Option<PathBuf>,
    pub api_token: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CloudflareConfig {
    pub tunnel_token: Option<String>,
    pub tunnel_id: Option<String>,
    #[serde(default)]
    pub enabled: bool,
}

impl Default for CloudflareConfig {
    fn default() -> Self {
        Self {
            tunnel_token: None,
            tunnel_id: None,
            enabled: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DaemonConfig {
    #[serde(default = "default_webhook_port")]
    pub webhook_port: u16,
    #[serde(default = "default_socket_path")]
    pub socket_path: PathBuf,
    #[serde(default = "default_log_level")]
    pub log_level: String,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            webhook_port: default_webhook_port(),
            socket_path: default_socket_path(),
            log_level: default_log_level(),
        }
    }
}

fn default_webhook_port() -> u16 {
    9876
}

fn default_socket_path() -> PathBuf {
    paths::socket_path()
}

fn default_log_level() -> String {
    "info".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CaddyConfig {
    #[serde(default = "default_caddy_admin")]
    pub admin_api: String,
}

impl Default for CaddyConfig {
    fn default() -> Self {
        Self {
            admin_api: default_caddy_admin(),
        }
    }
}

fn default_caddy_admin() -> String {
    "http://localhost:2019".to_string()
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            github: GitHubConfig::default(),
            cloudflare: CloudflareConfig::default(),
            daemon: DaemonConfig::default(),
            caddy: CaddyConfig::default(),
        }
    }
}

impl GlobalConfig {
    pub fn load() -> Result<Self, DockyardError> {
        let path = paths::global_config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let config: GlobalConfig = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(GlobalConfig::default())
        }
    }

    pub fn save(&self) -> Result<(), DockyardError> {
        let path = paths::global_config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content =
            toml::to_string_pretty(self).map_err(|e| DockyardError::Config(e.to_string()))?;
        std::fs::write(&path, content)?;
        Ok(())
    }
}
