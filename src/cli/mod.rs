pub mod commands;
pub mod deploy;
pub mod display;
pub mod projects;
pub mod settings;

use crate::cli::commands::Commands;

/// Handle a specific CLI subcommand
pub async fn handle_command(cmd: Commands) -> anyhow::Result<()> {
    match cmd {
        Commands::Daemon => unreachable!(),
        Commands::Deploy {
            repo,
            branch,
            public,
            domain,
            port,
        } => {
            if let Some(repo_url) = repo {
                deploy::deploy_direct(repo_url, branch, public, domain, port).await?;
            } else {
                deploy::deploy_interactive().await?;
            }
        }
        Commands::List => {
            projects::list_projects().await?;
        }
        Commands::Status { slug } => {
            projects::show_status(&slug).await?;
        }
        Commands::Rebuild { slug } => {
            projects::rebuild_project(&slug).await?;
        }
        Commands::Logs { slug, follow, tail } => {
            projects::show_logs(&slug, follow, tail).await?;
        }
        Commands::Stop { slug } => {
            projects::stop_project(&slug).await?;
        }
        Commands::Start { slug } => {
            projects::start_project(&slug).await?;
        }
        Commands::Delete { slug } => {
            projects::delete_project(&slug).await?;
        }
        Commands::Config { action } => {
            settings::handle_config_action(action).await?;
        }
    }
    Ok(())
}

/// Show the interactive main menu
pub async fn interactive_menu() -> anyhow::Result<()> {
    display::print_banner();

    loop {
        let options = vec![
            "Deploy novo projeto",
            "Gerenciar projetos",
            "Configuracoes",
            "Sair",
        ];

        let selection = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
            .with_prompt("O que voce quer fazer?")
            .items(&options)
            .default(0)
            .interact()?;

        match selection {
            0 => deploy::deploy_interactive().await?,
            1 => projects::projects_menu().await?,
            2 => settings::settings_menu().await?,
            3 => {
                println!("Ate mais!");
                break;
            }
            _ => unreachable!(),
        }

        println!();
    }

    Ok(())
}
