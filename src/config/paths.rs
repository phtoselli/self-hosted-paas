use std::path::PathBuf;

pub fn config_dir() -> PathBuf {
    PathBuf::from("/etc/dockyard")
}

pub fn global_config_path() -> PathBuf {
    config_dir().join("config.toml")
}

pub fn data_dir() -> PathBuf {
    PathBuf::from("/var/lib/dockyard")
}

pub fn projects_dir() -> PathBuf {
    data_dir().join("projects")
}

pub fn project_dir(slug: &str) -> PathBuf {
    projects_dir().join(slug)
}

pub fn project_config_path(slug: &str) -> PathBuf {
    project_dir(slug).join("project.toml")
}

pub fn project_repo_dir(slug: &str) -> PathBuf {
    project_dir(slug).join("repo")
}

pub fn project_logs_dir(slug: &str) -> PathBuf {
    project_dir(slug).join("logs")
}

pub fn socket_path() -> PathBuf {
    PathBuf::from("/var/run/dockyard.sock")
}

pub fn pid_file_path() -> PathBuf {
    PathBuf::from("/var/run/dockyard.pid")
}
