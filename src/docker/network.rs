use bollard::network::CreateNetworkOptions;

use crate::docker::DockerClient;
use crate::error::DockyardError;

pub const DOCKYARD_NETWORK: &str = "dockyard-network";

/// Ensure the dockyard Docker network exists
pub async fn ensure_network(docker: &DockerClient) -> Result<(), DockyardError> {
    let networks = docker.inner().list_networks::<String>(None).await?;

    let exists = networks
        .iter()
        .any(|n| n.name.as_deref() == Some(DOCKYARD_NETWORK));

    if !exists {
        let config = CreateNetworkOptions {
            name: DOCKYARD_NETWORK,
            driver: "bridge",
            ..Default::default()
        };
        docker.inner().create_network(config).await?;
        tracing::info!("Created Docker network: {}", DOCKYARD_NETWORK);
    }

    Ok(())
}
