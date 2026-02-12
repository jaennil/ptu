use std::collections::HashSet;
use std::process::{Command, ExitStatus};

use alpm::{Alpm, SigLevel};
use color_eyre::eyre;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum SearchMode {
    #[default]
    Package,
    File,
}

pub(crate) struct Pacman {
    handle: Alpm,
    installed_cache: HashSet<String>,
}

impl Pacman {
    pub(crate) fn new() -> eyre::Result<Self> {
        tracing::debug!("initializing pacman handle");
        let handle = Alpm::new("/", "/var/lib/pacman")?;

        tracing::debug!("registering sync databases");
        handle.register_syncdb("core", SigLevel::USE_DEFAULT)?;
        handle.register_syncdb("extra", SigLevel::USE_DEFAULT)?;
        handle.register_syncdb("community", SigLevel::USE_DEFAULT)?;

        // Cache installed packages at startup
        let installed_cache: HashSet<String> = handle
            .localdb()
            .pkgs()
            .iter()
            .map(|p| p.name().to_string())
            .collect();
        tracing::debug!(count = installed_cache.len(), "cached installed packages");

        tracing::debug!("pacman handle initialized successfully");
        Ok(Self {
            handle,
            installed_cache,
        })
    }

    /// Returns a reference to the cached installed packages (O(1) lookup)
    pub(crate) fn installed_packages(&self) -> &HashSet<String> {
        &self.installed_cache
    }

    /// Mark a package as installed in cache
    pub(crate) fn mark_installed(&mut self, name: &str) {
        self.installed_cache.insert(name.to_string());
    }

    /// Mark a package as removed from cache
    pub(crate) fn mark_removed(&mut self, name: &str) {
        self.installed_cache.remove(name);
    }

    pub(crate) fn get_installed_packages(&self) -> Vec<Package> {
        tracing::debug!("loading all installed packages from localdb");
        let mut packages = Vec::new();

        for pkg in self.handle.localdb().pkgs() {
            // Determine source by checking which syncdb contains the package
            let source = self
                .handle
                .syncdbs()
                .iter()
                .find(|db| db.pkg(pkg.name()).is_ok())
                .map(|db| db.name().to_string())
                .unwrap_or_else(|| "aur".to_string());

            packages.push(Package {
                name: pkg.name().to_owned(),
                source,
                installed: true,
                description: pkg.desc().unwrap_or("-").to_owned(),
                version: pkg.version().to_string(),
                filename: pkg.filename().unwrap_or("-").to_owned(),
                base: pkg.base().unwrap_or("-").to_owned(),
                url: pkg.url().unwrap_or("-").to_owned(),
                packager: pkg.packager().unwrap_or("-").to_owned(),
                md5sum: pkg.md5sum().unwrap_or("-").to_owned(),
                sha256sum: pkg.sha256sum().unwrap_or("-").to_owned(),
                arch: pkg.arch().unwrap_or("-").to_owned(),
                size: pkg.isize(),
                licenses: pkg.licenses().iter().map(|s| s.to_string()).collect(),
                depends: pkg.depends().iter().map(|d| d.name().to_string()).collect(),
                optdepends: pkg.optdepends().iter().map(|d| d.name().to_string()).collect(),
                groups: pkg.groups().iter().map(|s| s.to_string()).collect(),
                provides: pkg.provides().iter().map(|d| d.name().to_string()).collect(),
                conflicts: pkg.conflicts().iter().map(|d| d.name().to_string()).collect(),
                build_date: Some(pkg.build_date()),
                install_date: Some(pkg.install_date().unwrap_or(0)),
                matched_files: Vec::new(),
                votes: None,
                popularity: None,
                out_of_date: None,
                first_submitted: None,
                last_modified: None,
            });
        }

        tracing::debug!(count = packages.len(), "loaded installed packages");
        packages
    }

