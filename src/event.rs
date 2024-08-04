use crate::pacman::Package;

pub(crate) enum Event {
    FoundPackages(Vec<Package>),
}
