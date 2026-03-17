use std::path::{Path, PathBuf};
use std::process::Command;

use crate::errors::{ErrorCode, PymgrError, PymgrResult};
use crate::output;

pub fn build_sdist(source_dir: &Path, out_dir: &Path, python_exe: &Path) -> PymgrResult<PathBuf> {
    output::print_verbose(&format!(
        "Building source distribution at {}",
        source_dir.display()
    ));

    let status = Command::new(python_exe)
        .args([
            "-m",
            "pip",
            "wheel",
            "--no-deps",
            "-w",
            out_dir.to_str().unwrap(),
            source_dir.to_str().unwrap(),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()?;

    if !status.success() {
        return Err(PymgrError::coded(
            ErrorCode::InstallError,
            "Failed to build source distribution. PEP 517 build failed.",
        ));
    }

    for entry in std::fs::read_dir(out_dir)? {
        let entry = entry?;
        if entry.path().extension().is_some_and(|e| e == "whl") {
            return Ok(entry.path());
        }
    }

    Err(PymgrError::coded(
        ErrorCode::InstallError,
        "Build succeeded but no wheel found in output.",
    ))
}
