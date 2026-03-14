use std::path::Path;
use std::process::Command;

use crate::env::manager;
use crate::errors::{ErrorCode, PymgrError, PymgrResult};
use crate::output;

pub fn generate_shell_init(shell: &str) -> PymgrResult<String> {
    match shell.to_lowercase().as_str() {
        "bash" | "zsh" => Ok(generate_bash_hook()),
        "fish" => Ok(generate_fish_hook()),
        "powershell" | "pwsh" => Ok(generate_powershell_hook()),
        _ => Err(PymgrError::coded(
            ErrorCode::ConfigError,
            format!("Unsupported shell: {}", shell),
        )
        .with_suggestions(vec![
            "Supported shells: bash, zsh, fish, powershell".to_string(),
            "Usage: eval \"$(pymgr shell-init bash)\"".to_string(),
        ])),
    }
}

fn generate_bash_hook() -> String {
    r#"__pymgr_chpwd() {
    if [ -f ".pymgr/env.json" ]; then
        if [ "$PYMGR_PROJECT" != "$(pwd)" ]; then
            if [ -f ".pymgr/env/bin/activate" ]; then
                source .pymgr/env/bin/activate
                export PYMGR_PROJECT="$(pwd)"
            fi
        fi
    elif [ -n "$PYMGR_ACTIVE" ]; then
        deactivate 2>/dev/null
        unset PYMGR_PROJECT
    fi
}
cd() { builtin cd "$@" && __pymgr_chpwd; }
pushd() { builtin pushd "$@" && __pymgr_chpwd; }
popd() { builtin popd "$@" && __pymgr_chpwd; }
__pymgr_chpwd
"#
    .to_string()
}

fn generate_fish_hook() -> String {
    r#"function __pymgr_chpwd --on-variable PWD
    if test -f ".pymgr/env.json"
        if test "$PYMGR_PROJECT" != (pwd)
            if test -f ".pymgr/env/bin/activate.fish"
                source .pymgr/env/bin/activate.fish
                set -gx PYMGR_PROJECT (pwd)
            end
        end
    else if set -q PYMGR_ACTIVE
        deactivate 2>/dev/null
        set -e PYMGR_PROJECT
    end
end
__pymgr_chpwd
"#
    .to_string()
}

fn generate_powershell_hook() -> String {
    r#"function __pymgr_chpwd {
    if (Test-Path ".pymgr\env.json") {
        if ($env:PYMGR_PROJECT -ne (Get-Location).Path) {
            $activateScript = ".pymgr\env\Scripts\activate.ps1"
            if (Test-Path $activateScript) {
                & $activateScript
                $env:PYMGR_PROJECT = (Get-Location).Path
            }
        }
    } elseif ($env:PYMGR_ACTIVE) {
        if (Get-Command deactivate -ErrorAction SilentlyContinue) {
            deactivate
        }
        Remove-Item env:PYMGR_PROJECT -ErrorAction SilentlyContinue
    }
}

$ExecutionContext.SessionState.InvokeCommand.LocationChangedAction = {
    __pymgr_chpwd
}
__pymgr_chpwd
"#
    .to_string()
}

pub fn run_in_env(project_dir: &Path, cmd: &str, args: &[String]) -> PymgrResult<i32> {
    let python_path = manager::get_env_python(project_dir)?;
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

    let actual_cmd = if cmd == "python" {
        python_path.to_string_lossy().to_string()
    } else {
        let cmd_in_env = if cfg!(windows) {
            bin_dir.join(format!("{}.exe", cmd))
        } else {
            bin_dir.join(cmd)
        };
        if cmd_in_env.exists() {
            cmd_in_env.to_string_lossy().to_string()
        } else {
            cmd.to_string()
        }
    };

    let status = Command::new(&actual_cmd)
        .args(args)
        .env("PATH", &new_path)
        .env("VIRTUAL_ENV", env_dir)
        .env("PYMGR_ACTIVE", "1")
        .status()?;

    Ok(status.code().unwrap_or(1))
}

pub fn spawn_shell(project_dir: &Path) -> PymgrResult<()> {
    let python_path = manager::get_env_python(project_dir)?;
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

    let shell_cmd = if cfg!(windows) {
        std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
    } else {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
    };

    output::print_info("Spawning shell with environment active (type 'exit' to leave)");

    let _status = Command::new(&shell_cmd)
        .env("PATH", &new_path)
        .env("VIRTUAL_ENV", env_dir)
        .env("PYMGR_ACTIVE", "1")
        .status()?;

    Ok(())
}
