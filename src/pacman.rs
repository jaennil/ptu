use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
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

        let repos = read_repos_from_pacman_conf();
        tracing::debug!(?repos, "registering sync databases from pacman.conf");
        for repo in &repos {
            if let Err(e) = handle.register_syncdb(repo.as_str(), SigLevel::USE_DEFAULT) {
                tracing::warn!(repo, %e, "failed to register sync database");
            }
        }

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

    pub(crate) fn check_repo_updates(&self) -> HashMap<String, String> {
        tracing::debug!("checking repo updates via alpm version comparison");
        let mut updates = HashMap::new();

        for pkg in self.handle.localdb().pkgs() {
            let local_ver = pkg.version().to_string();
            for db in self.handle.syncdbs() {
                if let Ok(sync_pkg) = db.pkg(pkg.name()) {
                    let sync_ver = sync_pkg.version().to_string();
                    if alpm::vercmp(local_ver.clone(), sync_ver.clone()) == Ordering::Less {
                        tracing::debug!(
                            name = pkg.name(),
                            local = %local_ver,
                            sync = %sync_ver,
                            "repo update available"
                        );
                        updates.insert(pkg.name().to_string(), sync_ver);
                    }
                    break;
                }
            }
        }

        tracing::info!(count = updates.len(), "repo updates found");
        updates
    }

    pub(crate) fn get_installed_packages(&self, repo_updates: &HashMap<String, String>) -> Vec<Package> {
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

            let update_version = repo_updates.get(pkg.name()).cloned();

            let required_by: Vec<String> = pkg.required_by().iter().map(|s| s.to_string()).collect();
            let is_orphan = pkg.reason() == alpm::PackageReason::Depend && required_by.is_empty();

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
                required_by,
                files: Vec::new(),
                build_date: Some(pkg.build_date()),
                install_date: Some(pkg.install_date().unwrap_or(0)),
                matched_files: Vec::new(),
                votes: None,
                popularity: None,
                out_of_date: None,
                first_submitted: None,
                last_modified: None,
                update_version,
                is_orphan,
            });
        }

        tracing::debug!(count = packages.len(), "loaded installed packages");
        packages
    }

    pub(crate) fn get_package_files(&self, name: &str) -> Vec<String> {
        tracing::debug!(name, "loading package files from localdb");
        match self.handle.localdb().pkg(name) {
            Ok(pkg) => {
                let files: Vec<String> = pkg
                    .files()
                    .files()
                    .iter()
                    .map(|f| format!("/{}", String::from_utf8_lossy(f.name())))
                    .collect();
                tracing::debug!(name, count = files.len(), "loaded package files");
                files
            }
            Err(e) => {
                tracing::warn!(name, %e, "failed to load package files");
                Vec::new()
            }
        }
    }

    pub(crate) fn search_package(&self, package_name: &str) -> eyre::Result<Vec<Package>> {
        const MAX_RESULTS: usize = 100;
        let mut packages = Vec::with_capacity(MAX_RESULTS);

        'outer: for db in self.handle.syncdbs() {
            let results = match db.search([package_name].iter()) {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(query = package_name, %e, "invalid search pattern (treated as regex by alpm), returning empty");
                    return Ok(Vec::new());
                }
            };
            for pkg in results {
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
                    required_by: Vec::new(),
                    files: Vec::new(),
                    build_date: Some(pkg.build_date()),
                    install_date: None,
                    matched_files: Vec::new(),
                    votes: None,
                    popularity: None,
                    out_of_date: None,
                    first_submitted: None,
                    last_modified: None,
                    update_version: None,
                    is_orphan: false,
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

pub(crate) fn system_upgrade() -> eyre::Result<ExitStatus> {
    tracing::info!("executing sudo pacman -Syu (system upgrade)");
    let status = Command::new("sudo")
        .arg("pacman")
        .arg("-Syu")
        .status()?;
    tracing::debug!(code = ?status.code(), "pacman -Syu completed");
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
    pub(crate) required_by: Vec<String>,
    pub(crate) files: Vec<String>,
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
    pub(crate) update_version: Option<String>,
    pub(crate) is_orphan: bool,
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

/// Format unix timestamp to human-readable date (YYYY-MM-DD).
/// Uses Howard Hinnant's civil_from_days algorithm for correctness.
pub(crate) fn format_timestamp(ts: i64) -> String {
    let z = ts / 86400 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{:04}-{:02}-{:02}", y, m, d)
}

/// Read repository names from /etc/pacman.conf.
/// Falls back to ["core", "extra"] if the file can't be read.
fn read_repos_from_pacman_conf() -> Vec<String> {
    let content = match std::fs::read_to_string("/etc/pacman.conf") {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(%e, "failed to read /etc/pacman.conf, using default repos");
            return vec!["core".to_string(), "extra".to_string()];
        }
    };

    let mut repos = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            let name = &line[1..line.len() - 1];
            if name != "options" {
                repos.push(name.to_string());
            }
        }
    }

    if repos.is_empty() {
        tracing::warn!("no repositories found in pacman.conf, using default repos");
        return vec!["core".to_string(), "extra".to_string()];
    }

    tracing::info!(count = repos.len(), ?repos, "read repositories from pacman.conf");
    repos
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // format_timestamp
    // -----------------------------------------------------------------------

    #[test]
    fn format_timestamp_unix_epoch() {
        assert_eq!(format_timestamp(0), "1970-01-01");
    }

    #[test]
    fn format_timestamp_known_date() {
        // 2024-01-01 00:00:00 UTC = 1704067200
        assert_eq!(format_timestamp(1704067200), "2024-01-01");
    }

    #[test]
    fn format_timestamp_leap_year_feb_29() {
        // 2024-02-29 00:00:00 UTC = 1709164800
        assert_eq!(format_timestamp(1709164800), "2024-02-29");
    }

    #[test]
    fn format_timestamp_leap_year_mar_1() {
        // 2024-03-01 00:00:00 UTC = 1709251200
        assert_eq!(format_timestamp(1709251200), "2024-03-01");
    }

    #[test]
    fn format_timestamp_non_leap_year_mar_1() {
        // 2023-03-01 00:00:00 UTC = 1677628800
        assert_eq!(format_timestamp(1677628800), "2023-03-01");
    }

    #[test]
    fn format_timestamp_end_of_year() {
        // 2023-12-31 00:00:00 UTC = 1703980800
        assert_eq!(format_timestamp(1703980800), "2023-12-31");
    }

    #[test]
    fn format_timestamp_mid_day() {
        // 2024-06-15 12:30:00 UTC = 1718451000
        // Should still show 2024-06-15 (time is ignored)
        assert_eq!(format_timestamp(1718451000), "2024-06-15");
    }

    #[test]
    fn format_timestamp_y2k() {
        // 2000-01-01 00:00:00 UTC = 946684800
        assert_eq!(format_timestamp(946684800), "2000-01-01");
    }

    #[test]
    fn format_timestamp_century_boundary() {
        // 2100-01-01 00:00:00 UTC = 4102444800 (not a leap year)
        assert_eq!(format_timestamp(4102444800), "2100-01-01");
    }

    // -----------------------------------------------------------------------
    // format_size
    // -----------------------------------------------------------------------

    #[test]
    fn format_size_zero() {
        assert_eq!(format_size(0), "0 B");
    }

    #[test]
    fn format_size_bytes() {
        assert_eq!(format_size(512), "512 B");
    }

    #[test]
    fn format_size_one_kib() {
        assert_eq!(format_size(1024), "1.00 KiB");
    }

    #[test]
    fn format_size_kib() {
        assert_eq!(format_size(5120), "5.00 KiB");
    }

    #[test]
    fn format_size_one_mib() {
        assert_eq!(format_size(1048576), "1.00 MiB");
    }

    #[test]
    fn format_size_mib() {
        assert_eq!(format_size(15_728_640), "15.00 MiB");
    }

    #[test]
    fn format_size_one_gib() {
        assert_eq!(format_size(1_073_741_824), "1.00 GiB");
    }

    #[test]
    fn format_size_fractional_mib() {
        // 1.5 MiB = 1572864 bytes
        assert_eq!(format_size(1_572_864), "1.50 MiB");
    }

    #[test]
    fn format_size_just_under_kib() {
        assert_eq!(format_size(1023), "1023 B");
    }
}
