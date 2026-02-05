use console::style;

use crate::models::project::{ProjectState, ProjectStatus};

pub fn print_banner() {
    println!(
        r#"
  {}
  {}
  {}
  {}
                     {}
"#,
        style("  ___           _                       _").cyan(),
        style(" |   \\ ___  __ | |__ _  _ __ _ _ _ __| |").cyan(),
        style(" | |) / _ \\/ _|| / /| || / _` | '_/ _` |").cyan(),
        style(" |___/\\___/\\__||_\\_\\ \\_, \\__,_|_| \\__,_|").cyan(),
        style("Self-Hosted PaaS").dim(),
    );
}

pub fn print_project_table(projects: &[ProjectStatus]) {
    if projects.is_empty() {
        println!("  {}", style("Nenhum projeto encontrado.").dim());
        return;
    }

    println!(
        "  {:<20} {:<12} {:<35} {:<12} {:<10}",
        style("NOME").bold(),
        style("STATUS").bold(),
        style("URL").bold(),
        style("UPTIME").bold(),
        style("MEMORIA").bold(),
    );
    println!("  {}", "-".repeat(89));

    for project in projects {
        let status_display = format_state(&project.state);
        let url = project.url.as_deref().unwrap_or("--");
        let uptime = format_uptime(project.uptime_secs);
        let memory = project
            .memory_usage_mb
            .map(|m| format!("{:.0} MB", m))
            .unwrap_or_else(|| "--".to_string());

        println!(
            "  {:<20} {:<12} {:<35} {:<12} {:<10}",
            project.name, status_display, url, uptime, memory,
        );
    }
}

pub fn format_state(state: &ProjectState) -> String {
    match state {
        ProjectState::Online => style("Online").green().to_string(),
        ProjectState::Building => style("Building").yellow().to_string(),
        ProjectState::Rebuilding => style("Rebuilding").yellow().to_string(),
        ProjectState::Starting => style("Starting").yellow().to_string(),
        ProjectState::Offline => style("Offline").dim().to_string(),
        ProjectState::Stopped => style("Stopped").red().to_string(),
        ProjectState::Error => style("Error").red().bold().to_string(),
    }
}

pub fn format_uptime(secs: Option<u64>) -> String {
    match secs {
        None => "--".to_string(),
        Some(s) => {
            let days = s / 86400;
            let hours = (s % 86400) / 3600;
            let mins = (s % 3600) / 60;

            if days > 0 {
                format!("{}d {}h", days, hours)
            } else if hours > 0 {
                format!("{}h {}m", hours, mins)
            } else {
                format!("{}m", mins)
            }
        }
    }
}

pub fn print_project_detail(status: &ProjectStatus, repo_url: &str, branch: &str) {
    println!();
    println!("  {} {}", style("Projeto:").bold(), status.name);
    println!("  {} {}", style("Slug:").bold(), status.slug);
    println!(
        "  {} {}",
        style("Status:").bold(),
        format_state(&status.state)
    );
    println!("  {} {}", style("Repositorio:").bold(), repo_url);
    println!("  {} {}", style("Branch:").bold(), branch);
    println!("  {} {}", style("Rede:").bold(), status.network_mode);
    println!(
        "  {} {}",
        style("URL:").bold(),
        status.url.as_deref().unwrap_or("--")
    );
    println!(
        "  {} {} -> {} (host)",
        style("Porta:").bold(),
        status.container_port,
        status.host_port,
    );
    println!(
        "  {} {}",
        style("Uptime:").bold(),
        format_uptime(status.uptime_secs)
    );
    println!(
        "  {} {}",
        style("Memoria:").bold(),
        status
            .memory_usage_mb
            .map(|m| format!("{:.1} MB", m))
            .unwrap_or_else(|| "--".to_string())
    );
    println!(
        "  {} {}",
        style("CPU:").bold(),
        status
            .cpu_percent
            .map(|c| format!("{:.1}%", c))
            .unwrap_or_else(|| "--".to_string())
    );
    if let Some(deploy) = &status.last_deploy {
        println!(
            "  {} {}",
            style("Ultimo deploy:").bold(),
            deploy.format("%Y-%m-%d %H:%M:%S")
        );
    }
    println!();
}

pub fn print_success(msg: &str) {
    println!("  {} {}", style("OK").green().bold(), msg);
}

pub fn print_error(msg: &str) {
    println!("  {} {}", style("ERRO").red().bold(), msg);
}
