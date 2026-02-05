use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use crate::config::paths;
use crate::daemon::DaemonState;
use crate::docker::build;

#[derive(Debug)]
pub enum Job {
    Deploy { slug: String },
    Rebuild { slug: String, commit_sha: Option<String> },
    Stop { slug: String },
    Delete { slug: String },
}

/// Run the scheduler loop
pub async fn run(mut rx: mpsc::Receiver<Job>, state: Arc<DaemonState>) {
    let building: Arc<RwLock<HashSet<String>>> = Arc::new(RwLock::new(HashSet::new()));

    while let Some(job) = rx.recv().await {
        let state = Arc::clone(&state);
        let building = Arc::clone(&building);

        tokio::spawn(async move {
            match job {
                Job::Deploy { slug } => {
                    {
                        let mut set = building.write().await;
                        if set.contains(&slug) {
                            tracing::warn!("Deploy for '{}' already in progress", slug);
                            return;
                        }
                        set.insert(slug.clone());
                    }

                    if let Err(e) = execute_deploy(&state, &slug).await {
                        tracing::error!("Deploy failed for '{}': {}", slug, e);
                    }

                    building.write().await.remove(&slug);
                }
                Job::Rebuild { slug, commit_sha } => {
                    {
                        let mut set = building.write().await;
                        if set.contains(&slug) {
                            tracing::warn!("Rebuild for '{}' already in progress", slug);
                            return;
                        }
                        set.insert(slug.clone());
                    }

                    if let Err(e) =
                        execute_rebuild(&state, &slug, commit_sha.as_deref()).await
                    {
                        tracing::error!("Rebuild failed for '{}': {}", slug, e);
                    }

                    building.write().await.remove(&slug);
                }
                Job::Stop { slug } => {
                    if let Err(e) = state.stop_project(&slug).await {
                        tracing::error!("Stop failed for '{}': {}", slug, e);
                    }
                }
                Job::Delete { slug } => {
                    if let Err(e) = state.delete_project(&slug).await {
                        tracing::error!("Delete failed for '{}': {}", slug, e);
                    }
                }
            }
        });
    }
}

async fn execute_deploy(state: &DaemonState, slug: &str) -> anyhow::Result<()> {
    let (repo_url, branch, container_name, image_name, host_port, container_port, env_vars) = {
        let projects = state.projects.read().await;
        let config = projects
            .get(slug)
            .ok_or_else(|| anyhow::anyhow!("Project '{}' not found", slug))?;
        (
            config.repo_url.clone(),
            config.branch.clone(),
            config.container.container_name.clone(),
            config.container.image_name.clone(),
            config.domain.host_port,
            config.domain.container_port,
            config.container.env_vars.clone(),
        )
    };

    let repo_dir = paths::project_repo_dir(slug);

    tracing::info!("[{}] Cloning repository...", slug);
    crate::utils::git_clone(&repo_url, &repo_dir, &branch).await?;

    tracing::info!("[{}] Building Docker image...", slug);
    let dockerfile = build::find_dockerfile(&repo_dir)?;
    let tag = format!("{}:latest", image_name);
    build::build_image(&state.docker, &repo_dir, &tag, &dockerfile).await?;

    tracing::info!("[{}] Starting container...", slug);
    let container_id = state
        .docker
        .create_and_start_container(&container_name, &tag, host_port, container_port, &env_vars)
        .await?;

    tracing::info!(
        "[{}] Deployed (container: {}, port: {})",
        slug,
        &container_id[..12.min(container_id.len())],
        host_port
    );

    {
        let mut projects = state.projects.write().await;
        if let Some(config) = projects.get_mut(slug) {
            config.updated_at = chrono::Utc::now();
            let _ = config.save();
        }
    }

    Ok(())
}

async fn execute_rebuild(
    state: &DaemonState,
    slug: &str,
    _commit_sha: Option<&str>,
) -> anyhow::Result<()> {
    let (branch, container_name, image_name, host_port, container_port, env_vars) = {
        let projects = state.projects.read().await;
        let config = projects
            .get(slug)
            .ok_or_else(|| anyhow::anyhow!("Project '{}' not found", slug))?;
        (
            config.branch.clone(),
            config.container.container_name.clone(),
            config.container.image_name.clone(),
            config.domain.host_port,
            config.domain.container_port,
            config.container.env_vars.clone(),
        )
    };

    let repo_dir = paths::project_repo_dir(slug);

    // Pull latest code
    tracing::info!("[{}] Pulling latest code...", slug);
    let sha = crate::utils::git_pull(&repo_dir, &branch).await?;
    tracing::info!("[{}] Latest commit: {}", slug, &sha[..7.min(sha.len())]);

    // Build new image
    let timestamp = chrono::Utc::now().timestamp();
    let new_tag = format!("{}:build-{}", image_name, timestamp);
    tracing::info!("[{}] Building new image...", slug);
    let dockerfile = build::find_dockerfile(&repo_dir)?;
    build::build_image(&state.docker, &repo_dir, &new_tag, &dockerfile).await?;

    // Blue-green: start new container
    let temp_container = format!("{}-new", container_name);
    let temp_port = crate::utils::find_available_port()?;

    tracing::info!("[{}] Starting new container (blue-green)...", slug);
    let _ = state
        .docker
        .create_and_start_container(
            &temp_container,
            &new_tag,
            temp_port,
            container_port,
            &env_vars,
        )
        .await?;

    // Wait for stabilization
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    if !state
        .docker
        .is_container_running(&temp_container)
        .await?
    {
        let _ = state.docker.remove_container(&temp_container).await;
        let _ = state.docker.remove_image(&new_tag).await;
        return Err(anyhow::anyhow!("New container failed to start"));
    }

    // Stop old container
    tracing::info!("[{}] Switching to new container...", slug);
    let _ = state.docker.stop_container(&container_name).await;
    let _ = state.docker.remove_container(&container_name).await;

    // Rename new container
    state
        .docker
        .inner()
        .rename_container(
            &temp_container,
            bollard::container::RenameContainerOptions {
                name: container_name.clone(),
            },
        )
        .await?;

    // Re-tag image as latest
    state
        .docker
        .inner()
        .tag_image(
            &new_tag,
            Some(bollard::image::TagImageOptions {
                repo: image_name.as_str(),
                tag: "latest",
            }),
        )
        .await?;

    let _ = state.docker.remove_image(&new_tag).await;

    // Update timestamp
    {
        let mut projects = state.projects.write().await;
        if let Some(config) = projects.get_mut(slug) {
            config.updated_at = chrono::Utc::now();
            let _ = config.save();
        }
    }

    tracing::info!("[{}] Rebuild complete (zero-downtime)", slug);
    Ok(())
}
