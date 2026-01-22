use std::collections::HashSet;
use std::process::{Command, ExitStatus};

use color_eyre::eyre::{self, eyre};
use serde::Deserialize;

use crate::pacman::Package;

const AUR_RPC_URL: &str = "https://aur.archlinux.org/rpc/v5/search";

#[derive(Deserialize)]
struct AurResponse {
    results: Vec<AurPackage>,
}

#[derive(Deserialize)]
struct AurPackage {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Description")]
    description: Option<String>,
    #[serde(rename = "Version")]
    version: String,
    #[serde(rename = "PackageBase")]
    package_base: String,
    #[serde(rename = "URL")]
    url: Option<String>,
    #[serde(rename = "Maintainer")]
    maintainer: Option<String>,
}

pub(crate) fn search(query: &str, installed_packages: &HashSet<String>) -> eyre::Result<Vec<Package>> {
    if query.is_empty() {
        return Ok(Vec::new());
    }

    tracing::debug!(query, "searching AUR");

    let url = format!("{}?arg={}", AUR_RPC_URL, query);
    let response: AurResponse = ureq::get(&url).call()?.body_mut().read_json()?;

    tracing::debug!(count = response.results.len(), "AUR search results");

    const MAX_RESULTS: usize = 50;
    let packages = response
        .results
        .into_iter()
        .take(MAX_RESULTS)
        .map(|pkg| Package {
            name: pkg.name.clone(),
            source: "aur".to_string(),
            installed: installed_packages.contains(&pkg.name),
            description: pkg.description.unwrap_or_default(),
            version: pkg.version,
            filename: "-".to_string(),
            base: pkg.package_base,
            url: pkg.url.unwrap_or_default(),
            packager: pkg.maintainer.unwrap_or_else(|| "orphan".to_string()),
            md5sum: "-".to_string(),
            sha256sum: "-".to_string(),
            arch: "-".to_string(),
        })
        .collect();

    Ok(packages)
}

pub(crate) fn install(package_name: &str) -> eyre::Result<ExitStatus> {
    let helper = find_aur_helper()?;
    tracing::info!(package_name, helper = %helper, "installing AUR package");

    let status = Command::new(&helper).arg("-S").arg(package_name).status()?;

    tracing::debug!(package_name, code = ?status.code(), "AUR install completed");
    Ok(status)
}

pub(crate) fn remove(package_name: &str) -> eyre::Result<ExitStatus> {
    let helper = find_aur_helper()?;
    tracing::info!(package_name, helper = %helper, "removing AUR package");

    let status = Command::new(&helper).arg("-R").arg(package_name).status()?;

    tracing::debug!(package_name, code = ?status.code(), "AUR remove completed");
    Ok(status)
}

fn find_aur_helper() -> eyre::Result<String> {
    for helper in ["yay", "paru"] {
        if Command::new("which")
            .arg(helper)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Ok(helper.to_string());
        }
    }
    Err(eyre!("no AUR helper found (install yay or paru)"))
}
