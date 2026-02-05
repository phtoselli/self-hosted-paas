use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectState {
    Building,
    Starting,
    Online,
    Offline,
    Stopped,
    Error,
    Rebuilding,
}

impl std::fmt::Display for ProjectState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectState::Building => write!(f, "Building"),
            ProjectState::Starting => write!(f, "Starting"),
            ProjectState::Online => write!(f, "Online"),
            ProjectState::Offline => write!(f, "Offline"),
            ProjectState::Stopped => write!(f, "Stopped"),
            ProjectState::Error => write!(f, "Error"),
            ProjectState::Rebuilding => write!(f, "Rebuilding"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectStatus {
    pub slug: String,
    pub name: String,
    pub state: ProjectState,
    pub container_id: Option<String>,
    pub uptime_secs: Option<u64>,
    pub memory_usage_mb: Option<f64>,
    pub cpu_percent: Option<f64>,
    pub url: Option<String>,
    pub host_port: u16,
    pub container_port: u16,
    pub network_mode: String,
    pub last_deploy: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}
