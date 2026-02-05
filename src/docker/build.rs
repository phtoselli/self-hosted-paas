use bollard::image::BuildImageOptions;
use futures_util::StreamExt;
use std::path::Path;

use crate::docker::DockerClient;
use crate::error::DockyardError;

/// Find the Dockerfile in a project directory
pub fn find_dockerfile(project_dir: &Path) -> Result<String, DockyardError> {
    let candidates = ["Dockerfile", "dockerfile", "Dockerfile.prod"];
    for candidate in &candidates {
        if project_dir.join(candidate).exists() {
            return Ok(candidate.to_string());
        }
    }
    Err(DockyardError::BuildFailed(
        "No Dockerfile found in repository".into(),
    ))
}

/// Build a Docker image from a project directory
pub async fn build_image(
    docker: &DockerClient,
    project_dir: &Path,
    image_name: &str,
    dockerfile: &str,
) -> Result<(), DockyardError> {
    let tar_bytes = create_build_context(project_dir)?;

    let build_options = BuildImageOptions {
        t: image_name,
        dockerfile,
        rm: true,
        forcerm: true,
        ..Default::default()
    };

    let mut stream = docker
        .inner()
        .build_image(build_options, None, Some(tar_bytes.into()));

    while let Some(result) = stream.next().await {
        match result {
            Ok(output) => {
                if let Some(stream_msg) = output.stream {
                    let msg = stream_msg.trim();
                    if !msg.is_empty() {
                        tracing::info!("[build] {}", msg);
                    }
                }
                if let Some(error) = output.error {
                    return Err(DockyardError::BuildFailed(error));
                }
            }
            Err(e) => {
                return Err(DockyardError::Docker(e));
            }
        }
    }

    Ok(())
}

/// Create a tar archive from a directory for Docker build context
fn create_build_context(project_dir: &Path) -> Result<Vec<u8>, DockyardError> {
    let mut ar = tar::Builder::new(Vec::new());
    ar.append_dir_all(".", project_dir)
        .map_err(|e| DockyardError::BuildFailed(format!("Failed to create build context: {}", e)))?;
    ar.finish()
        .map_err(|e| DockyardError::BuildFailed(format!("Failed to finalize tar: {}", e)))?;
    let bytes = ar
        .into_inner()
        .map_err(|e| DockyardError::BuildFailed(format!("Failed to get tar bytes: {}", e)))?;
    Ok(bytes)
}
