use axum::routing::{delete, get, post, put};
use axum::Router;
use std::sync::Arc;
use tokio::net::{TcpListener, UnixListener};

use crate::config::global::GlobalConfig;
use crate::daemon::DaemonState;
use crate::error::DockyardError;
use crate::ipc::handlers;

pub async fn run(state: Arc<DaemonState>, config: &GlobalConfig) -> Result<(), DockyardError> {
    let ipc_router = create_router(Arc::clone(&state));
    let webhook_router = create_webhook_router(Arc::clone(&state));

    // Remove old socket file if exists
    let socket_path = &config.daemon.socket_path;
    let _ = std::fs::remove_file(socket_path);

    let uds_listener = UnixListener::bind(socket_path)?;
    tracing::info!("IPC server listening on {}", socket_path.display());

    // Set socket permissions
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(socket_path, std::fs::Permissions::from_mode(0o660))?;
    }

    let webhook_addr = format!("0.0.0.0:{}", config.daemon.webhook_port);
    let tcp_listener = TcpListener::bind(&webhook_addr).await?;
    tracing::info!("Webhook server listening on {}", webhook_addr);

    let ipc_service = ipc_router.into_make_service();
    let webhook_service = webhook_router.into_make_service();

    tokio::select! {
        result = axum::serve(uds_listener, ipc_service) => {
            if let Err(e) = result {
                tracing::error!("IPC server error: {}", e);
            }
        }
        result = axum::serve(tcp_listener, webhook_service) => {
            if let Err(e) = result {
                tracing::error!("Webhook server error: {}", e);
            }
        }
    }

    Ok(())
}

fn create_router(state: Arc<DaemonState>) -> Router {
    Router::new()
        .route("/api/health", get(handlers::health))
        .route("/api/projects", get(handlers::list_projects))
        .route("/api/projects", post(handlers::deploy_project))
        .route("/api/projects/{slug}", get(handlers::get_project))
        .route("/api/projects/{slug}", delete(handlers::delete_project))
        .route(
            "/api/projects/{slug}/rebuild",
            post(handlers::rebuild_project),
        )
        .route(
            "/api/projects/{slug}/start",
            post(handlers::start_project),
        )
        .route("/api/projects/{slug}/stop", post(handlers::stop_project))
        .route("/api/projects/{slug}/logs", get(handlers::get_logs))
        .route("/api/config", get(handlers::get_config))
        .route("/api/config", put(handlers::update_config))
        .with_state(state)
}

fn create_webhook_router(state: Arc<DaemonState>) -> Router {
    Router::new()
        .route(
            "/webhook/{slug}",
            post(crate::daemon::webhook::handle_webhook),
        )
        .with_state(state)
}
