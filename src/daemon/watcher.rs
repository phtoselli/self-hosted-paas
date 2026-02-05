use std::sync::Arc;
use std::time::Duration;

use crate::daemon::DaemonState;
use crate::models::project::ProjectState;

/// Run the health check watcher loop
pub async fn run(state: Arc<DaemonState>) {
    let mut interval = tokio::time::interval(Duration::from_secs(30));

    loop {
        interval.tick().await;

        let projects = state.projects.read().await;
        for (slug, config) in projects.iter() {
            if !config.enabled {
                continue;
            }

            match state
                .docker
                .get_container_state(&config.container.container_name)
                .await
            {
                Ok(ProjectState::Stopped) => {
                    tracing::warn!("[{}] Stopped unexpectedly, restarting...", slug);
                    if let Err(e) = state
                        .docker
                        .inner()
                        .start_container(
                            &config.container.container_name,
                            None::<bollard::container::StartContainerOptions<String>>,
                        )
                        .await
                    {
                        tracing::error!("[{}] Failed to restart: {}", slug, e);
                    } else {
                        tracing::info!("[{}] Restarted successfully", slug);
                    }
                }
                Ok(ProjectState::Online) => {}
                Ok(s) => {
                    tracing::debug!("[{}] State: {}", slug, s);
                }
                Err(e) => {
                    tracing::error!("[{}] Health check failed: {}", slug, e);
                }
            }
        }
    }
}
