use console::style;
use dialoguer::{Input, Select};

use crate::cli::commands::ConfigAction;
use crate::cli::display;
use crate::ipc::protocol::ConfigUpdateRequest;
use crate::ipc::IpcClient;

pub async fn settings_menu() -> anyhow::Result<()> {
    let client = IpcClient::new();

    loop {
        let options = vec![
            "Chave SSH do GitHub",
            "Token API GitHub",
            "Cloudflare Tunnel",
            "Ver configuracao atual",
            "Voltar",
        ];

        let selection = Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt("Configuracoes")
            .items(&options)
            .default(0)
            .interact()?;

        match selection {
            0 => {
                let path: String =
                    Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
                        .with_prompt("Caminho da chave SSH")
                        .default("~/.ssh/id_ed25519".to_string())
                        .interact_text()?;

                let req = ConfigUpdateRequest {
                    github_ssh_key_path: Some(path),
                    github_api_token: None,
                    cloudflare_tunnel_token: None,
                    cloudflare_enabled: None,
                };

                match client.update_config(&req).await {
                    Ok(_) => display::print_success("Chave SSH atualizada"),
                    Err(e) => display::print_error(&format!("{}", e)),
                }
            }
            1 => {
                let token: String =
                    Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
                        .with_prompt("Token API do GitHub (para registro de webhooks)")
                        .interact_text()?;

                let req = ConfigUpdateRequest {
                    github_ssh_key_path: None,
                    github_api_token: Some(token),
                    cloudflare_tunnel_token: None,
                    cloudflare_enabled: None,
                };

                match client.update_config(&req).await {
                    Ok(_) => display::print_success("Token API atualizado"),
                    Err(e) => display::print_error(&format!("{}", e)),
                }
            }
            2 => {
                let token: String =
                    Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
                        .with_prompt("Cloudflare Tunnel token")
                        .allow_empty(true)
                        .interact_text()?;

                let req = ConfigUpdateRequest {
                    github_ssh_key_path: None,
                    github_api_token: None,
                    cloudflare_tunnel_token: if token.is_empty() {
                        None
                    } else {
                        Some(token.clone())
                    },
                    cloudflare_enabled: Some(!token.is_empty()),
                };

                match client.update_config(&req).await {
                    Ok(_) => display::print_success("Cloudflare Tunnel configurado"),
                    Err(e) => display::print_error(&format!("{}", e)),
                }
            }
            3 => {
                show_config(&client).await?;
            }
            4 => return Ok(()),
            _ => unreachable!(),
        }

        println!();
    }
}

async fn show_config(client: &IpcClient) -> anyhow::Result<()> {
    match client.get_config().await {
        Ok(config) => {
            println!();
            println!("  {}", style("Configuracao atual").bold().cyan());
            println!("  {}", "-".repeat(40));
            println!(
                "  {} {}",
                style("GitHub SSH Key:").bold(),
                config
                    .github_ssh_key_path
                    .unwrap_or_else(|| "(nao configurado)".to_string())
            );
            println!(
                "  {} {}",
                style("GitHub API Token:").bold(),
                if config.github_api_token_set {
                    "configurado"
                } else {
                    "(nao configurado)"
                }
            );
            println!(
                "  {} {}",
                style("Cloudflare:").bold(),
                if config.cloudflare_enabled {
                    "ativo"
                } else {
                    "desativado"
                }
            );
            if let Some(tunnel_id) = &config.cloudflare_tunnel_id {
                println!("  {} {}", style("Tunnel ID:").bold(), tunnel_id);
            }
            println!(
                "  {} {}",
                style("Webhook Port:").bold(),
                config.webhook_port
            );
            println!(
                "  {} {}",
                style("Socket:").bold(),
                config.socket_path
            );
            println!();
        }
        Err(e) => display::print_error(&format!("{}", e)),
    }
    Ok(())
}

pub async fn handle_config_action(action: ConfigAction) -> anyhow::Result<()> {
    let client = IpcClient::new();

    match action {
        ConfigAction::Show => {
            show_config(&client).await?;
        }
        ConfigAction::Set { key, value } => {
            let mut req = ConfigUpdateRequest {
                github_ssh_key_path: None,
                github_api_token: None,
                cloudflare_tunnel_token: None,
                cloudflare_enabled: None,
            };

            match key.as_str() {
                "github.ssh_key_path" => req.github_ssh_key_path = Some(value),
                "github.api_token" => req.github_api_token = Some(value),
                "cloudflare.tunnel_token" => req.cloudflare_tunnel_token = Some(value),
                "cloudflare.enabled" => {
                    req.cloudflare_enabled = Some(value.parse().unwrap_or(false))
                }
                _ => {
                    display::print_error(&format!("Chave desconhecida: {}", key));
                    println!("  Chaves validas: github.ssh_key_path, github.api_token, cloudflare.tunnel_token, cloudflare.enabled");
                    return Ok(());
                }
            }

            match client.update_config(&req).await {
                Ok(_) => display::print_success(&format!("'{}' atualizado", key)),
                Err(e) => display::print_error(&format!("{}", e)),
            }
        }
    }

    Ok(())
}
