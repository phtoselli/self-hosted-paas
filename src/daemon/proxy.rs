use reqwest::Client;
use serde_json::json;

use crate::error::DockyardError;

pub struct CaddyProxy {
    client: Client,
    admin_api: String,
}

impl CaddyProxy {
    pub fn new(admin_api: &str) -> Self {
        Self {
            client: Client::new(),
            admin_api: admin_api.to_string(),
        }
    }

    /// Add a reverse proxy route for a project
    pub async fn add_route(
        &self,
        slug: &str,
        hostname: &str,
        upstream_port: u16,
    ) -> Result<(), DockyardError> {
        let route = json!({
            "@id": format!("dockyard-{}", slug),
            "match": [{"host": [hostname]}],
            "handle": [{
                "handler": "reverse_proxy",
                "upstreams": [{"dial": format!("localhost:{}", upstream_port)}]
            }]
        });

        let url = format!(
            "{}/config/apps/http/servers/dockyard/routes",
            self.admin_api
        );

        let resp = self.client.post(&url).json(&route).send().await;

        match resp {
            Ok(r) if r.status().is_success() => {
                tracing::info!(
                    "Added Caddy route: {} -> localhost:{}",
                    hostname,
                    upstream_port
                );
                Ok(())
            }
            Ok(r) => {
                let status = r.status();
                let body = r.text().await.unwrap_or_default();
                tracing::warn!("Caddy API responded with {}: {}", status, body);
                Err(DockyardError::Proxy(format!(
                    "Caddy API error {}: {}",
                    status, body
                )))
            }
            Err(e) => {
                tracing::warn!("Could not reach Caddy API: {}", e);
                Err(DockyardError::Proxy(format!("Caddy not available: {}", e)))
            }
        }
    }

    /// Update the upstream port for an existing route
    pub async fn update_route(
        &self,
        slug: &str,
        new_upstream_port: u16,
    ) -> Result<(), DockyardError> {
        let route_id = format!("dockyard-{}", slug);
        let url = format!("{}/id/{}", self.admin_api, route_id);

        let route = json!({
            "@id": route_id,
            "handle": [{
                "handler": "reverse_proxy",
                "upstreams": [{"dial": format!("localhost:{}", new_upstream_port)}]
            }]
        });

        let resp = self.client.put(&url).json(&route).send().await;

        match resp {
            Ok(r) if r.status().is_success() => {
                tracing::info!(
                    "Updated Caddy route '{}' -> localhost:{}",
                    slug,
                    new_upstream_port
                );
                Ok(())
            }
            Ok(r) => {
                let body = r.text().await.unwrap_or_default();
                Err(DockyardError::Proxy(format!(
                    "Caddy update error: {}",
                    body
                )))
            }
            Err(e) => Err(DockyardError::Proxy(format!("Caddy not available: {}", e))),
        }
    }

    /// Remove a route for a project
    pub async fn remove_route(&self, slug: &str) -> Result<(), DockyardError> {
        let route_id = format!("dockyard-{}", slug);
        let url = format!("{}/id/{}", self.admin_api, route_id);

        let resp = self.client.delete(&url).send().await;

        match resp {
            Ok(r) if r.status().is_success() => {
                tracing::info!("Removed Caddy route for '{}'", slug);
                Ok(())
            }
            Ok(_) | Err(_) => {
                tracing::warn!(
                    "Could not remove Caddy route for '{}' (may not exist)",
                    slug
                );
                Ok(())
            }
        }
    }
}
