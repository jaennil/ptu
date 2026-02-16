use std::collections::HashMap;

use crate::pacman::Package;

pub(crate) enum Event {
    FoundPackages(Vec<Package>),
    AurSearchStarted,
    AurPackagesFound(Vec<Package>),
    AurUpdatesChecked(HashMap<String, String>),
    PackageSelected(Box<Package>),
    PackageInstalled(String),
    PackageRemoved(String),
    PackagesInstalled(Vec<String>),
    PackagesRemoved(Vec<String>),
    OperationFailed { package: String, error: String },
}
