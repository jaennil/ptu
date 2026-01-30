use crate::pacman::Package;

#[derive(Default, Clone, Copy, PartialEq, Debug)]
pub enum InstallFilter {
    #[default]
    All,
    Installed,
    NotInstalled,
}

impl InstallFilter {
    pub fn cycle(&mut self) {
        *self = match self {
            InstallFilter::All => InstallFilter::Installed,
            InstallFilter::Installed => InstallFilter::NotInstalled,
            InstallFilter::NotInstalled => InstallFilter::All,
        };
        tracing::debug!(filter = ?self, "install filter cycled");
    }

    pub fn label(&self) -> &'static str {
        match self {
            InstallFilter::All => "All",
            InstallFilter::Installed => "Installed",
            InstallFilter::NotInstalled => "Not Installed",
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq, Debug)]
pub enum SourceFilter {
    #[default]
    All,
    Aur,
    Pacman,
}

impl SourceFilter {
    pub fn label(&self) -> &'static str {
        match self {
            SourceFilter::All => "All",
            SourceFilter::Aur => "AUR",
            SourceFilter::Pacman => "Pacman",
        }
    }
}

#[derive(Default, Clone)]
pub struct PackageFilter {
    pub install: InstallFilter,
    pub source: SourceFilter,
}

impl PackageFilter {
    pub fn matches(&self, package: &Package) -> bool {
        // Check install filter
        let install_match = match self.install {
            InstallFilter::All => true,
            InstallFilter::Installed => package.installed,
            InstallFilter::NotInstalled => !package.installed,
        };

        // Check source filter
        let source_match = match self.source {
            SourceFilter::All => true,
            SourceFilter::Aur => package.source == "aur",
            SourceFilter::Pacman => package.source != "aur",
        };

        install_match && source_match
    }

    pub fn clear(&mut self) {
        tracing::debug!("clearing all filters");
        self.install = InstallFilter::All;
        self.source = SourceFilter::All;
    }

    pub fn is_active(&self) -> bool {
        self.install != InstallFilter::All || self.source != SourceFilter::All
    }
}
