use crate::pacman::Package;

pub(crate) enum Event {
    FoundPackages(Vec<Package>),
    AurPackagesFound(Vec<Package>),
    PackageSelected(Box<Package>),
    PackageInstalled(String),
    PackageRemoved(String),
    PackagesInstalled(Vec<String>),
    PackagesRemoved(Vec<String>),
    OperationFailed { package: String, error: String },
}