    pub(crate) fn search_package(&self, package_name: &str) -> eyre::Result<Vec<Package>> {
        const MAX_RESULTS: usize = 100;
        let mut packages = Vec::with_capacity(MAX_RESULTS);

        'outer: for db in self.handle.syncdbs() {
            for pkg in db.search([package_name].iter())? {
                packages.push(Package {
                    name: pkg.name().to_owned(),
                    source: db.name().to_owned(),
                    installed: self.installed_cache.contains(pkg.name()),
                    description: pkg.desc().unwrap_or("-").to_owned(),
                    version: pkg.version().to_string(),
                    filename: pkg.filename().unwrap_or("-").to_owned(),
                    base: pkg.base().unwrap_or("-").to_owned(),
                    url: pkg.url().unwrap_or("-").to_owned(),
                    packager: pkg.packager().unwrap_or("-").to_owned(),
                    md5sum: pkg.md5sum().unwrap_or("-").to_owned(),
                    sha256sum: pkg.sha256sum().unwrap_or("-").to_owned(),
                    arch: pkg.arch().unwrap_or("-").to_owned(),
                    size: pkg.isize(),
                    licenses: pkg.licenses().iter().map(|s| s.to_string()).collect(),
                    depends: pkg.depends().iter().map(|d| d.name().to_string()).collect(),
                    optdepends: pkg.optdepends().iter().map(|d| d.name().to_string()).collect(),
                    groups: pkg.groups().iter().map(|s| s.to_string()).collect(),
                    provides: pkg.provides().iter().map(|d| d.name().to_string()).collect(),
                    conflicts: pkg.conflicts().iter().map(|d| d.name().to_string()).collect(),
                    build_date: Some(pkg.build_date()),
                    install_date: None,
                    matched_files: Vec::new(),
                    votes: None,
                    popularity: None,
                    out_of_date: None,
                    first_submitted: None,
                    last_modified: None,
                });
                if packages.len() >= MAX_RESULTS {
                    tracing::debug!("pacman search hit limit of {} results", MAX_RESULTS);
                    break 'outer;
                }
            }
        }

        Ok(packages)
    }
}

pub(crate) fn install_package(package_name: &str) -> eyre::Result<ExitStatus> {
    tracing::debug!(package_name, "executing pacman -S");
    let status = Command::new("sudo")
        .arg("pacman")
        .arg("-S")
        .arg(package_name)
        .status()?;
    tracing::debug!(package_name, code = ?status.code(), "pacman -S completed");
    Ok(status)
}

pub(crate) fn remove_package(package_name: &str) -> eyre::Result<ExitStatus> {
    tracing::debug!(package_name, "executing pacman -R");
    let status = Command::new("sudo")
        .arg("pacman")
        .arg("-R")
        .arg(package_name)
        .status()?;
    tracing::debug!(package_name, code = ?status.code(), "pacman -R completed");
    Ok(status)
}

pub(crate) fn update_install_package(package_name: &str) -> eyre::Result<ExitStatus> {
    tracing::debug!(package_name, "executing pacman -Syu");
    let status = Command::new("sudo")
        .arg("pacman")
        .arg("-Syu")
        .arg(package_name)
        .status()?;
    tracing::debug!(package_name, code = ?status.code(), "pacman -Syu completed");
    Ok(status)
}

pub(crate) fn install_packages(names: &[&str]) -> eyre::Result<ExitStatus> {
    tracing::debug!(?names, "executing pacman -S for multiple packages");
    let status = Command::new("sudo")
        .arg("pacman")
        .arg("-S")
        .args(names)
        .status()?;
    tracing::debug!(?names, code = ?status.code(), "pacman -S batch completed");
    Ok(status)
}

pub(crate) fn remove_packages(names: &[&str]) -> eyre::Result<ExitStatus> {
    tracing::debug!(?names, "executing pacman -R for multiple packages");
    let status = Command::new("sudo")
        .arg("pacman")
        .arg("-R")
        .args(names)
        .status()?;
    tracing::debug!(?names, code = ?status.code(), "pacman -R batch completed");
    Ok(status)
}

