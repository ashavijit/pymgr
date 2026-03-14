use std::path::Path;

use crate::errors::PymgrResult;

pub fn exec_shell_init(shell: &str) -> PymgrResult<()> {
    let script = crate::shell::generate_shell_init(shell)?;
    print!("{}", script);
    Ok(())
}

pub fn exec_activate(project_dir: &Path) -> PymgrResult<()> {
    let script = crate::env::activation::print_activation_script(project_dir)?;
    print!("{}", script);
    Ok(())
}

pub fn exec_deactivate() -> PymgrResult<()> {
    if std::env::var("PYMGR_ACTIVE").is_ok() {
        if cfg!(windows) {
            println!("Remove-Item env:VIRTUAL_ENV; Remove-Item env:PYMGR_ACTIVE; Remove-Item env:PYMGR_PROJECT");
        } else {
            println!("deactivate");
        }
    } else {
        crate::output::print_info("No active environment to deactivate");
    }
    Ok(())
}

pub fn exec_shell(project_dir: &Path) -> PymgrResult<()> {
    crate::shell::spawn_shell(project_dir)?;
    Ok(())
}
