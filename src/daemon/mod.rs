pub mod container;
pub mod proxy;
pub mod scheduler;
pub mod server;
pub mod tunnel;
pub mod watcher;
pub mod webhook;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

use crate::config::global::GlobalConfig;
use crate::config::project::ProjectConfig;
use crate::docker::DockerClient;
use crate::error::DockyardError;
use crate::ipc::protocol::*;
use crate::models::project::{ProjectState, ProjectStatus};

/// Shared daemon state accessible by all handlers
pub struct DaemonState {
    pub config: RwLock<GlobalConfig>,
    pub docker: DockerClient,
    pub projects: RwLock<HashMap<String, ProjectConfig>>,
    pub started_at: Instant,
    pub scheduler_tx: tokio::sync::mpsc::Sender<scheduler::Job>,
}

impl DaemonState {
    pub fn uptime_secs(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }

    pub async fn project_count(&self) -> usize {
        self.projects.read().await.len()
    }

    pub async fn list_project_statuses(&self) -> Result<Vec<ProjectStatus>, DockyardError> {
        let projects = self.projects.read().await;
        let mut statuses = Vec::new();

        for (slug, config) in projects.iter() {
            let state = self
                .docker
                .get_container_state(&config.container.container_name)
                .await
                .unwrap_or(ProjectState::Offline);

            let (memory, cpu) = if state == ProjectState::Online {
                self.docker
                    .get_container_stats(&config.container.container_name)
                    .await
                    .unwrap_or((0.0, 0.0))
            } else {
                (0.0, 0.0)
            };

            let uptime = if state == ProjectState::Online {
                self.docker
                    .get_container_uptime(&config.container.container_name)
                    .await
                    .unwrap_or(None)
            } else {
                None
            };

            let url = match (&config.network_mode, &config.domain.hostname) {
                (_, Some(hostname)) => Some(format!("https://{}", hostname)),
                (crate::config::project::NetworkMode::LocalOnly, None) => {
                    Some(format!("http://localhost:{}", config.domain.host_port))
                }
                _ => None,
            };

            statuses.push(ProjectStatus {
                slug: slug.clone(),
                name: config.name.clone(),
                state,
                container_id: None,
                uptime_secs: uptime,
                memory_usage_mb: Some(memory),
                cpu_percent: Some(cpu),
                url,
                host_port: config.domain.host_port,
                container_port: config.domain.container_port,
                network_mode: config.network_mode.to_string(),
                last_deploy: Some(config.updated_at),
                last_error: None,
            });
        }

        Ok(statuses)
    }

    pub async fn get_project_detail(
        &self,
        slug: &str,
    ) -> Result<ProjectDetailResponse, DockyardError> {
        let projects = self.projects.read().await;
        let config = projects
            .get(slug)
            .ok_or_else(|| DockyardError::ProjectNotFound(slug.to_string()))?;

        let state = self
            .docker
            .get_container_state(&config.container.container_name)
            .await
            .unwrap_or(ProjectState::Offline);

        let (memory, cpu) = if state == ProjectState::Online {
            self.docker
                .get_container_stats(&config.container.container_name)
                .await
                .unwrap_or((0.0, 0.0))
        } else {
            (0.0, 0.0)
        };

        let uptime = if state == ProjectState::Online {
            self.docker
                .get_container_uptime(&config.container.container_name)
                .await
                .unwrap_or(None)
        } else {
            None
        };

        let url = match (&config.network_mode, &config.domain.hostname) {
            (_, Some(hostname)) => Some(format!("https://{}", hostname)),
            (crate::config::project::NetworkMode::LocalOnly, None) => {
                Some(format!("http://localhost:{}", config.domain.host_port))
            }
            _ => None,
        };

        let status = ProjectStatus {
            slug: slug.to_string(),
            name: config.name.clone(),
            state,
            container_id: None,
            uptime_secs: uptime,
            memory_usage_mb: Some(memory),
            cpu_percent: Some(cpu),
            url,
            host_port: config.domain.host_port,
            container_port: config.domain.container_port,
            network_mode: config.network_mode.to_string(),
            last_deploy: Some(config.updated_at),
            last_error: None,
        };

        Ok(ProjectDetailResponse {
            status,
            repo_url: config.repo_url.clone(),
            branch: config.branch.clone(),
            webhook_secret: config.webhook.secret.clone(),
        })
    }

    pub async fn deploy_project(
        &self,
        req: DeployRequest,
    ) -> Result<DeployResponse, DockyardError> {
        let name = crate::utils::repo_name(&req.repo_url);
        let slug = crate::utils::slugify(&name);

        {
            let projects = self.projects.read().await;
            if projects.contains_key(&slug) {
                return Err(DockyardError::ProjectAlreadyExists(slug));
            }
        }

        let host_port = crate::utils::find_available_port()?;

        let config = ProjectConfig::new(
            name.clone(),
            slug.clone(),
            req.repo_url,
            req.branch,
            req.network_mode,
            req.hostname,
            req.container_port,
            host_port,
        );

        config.save()?;

        let _ = self
            .scheduler_tx
            .send(scheduler::Job::Deploy {
                slug: slug.clone(),
            })
            .await;

        {
            let mut projects = self.projects.write().await;
            projects.insert(slug.clone(), config);
        }

        let webhook_port = self.config.read().await.daemon.webhook_port;
        let webhook_url = format!("http://YOUR_SERVER:{}/webhook/{}", webhook_port, slug);

        Ok(DeployResponse {
            slug,
            name,
            url: Some(format!("http://localhost:{}", host_port)),
            webhook_url,
            host_port,
        })
    }

