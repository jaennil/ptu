use std::{io, process::Command};

use alpm::{Alpm, SigLevel};

// TODO: replace with alpm package
#[derive(Clone)]
pub struct Package {
    pub name: String,
    pub description: Option<String>,
}

impl Package {
    pub fn name(&self) -> &str {
        return &self.name;
    }
}

pub struct Pacman {
    handle: Alpm,
}

//impl Default for Pacman {
//    fn default() -> io::Self {
//        Pacman::new()
//    }
//}

impl Pacman {
    pub fn new() -> alpm::Result<Self> {
        let handle = Alpm::new("/", "/var/lib/pacman")?;

        handle.register_syncdb("core", SigLevel::USE_DEFAULT)?;
        handle.register_syncdb("extra", SigLevel::USE_DEFAULT)?;
        handle.register_syncdb("community", SigLevel::USE_DEFAULT)?;

        Ok(Pacman { handle })
    }

    pub fn search(&self, package: &str) -> Vec<Package> {
        let mut packages = Vec::new();

        for db in self.handle.syncdbs() {
            for pkg in db.search([package].iter()).unwrap() {
                packages.push(Package {
                    name: pkg.name().to_string(),
                    description: pkg.desc().map(str::to_string),
                });
            }
        }

        packages
    }
}

pub fn install(package_name: &str) -> io::Result<()> {
    Command::new("sudo")
        .arg("pacman")
        .arg("-S")
        .arg(package_name)
        .status()?;
    Ok(())
}
