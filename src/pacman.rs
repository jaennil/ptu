use std::collections::HashSet;
use std::process::{Command, ExitStatus};

use alpm::{Alpm, SigLevel};
use color_eyre::eyre;

pub(crate) struct Pacman {
    handle: Alpm,
    installed_cache: HashSet<String>,
}

impl Pacman {
    pub(crate) fn new() -> eyre::Result<Self> {
        tracing::debug!("initializing pacman handle");
        let handle = Alpm::new("/", "/var/lib/pacman")?;

        tracing::debug!("registering sync databases");
        handle.register_syncdb("core", SigLevel::USE_DEFAULT)?;
        handle.register_syncdb("extra", SigLevel::USE_DEFAULT)?;
        handle.register_syncdb("community", SigLevel::USE_DEFAULT)?;

        // Cache installed packages at startup
        let installed_cache: HashSet<String> = handle
            .localdb()
            .pkgs()
            .iter()
            .map(|p| p.name().to_string())
            .collect();
        tracing::debug!(count = installed_cache.len(), "cached installed packages");

        tracing::debug!("pacman handle initialized successfully");
        Ok(Self {
            handle,
            installed_cache,
        })
    }

    /// Returns a reference to the cached installed packages (O(1) lookup)
    pub(crate) fn installed_packages(&self) -> &HashSet<String> {
        &self.installed_cache
    }

    /// Mark a package as installed in cache
    pub(crate) fn mark_installed(&mut self, name: &str) {
        self.installed_cache.insert(name.to_string());
    }

    /// Mark a package as removed from cache
    pub(crate) fn mark_removed(&mut self, name: &str) {
        self.installed_cache.remove(name);
    }

    pub(crate) fn search_package(&self, package_name: &str) -> eyre::Result<Vec<Package>> {
        let mut packages = Vec::new();

        for db in self.handle.syncdbs() {
            for pkg in db.search([package_name].iter())? {
                packages.push(Package {
                    name: pkg.name().to_owned(),
                    source: db.name().to_owned(),
                    installed: self.installed_cache.contains(pkg.name()),
                    description: pkg.desc().unwrap_or("-").to_owned(),
                    version: pkg.version().to_string(),
                    filename: pkg.filename().unwrap_or("-").to_owned(),
                    base: pkg.base().unwrap_or("-").to_owned(),
                    url: pkg.url().unwrap_or("-").to_owned(),
                    packager: pkg.packager().unwrap_or("-").to_owned(),
                    md5sum: pkg.md5sum().unwrap_or("-").to_owned(),
                    sha256sum: pkg.sha256sum().unwrap_or("-").to_owned(),
                    arch: pkg.arch().unwrap_or("-").to_owned(),
                });
            }
        }

        Ok(packages)
    }
}

pub(crate) fn install_package(package_name: &str) -> eyre::Result<ExitStatus> {
    tracing::debug!(package_name, "executing pacman -S");
    let status = Command::new("sudo")
        .arg("pacman")
        .arg("-S")
        .arg(package_name)
        .status()?;
    tracing::debug!(package_name, code = ?status.code(), "pacman -S completed");
    Ok(status)
}

pub(crate) fn remove_package(package_name: &str) -> eyre::Result<ExitStatus> {
    tracing::debug!(package_name, "executing pacman -R");
    let status = Command::new("sudo")
        .arg("pacman")
        .arg("-R")
        .arg(package_name)
        .status()?;
    tracing::debug!(package_name, code = ?status.code(), "pacman -R completed");
    Ok(status)
}

pub(crate) fn update_install_package(package_name: &str) -> eyre::Result<ExitStatus> {
    tracing::debug!(package_name, "executing pacman -Syu");
    let status = Command::new("sudo")
        .arg("pacman")
        .arg("-Syu")
        .arg(package_name)
        .status()?;
    tracing::debug!(package_name, code = ?status.code(), "pacman -Syu completed");
    Ok(status)
}

#[derive(Clone, Default)]
pub(crate) struct Package {
    pub(crate) name: String,
    pub(crate) source: String,
    pub(crate) installed: bool,
    pub(crate) description: String,
    pub(crate) version: String,
    pub(crate) filename: String,
    pub(crate) base: String,
    pub(crate) url: String,
    pub(crate) packager: String,
    pub(crate) md5sum: String,
    pub(crate) sha256sum: String,
    pub(crate) arch: String,
}
