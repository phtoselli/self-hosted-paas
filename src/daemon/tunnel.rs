use std::sync::Arc;
use tokio::process::{Child, Command};
use tokio::sync::RwLock;

use crate::error::DockyardError;

pub struct TunnelManager {
    process: Arc<RwLock<Option<Child>>>,
    tunnel_url: Arc<RwLock<Option<String>>>,
}

impl TunnelManager {
    pub fn new() -> Self {
        Self {
            process: Arc::new(RwLock::new(None)),
            tunnel_url: Arc::new(RwLock::new(None)),
        }
    }

    /// Start a Cloudflare quick tunnel pointing to a local port
    pub async fn start_quick_tunnel(&self, local_port: u16) -> Result<String, DockyardError> {
        let child = Command::new("cloudflared")
            .args([
                "tunnel",
                "--url",
                &format!("http://localhost:{}", local_port),
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                DockyardError::Tunnel(format!(
                    "Failed to start cloudflared: {}. Is it installed?",
                    e
                ))
            })?;

        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        let tunnel_url = "https://tunnel-pending.trycloudflare.com".to_string();

        {
            let mut process = self.process.write().await;
            *process = Some(child);
        }
        {
            let mut url = self.tunnel_url.write().await;
            *url = Some(tunnel_url.clone());
        }

        tracing::info!("Cloudflare tunnel started");
        Ok(tunnel_url)
    }

    /// Start a named tunnel using a token
    pub async fn start_named_tunnel(&self, token: &str) -> Result<(), DockyardError> {
        let child = Command::new("cloudflared")
            .args(["tunnel", "run", "--token", token])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                DockyardError::Tunnel(format!("Failed to start cloudflared: {}", e))
            })?;

        {
            let mut process = self.process.write().await;
            *process = Some(child);
        }

        tracing::info!("Cloudflare named tunnel started");
        Ok(())
    }

    /// Stop the tunnel
    pub async fn stop(&self) -> Result<(), DockyardError> {
        let mut process = self.process.write().await;
        if let Some(ref mut child) = *process {
            child.kill().await.map_err(|e| {
                DockyardError::Tunnel(format!("Failed to stop cloudflared: {}", e))
            })?;
            tracing::info!("Cloudflare tunnel stopped");
        }
        *process = None;

        let mut url = self.tunnel_url.write().await;
        *url = None;

        Ok(())
    }

    /// Check if the tunnel is running
    pub async fn is_running(&self) -> bool {
        let process = self.process.read().await;
        process.is_some()
    }

    /// Get the current tunnel URL
    pub async fn get_url(&self) -> Option<String> {
        self.tunnel_url.read().await.clone()
    }
}
