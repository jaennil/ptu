use std::process::{Command, ExitStatus};

use alpm::{Alpm, SigLevel};
use color_eyre::eyre;

pub(crate) struct Pacman {
    handle: Alpm,
}

impl Pacman {
    pub(crate) fn new() -> eyre::Result<Self> {
        let handle = Alpm::new("/", "/var/lib/pacman")?;

        handle.register_syncdb("core", SigLevel::USE_DEFAULT)?;
        handle.register_syncdb("extra", SigLevel::USE_DEFAULT)?;
        handle.register_syncdb("community", SigLevel::USE_DEFAULT)?;

        Ok(Self { handle })
    }

    pub(crate) fn search_package(&self, package_name: &str) -> eyre::Result<Vec<Package>> {
        let mut packages = Vec::new();

        for db in self.handle.syncdbs() {
            for pkg in db.search([package_name].iter())? {
                let localdb = self.handle.localdb();
                let localpkg = localdb.pkg(pkg.name());
                packages.push(Package {
                    name: pkg.name().to_owned(),
                    source: db.name().to_owned(),
                    installed: localpkg.is_ok(),
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
    let status = Command::new("sudo")
        .arg("pacman")
        .arg("-S")
        .arg(package_name)
        .status()?;
    Ok(status)
}

pub(crate) fn remove_package(package_name: &str) -> eyre::Result<ExitStatus> {
    let status = Command::new("sudo")
        .arg("pacman")
        .arg("-R")
        .arg(package_name)
        .status()?;
    Ok(status)
}

pub(crate) fn update_install_package(package_name: &str) -> eyre::Result<ExitStatus> {
    let status = Command::new("sudo")
        .arg("pacman")
        .arg("-Syu")
        .arg(package_name)
        .status()?;
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
