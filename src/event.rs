use crate::pacman::Package;

pub(crate) enum Event {
    FoundPackages(Vec<Package>),
    PackageSelected(Package),
    PackageInstalled(String),
    PackageRemoved(String),
}
