use std::net::TcpListener;
use std::path::Path;
use tokio::process::Command;

use crate::error::DockyardError;

/// Generate a URL-safe slug from a project name
pub fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

/// Extract project name from a git repository URL
pub fn repo_name(url: &str) -> String {
    url.trim_end_matches('/')
        .trim_end_matches(".git")
        .rsplit('/')
        .next()
        .unwrap_or("project")
        .to_string()
}

/// Find an available port in the ephemeral range
pub fn find_available_port() -> Result<u16, DockyardError> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| DockyardError::Config(format!("No available ports: {}", e)))?;
    let port = listener.local_addr().unwrap().port();
    Ok(port)
}

/// Clone a git repository to a destination path
pub async fn git_clone(repo_url: &str, dest: &Path, branch: &str) -> Result<(), DockyardError> {
    let output = Command::new("git")
        .args([
            "clone",
            "--branch",
            branch,
            "--single-branch",
            "--depth",
            "1",
            repo_url,
        ])
        .arg(dest)
        .output()
        .await
        .map_err(|e| DockyardError::Git(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(DockyardError::Git(format!("git clone failed: {}", stderr)));
    }

    Ok(())
}

/// Pull latest changes in a git repository
pub async fn git_pull(repo_path: &Path, branch: &str) -> Result<String, DockyardError> {
    let output = Command::new("git")
        .args(["pull", "origin", branch])
        .current_dir(repo_path)
        .output()
        .await
        .map_err(|e| DockyardError::Git(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(DockyardError::Git(format!("git pull failed: {}", stderr)));
    }

    let sha_output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .await
        .map_err(|e| DockyardError::Git(e.to_string()))?;

    Ok(String::from_utf8_lossy(&sha_output.stdout)
        .trim()
        .to_string())
}

/// Generate a random webhook secret
pub fn generate_webhook_secret() -> String {
    uuid::Uuid::new_v4().to_string()
}
