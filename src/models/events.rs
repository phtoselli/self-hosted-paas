use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubPushEvent {
    #[serde(rename = "ref")]
    pub git_ref: String,
    pub after: String,
    pub repository: GitHubRepository,
    pub pusher: GitHubPusher,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubRepository {
    pub full_name: String,
    pub clone_url: String,
    pub ssh_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitHubPusher {
    pub name: String,
}

impl GitHubPushEvent {
    pub fn branch(&self) -> Option<&str> {
        self.git_ref.strip_prefix("refs/heads/")
    }
}
