use crate::config::PymgrConfig;
use crate::errors::{ErrorCode, PymgrError, PymgrResult};
use crate::lockfile::Lockfile;
use crate::output;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct OsvQuery {
    queries: Vec<OsvPackageQuery>,
}

#[derive(Serialize)]
struct OsvPackageQuery {
    package: OsvPackageName,
    version: String,
}

#[derive(Serialize)]
struct OsvPackageName {
    name: String,
    ecosystem: String,
}

#[derive(Deserialize, Debug)]
struct OsvResponse {
    results: Option<Vec<OsvResult>>,
}

#[derive(Deserialize, Debug)]
struct OsvResult {
    vulns: Option<Vec<OsvVuln>>,
}

#[derive(Deserialize, Debug)]
struct OsvVuln {
    id: String,
    summary: Option<String>,
    details: Option<String>,
}

pub async fn exec(json: bool) -> PymgrResult<()> {
    let project_dir = PymgrConfig::find_project_root()?;
    let path = project_dir.join("pymgr.lock");

    if !path.exists() {
        return Err(PymgrError::coded(
            ErrorCode::LockfileMissing,
            "No pymgr.lock found to audit.",
        ));
    }

    let lockfile = Lockfile::load(&path)?;
    if lockfile.packages.is_empty() {
        if json {
            output::print_json(&serde_json::json!({"vulnerabilities": []}));
        } else {
            output::print_success("No packages to audit.");
        }
        return Ok(());
    }

    if !json {
        println!("Auditing {} packages against OSV database...", lockfile.packages.len());
    }

    let mut queries = Vec::new();
    for pkg in &lockfile.packages {
        queries.push(OsvPackageQuery {
            package: OsvPackageName {
                name: pkg.name.clone(),
                ecosystem: "PyPI".to_string(),
            },
            version: pkg.version.clone(),
        });
    }

    let client = reqwest::Client::new();
    let res = client
        .post("https://api.osv.dev/v1/querybatch")
        .json(&OsvQuery { queries })
        .send()
        .await
        .map_err(|e| {
            PymgrError::coded(ErrorCode::NetworkError, format!("Failed to reach OSV API: {}", e))
        })?;

    if !res.status().is_success() {
        return Err(PymgrError::coded(ErrorCode::NetworkError, format!("OSV API error: {}", res.status())));
    }

    let osv_resp: OsvResponse = res.json().await.map_err(|e| {
        PymgrError::coded(ErrorCode::NetworkError, format!("Invalid OSV response: {}", e))
    })?;

    let results = osv_resp.results.unwrap_or_default();
    let mut total_vulns = 0;
    
    let mut report = Vec::new();

    let mut table = comfy_table::Table::new();
    table.load_preset(comfy_table::presets::UTF8_FULL);
    table.set_header(vec![
        comfy_table::Cell::new("Package").fg(comfy_table::Color::Red),
        comfy_table::Cell::new("Version").fg(comfy_table::Color::Red),
        comfy_table::Cell::new("Vuln ID").fg(comfy_table::Color::Red),
        comfy_table::Cell::new("Summary").fg(comfy_table::Color::Red),
    ]);

    for (i, result) in results.into_iter().enumerate() {
        let pkg = &lockfile.packages[i];
        if let Some(vulns) = result.vulns {
            for vuln in vulns {
                total_vulns += 1;
                report.push(serde_json::json!({
                    "package": pkg.name,
                    "version": pkg.version,
                    "id": vuln.id,
                    "summary": vuln.summary.clone().unwrap_or_default()
                }));

                if !json {
                    table.add_row(vec![
                        pkg.name.clone(),
                        pkg.version.clone(),
                        vuln.id.clone(),
                        vuln.summary.clone().unwrap_or_else(|| "No summary available".to_string()),
                    ]);
                }
            }
        }
    }

    if json {
        output::print_json(&serde_json::json!({
            "total_vulnerabilities": total_vulns,
            "vulnerabilities": report
        }));
    } else if total_vulns == 0 {
        output::print_success("No vulnerabilities found! Your environment is secure.");
    } else {
        println!("\n{}", table);
        output::print_warning(&format!("\nFound {} vulnerabilities. Please update the affected packages.", total_vulns));
    }

    Ok(())
}
