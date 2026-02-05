use bollard::container::{
    Config, CreateContainerOptions, ListContainersOptions, LogsOptions, RemoveContainerOptions,
    StartContainerOptions, StatsOptions, StopContainerOptions,
};
use bollard::image::RemoveImageOptions;
use bollard::models::{HostConfig, PortBinding, RestartPolicy, RestartPolicyNameEnum};
use bollard::Docker;
use futures_util::StreamExt;
use std::collections::HashMap;

use crate::error::DockyardError;
use crate::models::project::ProjectState;

pub struct DockerClient {
    docker: Docker,
}

impl DockerClient {
    pub fn connect() -> Result<Self, DockyardError> {
        let docker = Docker::connect_with_local_defaults()?;
        Ok(Self { docker })
    }

    pub async fn ping(&self) -> Result<(), DockyardError> {
        self.docker.ping().await?;
        Ok(())
    }

    pub fn inner(&self) -> &Docker {
        &self.docker
    }

    pub async fn create_and_start_container(
        &self,
        container_name: &str,
        image: &str,
        host_port: u16,
        container_port: u16,
        env_vars: &HashMap<String, String>,
    ) -> Result<String, DockyardError> {
        let env: Vec<String> = env_vars
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        let port_key = format!("{}/tcp", container_port);

        let mut port_bindings = HashMap::new();
        port_bindings.insert(
            port_key.clone(),
            Some(vec![PortBinding {
                host_ip: Some("0.0.0.0".to_string()),
                host_port: Some(host_port.to_string()),
            }]),
        );

        let mut exposed_ports = HashMap::new();
        exposed_ports.insert(port_key, HashMap::new());

        let config = Config {
            image: Some(image.to_string()),
            env: Some(env),
            exposed_ports: Some(exposed_ports),
            host_config: Some(HostConfig {
                port_bindings: Some(port_bindings),
                restart_policy: Some(RestartPolicy {
                    name: Some(RestartPolicyNameEnum::UNLESS_STOPPED),
                    maximum_retry_count: None,
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        let options = CreateContainerOptions {
            name: container_name,
            platform: None,
        };

        let response = self.docker.create_container(Some(options), config).await?;
        let container_id = response.id;

        self.docker
            .start_container(&container_id, None::<StartContainerOptions<String>>)
            .await?;

        Ok(container_id)
    }

    pub async fn stop_container(&self, container_name: &str) -> Result<(), DockyardError> {
        self.docker
            .stop_container(container_name, Some(StopContainerOptions { t: 10 }))
            .await?;
        Ok(())
    }

    pub async fn remove_container(&self, container_name: &str) -> Result<(), DockyardError> {
        self.docker
            .remove_container(
                container_name,
                Some(RemoveContainerOptions {
                    force: true,
                    v: true,
                    ..Default::default()
                }),
            )
            .await?;
        Ok(())
    }

    pub async fn remove_image(&self, image_name: &str) -> Result<(), DockyardError> {
        self.docker
            .remove_image(
                image_name,
                Some(RemoveImageOptions {
                    force: true,
                    ..Default::default()
                }),
                None,
            )
            .await?;
        Ok(())
    }

    pub async fn is_container_running(&self, container_name: &str) -> Result<bool, DockyardError> {
        let mut filters = HashMap::new();
        filters.insert("name", vec![container_name]);

        let containers = self
            .docker
            .list_containers(Some(ListContainersOptions {
                all: true,
                filters,
                ..Default::default()
            }))
            .await?;

        for container in &containers {
            if let Some(state) = &container.state {
                if state == "running" {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    pub async fn get_container_stats(
        &self,
        container_name: &str,
    ) -> Result<(f64, f64), DockyardError> {
        let mut stream = self.docker.stats(
            container_name,
            Some(StatsOptions {
                stream: false,
                one_shot: true,
            }),
        );

        if let Some(Ok(stats)) = stream.next().await {
            let memory_mb = stats.memory_stats.usage.unwrap_or(0) as f64 / 1_048_576.0;

            let cpu_percent = {
                let cpu_usage = stats.cpu_stats.cpu_usage.total_usage;
                let prev_cpu = stats.precpu_stats.cpu_usage.total_usage;
                let system_cpu = stats.cpu_stats.system_cpu_usage.unwrap_or(0);
                let prev_system = stats.precpu_stats.system_cpu_usage.unwrap_or(0);

                let cpu_delta = cpu_usage as f64 - prev_cpu as f64;
                let system_delta = system_cpu as f64 - prev_system as f64;
                let num_cpus = stats
                    .cpu_stats
                    .cpu_usage
                    .percpu_usage
                    .as_ref()
                    .map(|v| v.len())
                    .unwrap_or(1) as f64;

                if system_delta > 0.0 {
                    (cpu_delta / system_delta) * num_cpus * 100.0
                } else {
                    0.0
                }
            };

            Ok((memory_mb, cpu_percent))
        } else {
            Ok((0.0, 0.0))
        }
    }

    pub async fn get_logs(
        &self,
        container_name: &str,
        tail: u32,
        follow: bool,
    ) -> Result<Vec<String>, DockyardError> {
        let options = LogsOptions::<String> {
            stdout: true,
            stderr: true,
            tail: tail.to_string(),
            follow,
            ..Default::default()
        };

        let mut stream = self.docker.logs(container_name, Some(options));
        let mut logs = Vec::new();

        while let Some(Ok(log)) = stream.next().await {
            logs.push(log.to_string());
            if !follow && logs.len() >= tail as usize {
                break;
            }
        }

        Ok(logs)
    }

    pub async fn get_container_state(
        &self,
        container_name: &str,
    ) -> Result<ProjectState, DockyardError> {
        let mut filters = HashMap::new();
        filters.insert("name", vec![container_name]);

        let containers = self
            .docker
            .list_containers(Some(ListContainersOptions {
                all: true,
                filters,
                ..Default::default()
            }))
            .await?;

        if let Some(container) = containers.first() {
            match container.state.as_deref() {
                Some("running") => Ok(ProjectState::Online),
                Some("exited") => Ok(ProjectState::Stopped),
                Some("created") => Ok(ProjectState::Starting),
                Some("restarting") => Ok(ProjectState::Starting),
                _ => Ok(ProjectState::Offline),
            }
        } else {
            Ok(ProjectState::Offline)
        }
    }

    pub async fn get_container_uptime(
        &self,
        container_name: &str,
    ) -> Result<Option<u64>, DockyardError> {
        let inspect = self.docker.inspect_container(container_name, None).await?;

        if let Some(state) = inspect.state {
            if let Some(started_at) = state.started_at {
                if let Ok(start_time) = chrono::DateTime::parse_from_rfc3339(&started_at) {
                    let now = chrono::Utc::now();
                    let duration = now.signed_duration_since(start_time);
                    return Ok(Some(duration.num_seconds().max(0) as u64));
                }
            }
        }

        Ok(None)
    }
}
