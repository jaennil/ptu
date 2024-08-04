use std::process::Command;

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
                packages.push(Package {
                    name: pkg.name().to_owned(),
                });
            }
        }

        Ok(packages)
    }
}

pub(crate) fn install_package(package_name: &str) -> eyre::Result<()> {
    Command::new("sudo")
        .arg("pacman")
        .arg("-S")
        .arg(package_name)
        .status()?;
    Ok(())
}

#[derive(Clone)]
pub(crate) struct Package {
    pub(crate) name: String,
}
