use crate::pacman::Package;

pub(crate) enum Action {
    SearchPackage(String),
    InstallPackage(String),
    UpdateInstallPackage(String),
    RemovePackage(String),
    SelectPackage(Package),
}
