use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde::Deserialize;
use std::sync::Arc;

use crate::daemon::DaemonState;
use crate::ipc::protocol::*;

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    pub tail: Option<u32>,
}

pub async fn health(State(state): State<Arc<DaemonState>>) -> Json<HealthResponse> {
    let projects = state.project_count().await;
    let uptime = state.uptime_secs();
    Json(HealthResponse {
        status: "ok".to_string(),
        uptime_secs: uptime,
        project_count: projects,
    })
}

pub async fn list_projects(
    State(state): State<Arc<DaemonState>>,
) -> Result<Json<ProjectListResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.list_project_statuses().await {
        Ok(projects) => Ok(Json(ProjectListResponse { projects })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )),
    }
}

pub async fn get_project(
    State(state): State<Arc<DaemonState>>,
    Path(slug): Path<String>,
) -> Result<Json<ProjectDetailResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.get_project_detail(&slug).await {
        Ok(detail) => Ok(Json(detail)),
        Err(e) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )),
    }
}

pub async fn deploy_project(
    State(state): State<Arc<DaemonState>>,
    Json(req): Json<DeployRequest>,
) -> Result<(StatusCode, Json<DeployResponse>), (StatusCode, Json<ErrorResponse>)> {
    match state.deploy_project(req).await {
        Ok(resp) => Ok((StatusCode::CREATED, Json(resp))),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )),
    }
}

pub async fn rebuild_project(
    State(state): State<Arc<DaemonState>>,
    Path(slug): Path<String>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.rebuild_project(&slug).await {
        Ok(()) => Ok(Json(SuccessResponse {
            message: format!("Rebuild started for '{}'", slug),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )),
    }
}

pub async fn start_project(
    State(state): State<Arc<DaemonState>>,
    Path(slug): Path<String>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.start_project(&slug).await {
        Ok(()) => Ok(Json(SuccessResponse {
            message: format!("Project '{}' started", slug),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )),
    }
}

pub async fn stop_project(
    State(state): State<Arc<DaemonState>>,
    Path(slug): Path<String>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.stop_project(&slug).await {
        Ok(()) => Ok(Json(SuccessResponse {
            message: format!("Project '{}' stopped", slug),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )),
    }
}

pub async fn delete_project(
    State(state): State<Arc<DaemonState>>,
    Path(slug): Path<String>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.delete_project(&slug).await {
        Ok(()) => Ok(Json(SuccessResponse {
            message: format!("Project '{}' deleted", slug),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )),
    }
}

pub async fn get_logs(
    State(state): State<Arc<DaemonState>>,
    Path(slug): Path<String>,
    Query(query): Query<LogsQuery>,
) -> Result<Json<LogsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let tail = query.tail.unwrap_or(100);
    match state.get_project_logs(&slug, tail).await {
        Ok(logs) => Ok(Json(LogsResponse { logs })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )),
    }
}

pub async fn get_config(
    State(state): State<Arc<DaemonState>>,
) -> Result<Json<ConfigResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.get_config_info().await {
        Ok(config) => Ok(Json(config)),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )),
    }
}

pub async fn update_config(
    State(state): State<Arc<DaemonState>>,
    Json(req): Json<ConfigUpdateRequest>,
) -> Result<Json<SuccessResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.update_config(req).await {
        Ok(()) => Ok(Json(SuccessResponse {
            message: "Configuration updated".to_string(),
        })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: e.to_string(),
            }),
        )),
    }
}
