use std::collections::HashSet;
use std::process::{Command, ExitStatus};

use color_eyre::eyre::{self, eyre};
use serde::Deserialize;

use crate::pacman::Package;

const AUR_RPC_URL: &str = "https://aur.archlinux.org/rpc/v5/search";
const MAX_RESULTS: usize = 50;

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
    #[serde(rename = "NumVotes")]
    num_votes: i64,
    #[serde(rename = "Popularity")]
    popularity: f64,
    #[serde(rename = "OutOfDate")]
    out_of_date: Option<i64>,
    #[serde(rename = "FirstSubmitted")]
    first_submitted: i64,
    #[serde(rename = "LastModified")]
    last_modified: i64,
}

/// Async AUR search using reqwest
pub(crate) async fn search(query: &str, installed_packages: &HashSet<String>) -> eyre::Result<Vec<Package>> {
    if query.is_empty() {
        return Ok(Vec::new());
    }

    tracing::debug!(query, "searching AUR (async)");

    let response: AurResponse = reqwest::Client::new()
        .get(AUR_RPC_URL)
        .query(&[("arg", query)])
        .send()
        .await?
        .json()
        .await?;

    tracing::debug!(count = response.results.len(), "AUR search results");

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
            size: 0,
            licenses: Vec::new(),
            depends: Vec::new(),
            optdepends: Vec::new(),
            groups: Vec::new(),
            provides: Vec::new(),
            conflicts: Vec::new(),
            build_date: None,
            install_date: None,
            matched_files: Vec::new(),
            votes: Some(pkg.num_votes),
            popularity: Some(pkg.popularity),
            out_of_date: pkg.out_of_date,
            first_submitted: Some(pkg.first_submitted),
            last_modified: Some(pkg.last_modified),
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

pub(crate) fn install_packages(names: &[&str]) -> eyre::Result<ExitStatus> {
    let helper = find_aur_helper()?;
    tracing::info!(?names, helper = %helper, "installing multiple AUR packages");

    let status = Command::new(&helper).arg("-S").args(names).status()?;

    tracing::debug!(?names, code = ?status.code(), "AUR batch install completed");
    Ok(status)
}

pub(crate) fn remove_packages(names: &[&str]) -> eyre::Result<ExitStatus> {
    let helper = find_aur_helper()?;
    tracing::info!(?names, helper = %helper, "removing multiple AUR packages");

    let status = Command::new(&helper).arg("-R").args(names).status()?;

    tracing::debug!(?names, code = ?status.code(), "AUR batch remove completed");
    Ok(status)
}

fn find_aur_helper() -> eyre::Result<String> {
    for helper in ["yay", "paru"] {
        if Command::new(helper)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            tracing::debug!(helper, "found AUR helper");
            return Ok(helper.to_string());
        }
    }
    Err(eyre!("no AUR helper found (install yay or paru)"))
}
