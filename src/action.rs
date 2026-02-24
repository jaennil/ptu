use crate::pacman::Package;

#[derive(Clone)]
pub(crate) enum Action {
    SearchPackage(String),
    SearchFile(String),
    InstallPackage { name: String, source: String },
    UpdateInstallPackage { name: String, source: String },
    RemovePackage { name: String, source: String },
    InstallPackages { packages: Vec<(String, String)> },  // (name, source)
    RemovePackages { packages: Vec<(String, String)> },   // (name, source)
    SelectPackage(Box<Package>),
    OpenUrl(String),
    RefreshInstalled,
    SystemUpgrade,
    RepoUpgrade,
    AurUpgrade,
    ToggleBasket { name: String, source: String },
}
