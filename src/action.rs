use crate::pacman::Package;

pub(crate) enum Action {
    SearchPackage(String),
    InstallPackage { name: String, source: String },
    UpdateInstallPackage { name: String, source: String },
    RemovePackage { name: String, source: String },
    InstallPackages { packages: Vec<(String, String)> },  // (name, source)
    RemovePackages { packages: Vec<(String, String)> },   // (name, source)
    SelectPackage(Box<Package>),
}
