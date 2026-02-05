use thiserror::Error;

#[derive(Error, Debug)]
pub enum DockyardError {
    #[error("Docker error: {0}")]
    Docker(#[from] bollard::errors::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Project '{0}' not found")]
    ProjectNotFound(String),

    #[error("Project '{0}' already exists")]
    ProjectAlreadyExists(String),

    #[error("IPC error: {0}")]
    Ipc(String),

    #[error("Daemon not running. Start with: sudo dockyard daemon")]
    DaemonNotRunning,

    #[error("Build failed: {0}")]
    BuildFailed(String),

    #[error("Git error: {0}")]
    Git(String),

    #[error("Proxy error: {0}")]
    Proxy(String),

    #[error("Tunnel error: {0}")]
    Tunnel(String),

    #[error("Webhook error: {0}")]
    Webhook(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Port {0} is already in use")]
    PortInUse(u16),
}

impl From<serde_json::Error> for DockyardError {
    fn from(e: serde_json::Error) -> Self {
        DockyardError::Serialization(e.to_string())
    }
}

impl From<toml::de::Error> for DockyardError {
    fn from(e: toml::de::Error) -> Self {
        DockyardError::Config(e.to_string())
    }
}

impl From<reqwest::Error> for DockyardError {
    fn from(e: reqwest::Error) -> Self {
        DockyardError::Http(e.to_string())
    }
}
