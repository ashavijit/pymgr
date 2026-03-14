use minijinja::{context, Environment};
use std::path::Path;

use crate::errors::PymgrResult;
use crate::output;

const ACTIVATE_BASH: &str = r#"export VIRTUAL_ENV="{{ env_path }}"
export PYMGR_ACTIVE="1"
export PYMGR_PROJECT="{{ project_name }}"

_pymgr_old_path="$PATH"
export PATH="$VIRTUAL_ENV/bin:$PATH"

_pymgr_old_ps1="${PS1-}"
PS1="({{ project_name }}) ${PS1-}"
export PS1

deactivate() {
    export PATH="$_pymgr_old_path"
    export PS1="$_pymgr_old_ps1"
    unset VIRTUAL_ENV PYMGR_ACTIVE PYMGR_PROJECT
    unset -f deactivate
}
"#;

const ACTIVATE_FISH: &str = r#"set -gx VIRTUAL_ENV "{{ env_path }}"
set -gx PYMGR_ACTIVE "1"
set -gx PYMGR_PROJECT "{{ project_name }}"

set -g _pymgr_old_path $PATH
set -gx PATH "$VIRTUAL_ENV/bin" $PATH

functions -c fish_prompt _pymgr_old_prompt
function fish_prompt
    echo -n "({{ project_name }}) "
    _pymgr_old_prompt
end

function deactivate
    set -gx PATH $_pymgr_old_path
    functions -e fish_prompt
    functions -c _pymgr_old_prompt fish_prompt
    functions -e _pymgr_old_prompt
    set -e VIRTUAL_ENV
    set -e PYMGR_ACTIVE
    set -e PYMGR_PROJECT
    functions -e deactivate
end
"#;

const ACTIVATE_PS1: &str = r#"$env:VIRTUAL_ENV = "{{ env_path }}"
$env:PYMGR_ACTIVE = "1"
$env:PYMGR_PROJECT = "{{ project_name }}"

$script:OldPath = $env:PATH
if ($IsWindows -or $env:OS -eq "Windows_NT") {
    $env:PATH = "$env:VIRTUAL_ENV\Scripts;$env:PATH"
} else {
    $env:PATH = "$env:VIRTUAL_ENV/bin:$env:PATH"
}

$script:OldPrompt = $function:prompt
function global:prompt {
    Write-Host "({{ project_name }}) " -NoNewline -ForegroundColor Cyan
    & $script:OldPrompt
}

function global:deactivate {
    $env:PATH = $script:OldPath
    $function:prompt = $script:OldPrompt
    Remove-Item env:VIRTUAL_ENV
    Remove-Item env:PYMGR_ACTIVE
    Remove-Item env:PYMGR_PROJECT
    Remove-Item function:deactivate
}
"#;

const ACTIVATE_BAT: &str = r#"@echo off
set "VIRTUAL_ENV={{ env_path }}"
set "PYMGR_ACTIVE=1"
set "PYMGR_PROJECT={{ project_name }}"
set "PROMPT=({{ project_name }}) %PROMPT%"
set "PATH=%VIRTUAL_ENV%\Scripts;%PATH%"
"#;

pub fn generate_all(env_dir: &Path, project_dir: &Path) -> PymgrResult<()> {
    let env_path = env_dir
        .canonicalize()
        .unwrap_or_else(|_| env_dir.to_path_buf())
        .to_string_lossy()
        .to_string();

    let project_name = project_dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let mut jinja = Environment::new();
    jinja.add_template("activate", ACTIVATE_BASH).unwrap();
    jinja.add_template("activate.fish", ACTIVATE_FISH).unwrap();
    jinja.add_template("activate.ps1", ACTIVATE_PS1).unwrap();
    jinja.add_template("activate.bat", ACTIVATE_BAT).unwrap();

    let ctx = context! {
        env_path => env_path,
        project_name => project_name,
        version => env!("CARGO_PKG_VERSION"),
    };

    let scripts_dir = if cfg!(windows) {
        env_dir.join("Scripts")
    } else {
        env_dir.join("bin")
    };
    std::fs::create_dir_all(&scripts_dir)?;

    let templates = [
        ("activate", "activate"),
        ("activate.fish", "activate.fish"),
        ("activate.ps1", "activate.ps1"),
        ("activate.bat", "activate.bat"),
    ];

    for (template_name, file_name) in &templates {
        let tmpl = jinja.get_template(template_name).unwrap();
        let rendered = tmpl.render(&ctx).map_err(|e| {
            crate::errors::PymgrError::Other(format!("Template render error: {}", e))
        })?;
        std::fs::write(scripts_dir.join(file_name), rendered)?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let activate_path = scripts_dir.join("activate");
        if activate_path.exists() {
            let mut perms = std::fs::metadata(&activate_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&activate_path, perms)?;
        }
    }

    output::print_verbose("Generated activation scripts");
    Ok(())
}

pub fn print_activation_script(project_dir: &Path) -> PymgrResult<String> {
    let config = crate::config::PymgrConfig::load(project_dir).unwrap_or_default();
    let env_dir = project_dir.join(config.env_dir());

    let script = if cfg!(windows) {
        env_dir.join("Scripts").join("activate.ps1")
    } else {
        env_dir.join("bin").join("activate")
    };

    if !script.exists() {
        return Err(crate::errors::PymgrError::coded(
            crate::errors::ErrorCode::EnvNotFound,
            "No activation script found",
        )
        .with_suggestion("Run `pymgr init` first"));
    }

    Ok(std::fs::read_to_string(&script)?)
}
