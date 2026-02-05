use console::style;
use dialoguer::{Input, Select};
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::time::Duration;

use crate::cli::display;
use crate::config::project::NetworkMode;
use crate::ipc::protocol::DeployRequest;
use crate::ipc::IpcClient;

pub async fn deploy_interactive() -> anyhow::Result<()> {
    println!();
    println!("  {}", style("Deploy novo projeto").bold().cyan());
    println!();

    let repo_url: String = Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("URL do repositorio Git")
        .interact_text()?;

    let branch: String = Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("Branch para acompanhar")
        .default("main".to_string())
        .interact_text()?;

    let network_options = vec!["Rede local apenas", "Publico (via Cloudflare Tunnel)"];
    let network_selection =
        Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt("Modo de rede")
            .items(&network_options)
            .default(0)
            .interact()?;

    let network_mode = match network_selection {
        0 => NetworkMode::LocalOnly,
        1 => NetworkMode::Public,
        _ => unreachable!(),
    };

    let hostname: String = Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("Hostname personalizado (deixe vazio para auto)")
        .allow_empty(true)
        .interact_text()?;

    let hostname = if hostname.is_empty() {
        None
    } else {
        Some(hostname)
    };

    let container_port: u16 = Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("Porta do container (porta que seu app escuta)")
        .default(3000u16)
        .interact_text()?;

    println!();

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.enable_steady_tick(Duration::from_millis(100));
    spinner.set_message("Conectando ao daemon...");

    let client = IpcClient::new();

    let req = DeployRequest {
        repo_url,
        branch,
        network_mode,
        hostname,
        container_port,
        env_vars: HashMap::new(),
    };

    spinner.set_message("Enviando deploy...");

    match client.deploy(&req).await {
        Ok(resp) => {
            spinner.finish_and_clear();
            println!();
            display::print_success("Projeto implantado com sucesso!");
            println!();
            println!("  {} {}", style("Nome:").bold(), resp.name);
            println!("  {} {}", style("Slug:").bold(), resp.slug);
            println!(
                "  {} {}",
                style("URL:").bold(),
                resp.url.unwrap_or_else(|| "--".to_string())
            );
            println!("  {} {}", style("Porta host:").bold(), resp.host_port);
            println!();
            println!(
                "  {} {}",
                style("Webhook URL:").bold().dim(),
                resp.webhook_url
            );
            println!(
                "  {}",
                style("(Configure no GitHub para auto-deploy)").dim()
            );
            println!();
        }
        Err(e) => {
            spinner.finish_and_clear();
            display::print_error(&format!("Deploy falhou: {}", e));
        }
    }

    Ok(())
}

pub async fn deploy_direct(
    repo_url: String,
    branch: String,
    public: bool,
    domain: Option<String>,
    port: Option<u16>,
) -> anyhow::Result<()> {
    let client = IpcClient::new();

    let req = DeployRequest {
        repo_url,
        branch,
        network_mode: if public {
            NetworkMode::Public
        } else {
            NetworkMode::LocalOnly
        },
        hostname: domain,
        container_port: port.unwrap_or(3000),
        env_vars: HashMap::new(),
    };

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap(),
    );
    spinner.enable_steady_tick(Duration::from_millis(100));
    spinner.set_message("Deploying...");

    match client.deploy(&req).await {
        Ok(resp) => {
            spinner.finish_and_clear();
            display::print_success(&format!(
                "Deployed '{}' on port {}",
                resp.name, resp.host_port
            ));
            if let Some(url) = resp.url {
                println!("  URL: {}", url);
            }
            println!("  Webhook: {}", resp.webhook_url);
        }
        Err(e) => {
            spinner.finish_and_clear();
            display::print_error(&format!("Deploy failed: {}", e));
        }
    }

    Ok(())
}