    pub async fn rebuild_project(&self, slug: &str) -> Result<(), DockyardError> {
        {
            let projects = self.projects.read().await;
            if !projects.contains_key(slug) {
                return Err(DockyardError::ProjectNotFound(slug.to_string()));
            }
        }

        let _ = self
            .scheduler_tx
            .send(scheduler::Job::Rebuild {
                slug: slug.to_string(),
                commit_sha: None,
            })
            .await;

        Ok(())
    }

    pub async fn start_project(&self, slug: &str) -> Result<(), DockyardError> {
        let projects = self.projects.read().await;
        let config = projects
            .get(slug)
            .ok_or_else(|| DockyardError::ProjectNotFound(slug.to_string()))?;

        self.docker
            .inner()
            .start_container(
                &config.container.container_name,
                None::<bollard::container::StartContainerOptions<String>>,
            )
            .await?;

        Ok(())
    }

    pub async fn stop_project(&self, slug: &str) -> Result<(), DockyardError> {
        let projects = self.projects.read().await;
        let config = projects
            .get(slug)
            .ok_or_else(|| DockyardError::ProjectNotFound(slug.to_string()))?;

        self.docker
            .stop_container(&config.container.container_name)
            .await?;

        Ok(())
    }

    pub async fn delete_project(&self, slug: &str) -> Result<(), DockyardError> {
        let container_name;
        let image_name;
        {
            let projects = self.projects.read().await;
            let config = projects
                .get(slug)
                .ok_or_else(|| DockyardError::ProjectNotFound(slug.to_string()))?;
            container_name = config.container.container_name.clone();
            image_name = config.container.image_name.clone();
        }

        let _ = self.docker.stop_container(&container_name).await;
        let _ = self.docker.remove_container(&container_name).await;
        let _ = self.docker.remove_image(&image_name).await;

        {
            let mut projects = self.projects.write().await;
            projects.remove(slug);
        }

        ProjectConfig::delete(slug)?;
        tracing::info!("Deleted project '{}'", slug);
        Ok(())
    }

    pub async fn get_project_logs(
        &self,
        slug: &str,
        tail: u32,
    ) -> Result<Vec<String>, DockyardError> {
        let projects = self.projects.read().await;
        let config = projects
            .get(slug)
            .ok_or_else(|| DockyardError::ProjectNotFound(slug.to_string()))?;

        self.docker
            .get_logs(&config.container.container_name, tail, false)
            .await
    }

    pub async fn get_config_info(&self) -> Result<ConfigResponse, DockyardError> {
        let config = self.config.read().await;
        Ok(ConfigResponse {
            github_ssh_key_path: config
                .github
                .ssh_key_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            github_api_token_set: config.github.api_token.is_some(),
            cloudflare_enabled: config.cloudflare.enabled,
            cloudflare_tunnel_id: config.cloudflare.tunnel_id.clone(),
            webhook_port: config.daemon.webhook_port,
            socket_path: config.daemon.socket_path.to_string_lossy().to_string(),
        })
    }

    pub async fn update_config(&self, req: ConfigUpdateRequest) -> Result<(), DockyardError> {
        let mut config = self.config.write().await;

        if let Some(path) = req.github_ssh_key_path {
            config.github.ssh_key_path = Some(path.into());
        }
        if let Some(token) = req.github_api_token {
            config.github.api_token = Some(token);
        }
        if let Some(token) = req.cloudflare_tunnel_token {
            config.cloudflare.tunnel_token = Some(token);
        }
        if let Some(enabled) = req.cloudflare_enabled {
            config.cloudflare.enabled = enabled;
        }

        config.save()?;
        Ok(())
    }
}

/// Main entry point for the daemon
pub async fn run() -> anyhow::Result<()> {
    tracing::info!("Starting dockyard daemon...");

    let config = GlobalConfig::load()?;

    let docker = DockerClient::connect()?;
    docker.ping().await?;
    tracing::info!("Connected to Docker daemon");

    crate::docker::network::ensure_network(&docker).await?;

    let (scheduler_tx, scheduler_rx) = tokio::sync::mpsc::channel(100);

    let project_configs = ProjectConfig::load_all()?;
    let mut projects = HashMap::new();
    for pc in project_configs {
        tracing::info!("Loaded project: {}", pc.slug);
        projects.insert(pc.slug.clone(), pc);
    }

    let state = Arc::new(DaemonState {
        config: RwLock::new(config.clone()),
        docker,
        projects: RwLock::new(projects),
        started_at: Instant::now(),
        scheduler_tx,
    });

    // Start all enabled projects
    container::start_all_projects(&state).await?;

    // Write PID file
    let pid = std::process::id();
    let pid_path = crate::config::paths::pid_file_path();
    if let Err(e) = std::fs::write(&pid_path, pid.to_string()) {
        tracing::warn!("Could not write PID file: {}", e);
    }

    // Start scheduler
    let scheduler_state = Arc::clone(&state);
    tokio::spawn(async move {
        scheduler::run(scheduler_rx, scheduler_state).await;
    });

    // Start health check watcher
    let watcher_state = Arc::clone(&state);
    tokio::spawn(async move {
        watcher::run(watcher_state).await;
    });

    // Start servers (IPC + webhook)
    let server_state = Arc::clone(&state);
    let server_config = config.clone();
    tokio::spawn(async move {
        if let Err(e) = server::run(server_state, &server_config).await {
            tracing::error!("Server error: {}", e);
        }
    });

    tracing::info!("Dockyard daemon is running");

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutdown signal received, stopping...");

    let _ = std::fs::remove_file(&pid_path);
    let socket_path = crate::config::paths::socket_path();
    let _ = std::fs::remove_file(&socket_path);

    tracing::info!("Dockyard daemon stopped");
    Ok(())
}
