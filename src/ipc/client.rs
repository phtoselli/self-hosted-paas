use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::{Method, Request};
use hyper_util::rt::TokioIo;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::net::UnixStream;

use crate::config::paths;
use crate::error::DockyardError;
use crate::ipc::protocol::*;
use crate::models::project::ProjectStatus;

pub struct IpcClient {
    socket_path: String,
}

impl IpcClient {
    pub fn new() -> Self {
        Self {
            socket_path: paths::socket_path().to_string_lossy().to_string(),
        }
    }

    async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&impl Serialize>,
    ) -> Result<T, DockyardError> {
        let stream = UnixStream::connect(&self.socket_path)
            .await
            .map_err(|_| DockyardError::DaemonNotRunning)?;

        let io = TokioIo::new(stream);

        let (mut sender, conn) = hyper::client::conn::http1::handshake(io)
            .await
            .map_err(|e| DockyardError::Ipc(e.to_string()))?;

        tokio::spawn(async move {
            if let Err(e) = conn.await {
                tracing::error!("IPC connection error: {}", e);
            }
        });

        let body_bytes = if let Some(b) = body {
            Full::new(Bytes::from(serde_json::to_vec(b)?))
        } else {
            Full::new(Bytes::new())
        };

        let req = Request::builder()
            .method(method)
            .uri(format!("http://localhost{}", path))
            .header("content-type", "application/json")
            .body(body_bytes)
            .map_err(|e| DockyardError::Ipc(e.to_string()))?;

        let response = sender
            .send_request(req)
            .await
            .map_err(|e| DockyardError::Ipc(e.to_string()))?;

        let status = response.status();
        let body_bytes = response
            .into_body()
            .collect()
            .await
            .map_err(|e| DockyardError::Ipc(e.to_string()))?
            .to_bytes();

        if !status.is_success() {
            if let Ok(err) = serde_json::from_slice::<ErrorResponse>(&body_bytes) {
                return Err(DockyardError::Ipc(err.error));
            }
            return Err(DockyardError::Ipc(format!("HTTP {}", status)));
        }

        serde_json::from_slice(&body_bytes).map_err(|e| DockyardError::Ipc(e.to_string()))
    }

    pub async fn health(&self) -> Result<HealthResponse, DockyardError> {
        self.request::<HealthResponse>(Method::GET, "/api/health", None::<&()>)
            .await
    }

    pub async fn list_projects(&self) -> Result<Vec<ProjectStatus>, DockyardError> {
        let resp: ProjectListResponse = self
            .request(Method::GET, "/api/projects", None::<&()>)
            .await?;
        Ok(resp.projects)
    }

    pub async fn get_project(&self, slug: &str) -> Result<ProjectDetailResponse, DockyardError> {
        self.request(
            Method::GET,
            &format!("/api/projects/{}", slug),
            None::<&()>,
        )
        .await
    }

    pub async fn deploy(&self, req: &DeployRequest) -> Result<DeployResponse, DockyardError> {
        self.request(Method::POST, "/api/projects", Some(req)).await
    }

    pub async fn rebuild(&self, slug: &str) -> Result<SuccessResponse, DockyardError> {
        self.request(
            Method::POST,
            &format!("/api/projects/{}/rebuild", slug),
            None::<&()>,
        )
        .await
    }

    pub async fn start_project(&self, slug: &str) -> Result<SuccessResponse, DockyardError> {
        self.request(
            Method::POST,
            &format!("/api/projects/{}/start", slug),
            None::<&()>,
        )
        .await
    }

    pub async fn stop_project(&self, slug: &str) -> Result<SuccessResponse, DockyardError> {
        self.request(
            Method::POST,
            &format!("/api/projects/{}/stop", slug),
            None::<&()>,
        )
        .await
    }

    pub async fn delete_project(&self, slug: &str) -> Result<SuccessResponse, DockyardError> {
        self.request(
            Method::DELETE,
            &format!("/api/projects/{}", slug),
            None::<&()>,
        )
        .await
    }

    pub async fn get_logs(&self, slug: &str, tail: u32) -> Result<LogsResponse, DockyardError> {
        self.request(
            Method::GET,
            &format!("/api/projects/{}/logs?tail={}", slug, tail),
            None::<&()>,
        )
        .await
    }

    pub async fn get_config(&self) -> Result<ConfigResponse, DockyardError> {
        self.request(Method::GET, "/api/config", None::<&()>).await
    }

    pub async fn update_config(
        &self,
        req: &ConfigUpdateRequest,
    ) -> Result<SuccessResponse, DockyardError> {
        self.request(Method::PUT, "/api/config", Some(req)).await
    }
}
