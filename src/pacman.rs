use alpm::{Alpm, SigLevel};

pub struct Package {
    pub name: String,
    pub description: Option<String>,
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
