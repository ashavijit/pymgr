use std::path::Path;
use std::process::Command;

use crate::config::PymgrConfig;
use crate::errors::PymgrResult;
use crate::output;

fn parse_script_tokens(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_double_quote = false;
    let mut in_single_quote = false;

    for ch in input.chars() {
        match ch {
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
            }
            ' ' if !in_double_quote && !in_single_quote => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

pub fn exec(project_dir: &Path, cmd: &str, args: &[String]) -> PymgrResult<()> {
    let config = PymgrConfig::load(project_dir).unwrap_or_default();

    if let Some(script) = config.scripts.get(cmd) {
        output::print_info(&format!("Running script '{}': {}", cmd, script));

        let mut tokens = parse_script_tokens(script);
        for a in args {
            tokens.push(a.clone());
        }

        if tokens.is_empty() {
            output::print_error("Script is empty");
            std::process::exit(1);
        }

        let program = tokens.remove(0);

        let python_path = crate::env::manager::get_env_python(project_dir)?;
        let env_dir = python_path.parent().unwrap().parent().unwrap();
        let bin_dir = if cfg!(windows) {
            env_dir.join("Scripts")
        } else {
            env_dir.join("bin")
        };
        let path_var = std::env::var("PATH").unwrap_or_default();
        let new_path = if cfg!(windows) {
            format!("{};{}", bin_dir.display(), path_var)
        } else {
            format!("{}:{}", bin_dir.display(), path_var)
        };

        let actual_program = if program == "python" {
            python_path.to_string_lossy().to_string()
        } else {
            let cmd_in_env = if cfg!(windows) {
                bin_dir.join(format!("{}.exe", program))
            } else {
                bin_dir.join(&program)
            };
            if cmd_in_env.exists() {
                cmd_in_env.to_string_lossy().to_string()
            } else {
                program
            }
        };

        let result = Command::new(&actual_program)
            .args(&tokens)
            .env("PATH", &new_path)
            .env("VIRTUAL_ENV", env_dir)
            .env("PYMGR_ACTIVE", "1")
            .status();

        let status = match result {
            Ok(s) => s,
            Err(_) => {
                let full_cmd = format!("{} {}", script, args.join(" ")).trim().to_string();
                if cfg!(windows) {
                    Command::new("cmd")
                        .arg("/C")
                        .arg(&full_cmd)
                        .env("PATH", &new_path)
                        .env("VIRTUAL_ENV", env_dir)
                        .env("PYMGR_ACTIVE", "1")
                        .status()?
                } else {
                    Command::new("sh")
                        .arg("-c")
                        .arg(&full_cmd)
                        .env("PATH", &new_path)
                        .env("VIRTUAL_ENV", env_dir)
                        .env("PYMGR_ACTIVE", "1")
                        .status()?
                }
            }
        };

        std::process::exit(status.code().unwrap_or(1));
    }

    let exit_code = crate::shell::run_in_env(project_dir, cmd, args)?;
    std::process::exit(exit_code);
}

