use crate::pacman::Package;

pub(crate) enum Action {
    SearchPackage(String),
    InstallPackage(String),
    SelectPackage(Package),
}
