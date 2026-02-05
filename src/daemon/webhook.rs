use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Json;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::sync::Arc;

use crate::daemon::scheduler::Job;
use crate::daemon::DaemonState;
use crate::ipc::protocol::ErrorResponse;
use crate::models::events::GitHubPushEvent;

type HmacSha256 = Hmac<Sha256>;

pub async fn handle_webhook(
    State(state): State<Arc<DaemonState>>,
    Path(slug): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let (webhook_secret, tracked_branch) = {
        let projects = state.projects.read().await;
        let config = projects.get(&slug).ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Project '{}' not found", slug),
                }),
            )
        })?;
        (config.webhook.secret.clone(), config.branch.clone())
    };

    // Verify HMAC signature
    if let Some(signature_header) = headers.get("x-hub-signature-256") {
        let signature = signature_header.to_str().unwrap_or("");
        let expected_signature = signature.strip_prefix("sha256=").unwrap_or(signature);

        let mut mac = HmacSha256::new_from_slice(webhook_secret.as_bytes()).map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "HMAC error".to_string(),
                }),
            )
        })?;
        mac.update(&body);

        let computed = hex::encode(mac.finalize().into_bytes());

        if !constant_time_eq(computed.as_bytes(), expected_signature.as_bytes()) {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "Invalid signature".to_string(),
                }),
            ));
        }
    } else {
        tracing::warn!("Webhook for '{}' received without signature header", slug);
    }

    // Parse push event
    let event: GitHubPushEvent = serde_json::from_slice(&body).map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Invalid payload: {}", e),
            }),
        )
    })?;

    // Check if push is to the tracked branch
    if let Some(branch) = event.branch() {
        if branch == tracked_branch {
            tracing::info!(
                "Webhook triggered rebuild for '{}' (branch: {}, commit: {})",
                slug,
                branch,
                &event.after[..7.min(event.after.len())]
            );

            let _ = state
                .scheduler_tx
                .send(Job::Rebuild {
                    slug: slug.clone(),
                    commit_sha: Some(event.after),
                })
                .await;

            return Ok(StatusCode::OK);
        }
    }

    tracing::debug!(
        "Webhook for '{}': ignoring push to {}",
        slug,
        event.git_ref
    );
    Ok(StatusCode::OK)
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}
