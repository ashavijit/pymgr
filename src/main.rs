#![allow(dead_code)]

mod cache;
mod cli;
mod commands;
mod config;
mod doctor;
mod env;
mod errors;
mod installer;
mod lockfile;
mod output;
mod python;
mod resolver;
mod self_update;
mod shell;

use clap::Parser;
use cli::{Cli, Commands, EnvCommands, PythonCommands};
use errors::PymgrResult;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    output::set_verbose_mode(cli.verbose);
    output::set_json_mode(cli.json);
    output::set_no_color(cli.no_color);

    if cli.no_color {
        console::set_colors_enabled(false);
    }

    let result = run(cli).await;

    if let Err(e) = result {
        if output::is_json_mode() {
            output::print_json(&e.to_json());
        } else {
            output::print_error(&e.format_human());
        }
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> PymgrResult<()> {
    let project_dir = config::PymgrConfig::find_project_root()?;

    match cli.command {
        Commands::Init { python } => {
            commands::init::exec(&project_dir, python.as_deref())?;
        }

        Commands::Create { name, python } => {
            config::ensure_pymgr_dirs()?;
            crate::env::manager::create_named_env(&name, python.as_deref())?;
        }

        Commands::Activate => {
            commands::shell::exec_activate(&project_dir)?;
        }

        Commands::Deactivate => {
            commands::shell::exec_deactivate()?;
        }

        Commands::Run { cmd, args } => {
            commands::run::exec(&project_dir, &cmd, &args)?;
        }

        Commands::Shell => {
            commands::shell::exec_shell(&project_dir)?;
        }

        Commands::Python(subcmd) => match subcmd {
            PythonCommands::List => {
                commands::python::exec_list()?;
            }
            PythonCommands::Install { version } => {
                commands::python::exec_install(&version).await?;
            }
            PythonCommands::Use { version } => {
                commands::python::exec_use(&project_dir, &version)?;
            }
            PythonCommands::Remove { version } => {
                commands::python::exec_remove(&version)?;
            }
        },

        Commands::Add { packages, dev } => {
            commands::packages::exec_add(&project_dir, &packages, dev).await?;
        }

        Commands::Remove { packages } => {
            commands::packages::exec_remove(&project_dir, &packages).await?;
        }

        Commands::Install { frozen } => {
            commands::packages::exec_install(&project_dir, frozen).await?;
        }

        Commands::Update { packages } => {
            commands::packages::exec_update(&project_dir, &packages).await?;
        }

        Commands::Sync => {
            commands::packages::exec_sync(&project_dir).await?;
        }

        Commands::List => {
            commands::packages::exec_list(&project_dir)?;
        }

        Commands::Env(subcmd) => match subcmd {
            EnvCommands::List => {
                commands::env::exec_list()?;
            }
            EnvCommands::Remove { name } => {
                commands::env::exec_remove(&name)?;
            }
            EnvCommands::Info => {
                commands::env::exec_info(&project_dir)?;
            }
        },

        Commands::ShellInit { shell } => {
            commands::shell::exec_shell_init(&shell)?;
        }

        Commands::SelfUpdate => {
            commands::self_update::exec().await?;
        }

        Commands::Doctor => {
            commands::doctor::exec(&project_dir)?;
        }
    }

    Ok(())
}
