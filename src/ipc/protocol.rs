use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::config::project::NetworkMode;
use crate::models::project::ProjectStatus;

// ---- Requests ----

#[derive(Debug, Serialize, Deserialize)]
pub struct DeployRequest {
    pub repo_url: String,
    pub branch: String,
    pub network_mode: NetworkMode,
    pub hostname: Option<String>,
    pub container_port: u16,
    #[serde(default)]
    pub env_vars: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigUpdateRequest {
    pub github_ssh_key_path: Option<String>,
    pub github_api_token: Option<String>,
    pub cloudflare_tunnel_token: Option<String>,
    pub cloudflare_enabled: Option<bool>,
}

// ---- Responses ----

#[derive(Debug, Serialize, Deserialize)]
pub struct DeployResponse {
    pub slug: String,
    pub name: String,
    pub url: Option<String>,
    pub webhook_url: String,
    pub host_port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectListResponse {
    pub projects: Vec<ProjectStatus>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectDetailResponse {
    pub status: ProjectStatus,
    pub repo_url: String,
    pub branch: String,
    pub webhook_secret: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogsResponse {
    pub logs: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub uptime_secs: u64,
    pub project_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigResponse {
    pub github_ssh_key_path: Option<String>,
    pub github_api_token_set: bool,
    pub cloudflare_enabled: bool,
    pub cloudflare_tunnel_id: Option<String>,
    pub webhook_port: u16,
    pub socket_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SuccessResponse {
    pub message: String,
}
