use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::config::paths;
use crate::error::DockyardError;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectConfig {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub repo_url: String,
    #[serde(default = "default_branch")]
    pub branch: String,
    pub network_mode: NetworkMode,
    pub domain: DomainConfig,
    pub container: ContainerConfig,
    pub webhook: WebhookConfig,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_branch() -> String {
    "main".to_string()
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum NetworkMode {
    LocalOnly,
    Public,
}

impl std::fmt::Display for NetworkMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkMode::LocalOnly => write!(f, "Local Only"),
            NetworkMode::Public => write!(f, "Public"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DomainConfig {
    pub hostname: Option<String>,
    pub container_port: u16,
    pub host_port: u16,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContainerConfig {
    pub image_name: String,
    pub container_name: String,
    #[serde(default = "default_dockerfile")]
    pub dockerfile_path: String,
    #[serde(default)]
    pub env_vars: HashMap<String, String>,
}

fn default_dockerfile() -> String {
    "Dockerfile".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WebhookConfig {
    pub secret: String,
    pub github_webhook_id: Option<u64>,
}

impl ProjectConfig {
    pub fn new(
        name: String,
        slug: String,
        repo_url: String,
        branch: String,
        network_mode: NetworkMode,
        hostname: Option<String>,
        container_port: u16,
        host_port: u16,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            slug: slug.clone(),
            repo_url,
            branch,
            network_mode,
            domain: DomainConfig {
                hostname,
                container_port,
                host_port,
            },
            container: ContainerConfig {
                image_name: format!("dockyard/{}", slug),
                container_name: format!("dockyard-{}", slug),
                dockerfile_path: default_dockerfile(),
                env_vars: HashMap::new(),
            },
            webhook: WebhookConfig {
                secret: crate::utils::generate_webhook_secret(),
                github_webhook_id: None,
            },
            created_at: now,
            updated_at: now,
            enabled: true,
        }
    }

    pub fn load(slug: &str) -> Result<Self, DockyardError> {
        let path = paths::project_config_path(slug);
        if !path.exists() {
            return Err(DockyardError::ProjectNotFound(slug.to_string()));
        }
        let content = std::fs::read_to_string(&path)?;
        let config: ProjectConfig = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<(), DockyardError> {
        let dir = paths::project_dir(&self.slug);
        std::fs::create_dir_all(&dir)?;
        let path = paths::project_config_path(&self.slug);
        let content =
            toml::to_string_pretty(self).map_err(|e| DockyardError::Config(e.to_string()))?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub fn delete(slug: &str) -> Result<(), DockyardError> {
        let dir = paths::project_dir(slug);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        Ok(())
    }

    pub fn list_slugs() -> Result<Vec<String>, DockyardError> {
        let dir = paths::projects_dir();
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut slugs = Vec::new();
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    let config_path = paths::project_config_path(name);
                    if config_path.exists() {
                        slugs.push(name.to_string());
                    }
                }
            }
        }
        slugs.sort();
        Ok(slugs)
    }

    pub fn load_all() -> Result<Vec<Self>, DockyardError> {
        let slugs = Self::list_slugs()?;
        let mut projects = Vec::new();
        for slug in slugs {
            match Self::load(&slug) {
                Ok(config) => projects.push(config),
                Err(e) => {
                    tracing::warn!("Failed to load project '{}': {}", slug, e);
                }
            }
        }
        Ok(projects)
    }
}
