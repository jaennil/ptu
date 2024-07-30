use crate::{components::home::Focus, pacman::Package};

#[derive(Clone)]
pub enum Action {
    InstallPackage(String),
    SearchPackage(String),
    FoundPackages(Vec<Package>),
    Focus(Focus),
}
