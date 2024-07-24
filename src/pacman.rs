use std::process::Command;

use alpm::{Alpm, SigLevel};

// TODO: replace with alpm package
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

impl Default for Pacman {
    fn default() -> Self {
        Pacman::new()
    }
}

impl Pacman {
    pub fn new() -> Pacman {
        let handle = Alpm::new("/", "/var/lib/pacman").unwrap();

        handle
            .register_syncdb("core", SigLevel::USE_DEFAULT)
            .unwrap();
        handle
            .register_syncdb("extra", SigLevel::USE_DEFAULT)
            .unwrap();
        handle
            .register_syncdb("community", SigLevel::USE_DEFAULT)
            .unwrap();

        Pacman { handle }
    }

    pub fn search(&self, package: &str) -> Vec<Package> {
        let mut res = Vec::new();
        for db in self.handle.syncdbs() {
            for pkg in db.search([package].iter()).unwrap() {
                res.push(Package {
                    name: pkg.name().to_string(),
                    description: pkg.desc().map(str::to_string),
                });
            }
        }

        res
    }
}

pub fn install(package_name: &str) {
    Command::new("sudo")
        .arg("pacman")
        .arg("-S")
        .arg(package_name)
        .status()
        .unwrap();
}
