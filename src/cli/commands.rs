use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "dockyard",
    about = "Self-hosted PaaS - Deploy and manage Docker containers",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Clone)]
pub enum Commands {
    /// Start the daemon (systemd service mode)
    Daemon,

    /// Deploy a new project
    Deploy {
        /// Git repository URL
        #[arg(long)]
        repo: Option<String>,

        /// Branch to track
        #[arg(long, default_value = "main")]
        branch: String,

        /// Expose publicly via Cloudflare Tunnel
        #[arg(long)]
        public: bool,

        /// Custom hostname/domain
        #[arg(long)]
        domain: Option<String>,

        /// Container port (the port your app listens on)
        #[arg(long)]
        port: Option<u16>,
    },

    /// List all projects
    List,

    /// Show project status
    Status {
        /// Project slug
        slug: String,
    },

    /// Rebuild a project
    Rebuild {
        /// Project slug
        slug: String,
    },

    /// View project logs
    Logs {
        /// Project slug
        slug: String,

        /// Follow log output
        #[arg(short, long)]
        follow: bool,

        /// Number of lines to show
        #[arg(short = 'n', long, default_value = "100")]
        tail: u32,
    },

    /// Stop (disable) a project
    Stop {
        /// Project slug
        slug: String,
    },

    /// Start (enable) a project
    Start {
        /// Project slug
        slug: String,
    },

    /// Delete a project
    Delete {
        /// Project slug
        slug: String,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand, Clone)]
pub enum ConfigAction {
    /// Show current configuration
    Show,

    /// Set a configuration value
    Set {
        /// Config key (github.ssh_key_path, github.api_token, cloudflare.tunnel_token, cloudflare.enabled)
        key: String,

        /// Value to set
        value: String,
    },
}
