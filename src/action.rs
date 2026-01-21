use crate::pacman::Package;

pub(crate) enum Action {
    SearchPackage(String),
    InstallPackage { name: String, source: String },
    UpdateInstallPackage { name: String, source: String },
    RemovePackage { name: String, source: String },
    SelectPackage(Box<Package>),
}
