use console::style;
use dialoguer::{Confirm, Select};

use crate::cli::display;
use crate::ipc::IpcClient;

pub async fn projects_menu() -> anyhow::Result<()> {
    let client = IpcClient::new();

    loop {
        let projects = match client.list_projects().await {
            Ok(p) => p,
            Err(e) => {
                display::print_error(&format!("Erro ao listar projetos: {}", e));
                return Ok(());
            }
        };

        if projects.is_empty() {
            println!();
            println!("  {}", style("Nenhum projeto encontrado.").dim());
            println!();
            return Ok(());
        }

        println!();
        display::print_project_table(&projects);
        println!();

        let mut options: Vec<String> = projects
            .iter()
            .map(|p| {
                format!(
                    "{} [{}]",
                    p.name,
                    display::format_state(&p.state)
                )
            })
            .collect();
        options.push("Voltar".to_string());

        let selection = Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt("Selecione um projeto")
            .items(&options)
            .default(0)
            .interact()?;

        if selection == options.len() - 1 {
            return Ok(());
        }

        let slug = &projects[selection].slug;
        project_actions_menu(&client, slug).await?;
    }
}

async fn project_actions_menu(client: &IpcClient, slug: &str) -> anyhow::Result<()> {
    loop {
        let actions = vec![
            "Ver detalhes",
            "Ver logs",
            "Rebuildar",
            "Iniciar",
            "Parar",
            "Deletar",
            "Voltar",
        ];

        let selection = Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt(format!("Acoes para '{}'", slug))
            .items(&actions)
            .default(0)
            .interact()?;

        match selection {
            0 => {
                match client.get_project(slug).await {
                    Ok(detail) => {
                        display::print_project_detail(
                            &detail.status,
                            &detail.repo_url,
                            &detail.branch,
                        );
                    }
                    Err(e) => display::print_error(&format!("{}", e)),
                }
            }
            1 => {
                show_logs(slug, false, 50).await?;
            }
            2 => {
                rebuild_project(slug).await?;
            }
            3 => {
                start_project(slug).await?;
            }
            4 => {
                stop_project(slug).await?;
            }
            5 => {
                delete_project(slug).await?;
                return Ok(());
            }
            6 => return Ok(()),
            _ => unreachable!(),
        }
    }
}

pub async fn list_projects() -> anyhow::Result<()> {
    let client = IpcClient::new();
    match client.list_projects().await {
        Ok(projects) => {
            println!();
            display::print_project_table(&projects);
            println!();
        }
        Err(e) => display::print_error(&format!("{}", e)),
    }
    Ok(())
}

pub async fn show_status(slug: &str) -> anyhow::Result<()> {
    let client = IpcClient::new();
    match client.get_project(slug).await {
        Ok(detail) => {
            display::print_project_detail(&detail.status, &detail.repo_url, &detail.branch);
        }
        Err(e) => display::print_error(&format!("{}", e)),
    }
    Ok(())
}

pub async fn rebuild_project(slug: &str) -> anyhow::Result<()> {
    let client = IpcClient::new();
    match client.rebuild(slug).await {
        Ok(resp) => display::print_success(&resp.message),
        Err(e) => display::print_error(&format!("{}", e)),
    }
    Ok(())
}

pub async fn start_project(slug: &str) -> anyhow::Result<()> {
    let client = IpcClient::new();
    match client.start_project(slug).await {
        Ok(resp) => display::print_success(&resp.message),
        Err(e) => display::print_error(&format!("{}", e)),
    }
    Ok(())
}

pub async fn stop_project(slug: &str) -> anyhow::Result<()> {
    let client = IpcClient::new();
    match client.stop_project(slug).await {
        Ok(resp) => display::print_success(&resp.message),
        Err(e) => display::print_error(&format!("{}", e)),
    }
    Ok(())
}

pub async fn delete_project(slug: &str) -> anyhow::Result<()> {
    let confirm = Confirm::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt(format!(
            "Tem certeza que deseja deletar '{}'? Esta acao e irreversivel.",
            slug
        ))
        .default(false)
        .interact()?;

    if !confirm {
        println!("  Cancelado.");
        return Ok(());
    }

    let client = IpcClient::new();
    match client.delete_project(slug).await {
        Ok(resp) => display::print_success(&resp.message),
        Err(e) => display::print_error(&format!("{}", e)),
    }
    Ok(())
}

pub async fn show_logs(slug: &str, _follow: bool, tail: u32) -> anyhow::Result<()> {
    let client = IpcClient::new();
    match client.get_logs(slug, tail).await {
        Ok(resp) => {
            println!();
            println!(
                "  {} {} (ultimas {} linhas)",
                style("Logs de").dim(),
                style(slug).bold(),
                tail,
            );
            println!("  {}", "-".repeat(60));
            for line in &resp.logs {
                println!("  {}", line);
            }
            println!();
        }
        Err(e) => display::print_error(&format!("{}", e)),
    }
    Ok(())
}
