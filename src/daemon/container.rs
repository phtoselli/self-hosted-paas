use std::sync::Arc;

use crate::daemon::DaemonState;
use crate::error::DockyardError;
use crate::models::project::ProjectState;

/// Start all enabled projects on daemon startup
pub async fn start_all_projects(state: &Arc<DaemonState>) -> Result<(), DockyardError> {
    let projects = state.projects.read().await;

    for (slug, config) in projects.iter() {
        if !config.enabled {
            tracing::info!("[{}] Disabled, skipping", slug);
            continue;
        }

        let container_state = state
            .docker
            .get_container_state(&config.container.container_name)
            .await?;

        match container_state {
            ProjectState::Online => {
                tracing::info!("[{}] Already running", slug);
            }
            ProjectState::Stopped => {
                tracing::info!("[{}] Starting stopped container...", slug);
                state
                    .docker
                    .inner()
                    .start_container(
                        &config.container.container_name,
                        None::<bollard::container::StartContainerOptions<String>>,
                    )
                    .await?;
                tracing::info!("[{}] Started", slug);
            }
            _ => {
                tracing::info!("[{}] Container not found, queueing deploy...", slug);
                let _ = state
                    .scheduler_tx
                    .send(crate::daemon::scheduler::Job::Deploy {
                        slug: slug.clone(),
                    })
                    .await;
            }
        }
    }

    Ok(())
}