/// Search for packages by file/command name using `pacman -F`.
/// This runs an external command (~3s) and parses its output.
pub(crate) fn search_file(query: &str, installed: &HashSet<String>) -> eyre::Result<Vec<Package>> {
    if query.is_empty() {
        return Ok(Vec::new());
    }

    tracing::debug!(query, "executing pacman -F for file search");

    let output = Command::new("pacman")
        .arg("-F")
        .arg("--color=never")
        .arg(query)
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    tracing::debug!(
        query,
        exit_code = ?output.status.code(),
        stdout_lines = stdout.lines().count(),
        "pacman -F completed"
    );

    if !output.status.success() {
        // pacman -F returns 1 when no results found
        tracing::debug!(query, "pacman -F found no results");
        return Ok(Vec::new());
    }

    let mut packages: Vec<Package> = Vec::new();

    for line in stdout.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            // File line: belongs to the last package
            let file_path = line.trim().to_string();
            if let Some(last_pkg) = packages.last_mut() {
                last_pkg.matched_files.push(file_path);
            }
        } else {
            // Package line format: "repo/name version"
            // e.g. "core/coreutils 9.4-2"
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Parse "repo/name version" or "repo/name version [installed]" etc.
            let mut parts = line.splitn(2, ' ');
            let repo_name = parts.next().unwrap_or("");
            let rest = parts.next().unwrap_or("");

            let (repo, name) = if let Some((r, n)) = repo_name.split_once('/') {
                (r.to_string(), n.to_string())
            } else {
                continue;
            };

            // Version is the first word in rest
            let version = rest.split_whitespace().next().unwrap_or("").to_string();
            let is_installed = installed.contains(&name) || rest.contains("[installed]");

            tracing::trace!(repo, name, version, is_installed, "parsed file search result package");

            packages.push(Package {
                name,
                source: repo,
                installed: is_installed,
                version,
                description: String::new(),
                ..Default::default()
            });
        }
    }

    tracing::debug!(
        query,
        count = packages.len(),
        "file search parsed packages"
    );

    Ok(packages)
}

#[derive(Clone, Default)]
pub(crate) struct Package {
    pub(crate) name: String,
    pub(crate) source: String,
    pub(crate) installed: bool,
    pub(crate) description: String,
    pub(crate) version: String,
    pub(crate) filename: String,
    pub(crate) base: String,
    pub(crate) url: String,
    pub(crate) packager: String,
    pub(crate) md5sum: String,
    pub(crate) sha256sum: String,
    pub(crate) arch: String,
    pub(crate) size: i64,
    pub(crate) licenses: Vec<String>,
    pub(crate) depends: Vec<String>,
    pub(crate) optdepends: Vec<String>,
    pub(crate) groups: Vec<String>,
    pub(crate) provides: Vec<String>,
    pub(crate) conflicts: Vec<String>,
    pub(crate) build_date: Option<i64>,
    pub(crate) install_date: Option<i64>,
    // File search results
    pub(crate) matched_files: Vec<String>,
    // AUR-specific fields
    pub(crate) votes: Option<i64>,
    pub(crate) popularity: Option<f64>,
    pub(crate) out_of_date: Option<i64>,
    pub(crate) first_submitted: Option<i64>,
    pub(crate) last_modified: Option<i64>,
}

/// Format bytes into human-readable string (KiB, MiB, GiB)
pub(crate) fn format_size(bytes: i64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;

    let bytes = bytes as f64;
    if bytes >= GIB {
        format!("{:.2} GiB", bytes / GIB)
    } else if bytes >= MIB {
        format!("{:.2} MiB", bytes / MIB)
    } else if bytes >= KIB {
        format!("{:.2} KiB", bytes / KIB)
    } else {
        format!("{} B", bytes as i64)
    }
}

/// Format unix timestamp to human-readable date
pub(crate) fn format_timestamp(ts: i64) -> String {
    use std::time::{Duration, UNIX_EPOCH};
    let datetime = UNIX_EPOCH + Duration::from_secs(ts as u64);
    let secs = datetime.duration_since(UNIX_EPOCH).unwrap().as_secs();

    // Simple date formatting (YYYY-MM-DD)
    let days = secs / 86400;
    let years = 1970 + days / 365;
    let remaining_days = days % 365;
    let month = remaining_days / 30 + 1;
    let day = remaining_days % 30 + 1;

    format!("{:04}-{:02}-{:02}", years, month.min(12), day.min(31))
}
