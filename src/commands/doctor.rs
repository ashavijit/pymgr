use std::path::Path;

use crate::errors::PymgrResult;

pub fn exec(project_dir: &Path) -> PymgrResult<()> {
    crate::doctor::run_diagnostics(project_dir)
}
