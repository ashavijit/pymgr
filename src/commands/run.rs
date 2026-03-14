use std::path::Path;

use crate::errors::PymgrResult;

pub fn exec(project_dir: &Path, cmd: &str, args: &[String]) -> PymgrResult<()> {
    let exit_code = crate::shell::run_in_env(project_dir, cmd, args)?;
    std::process::exit(exit_code);
}
