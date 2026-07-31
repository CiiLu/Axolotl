use std::{
    fmt,
    path::{Path, PathBuf},
};

use futures::stream::{FuturesUnordered, StreamExt};
use io::IOError;
use serde::{Deserialize, Serialize};

use crate::{
    install::{
        InstallPhaseDetails, InstallPhaseId, InstallProgress,
        InstallProgressReporter,
    },
    util::{
        fetch::{self, IoSemaphore},
        io,
    },
};

pub mod atlauncher;
pub mod curseforge;
pub mod gdlauncher;
pub(crate) mod generic;
pub mod hmcl;
pub mod hmcl_config;
mod instance_json;
pub mod mmc;
mod modrinth_app;
mod pcl;
pub use pcl::config_exists;
pub use pcl::read_pcl_registry;
pub mod pe_info;

/// A scanned importable instance with its resolved filesystem path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportableInstance {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImportLauncherType {
    MultiMC,
    PrismLauncher,
    ATLauncher,
    GDLauncher,
    Curseforge,
    ModrinthApp,
    PCL2,
    PCL2CE,
    HMCL,
    Generic,
    #[serde(other)]
    Unknown,
}
// impl display
impl fmt::Display for ImportLauncherType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImportLauncherType::MultiMC => write!(f, "MultiMC"),
            ImportLauncherType::PrismLauncher => write!(f, "PrismLauncher"),
            ImportLauncherType::ATLauncher => write!(f, "ATLauncher"),
            ImportLauncherType::GDLauncher => write!(f, "GDLauncher"),
            ImportLauncherType::Curseforge => write!(f, "Curseforge"),
            ImportLauncherType::ModrinthApp => {
                write!(f, "Modrinth source installation")
            }
            ImportLauncherType::PCL2 => write!(f, "PCL2"),
            ImportLauncherType::PCL2CE => write!(f, "PCL2CE"),
            ImportLauncherType::HMCL => write!(f, "HMCL"),
            ImportLauncherType::Generic => write!(f, "Generic"),
            ImportLauncherType::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Return a list of importable instances from a launcher type and base path,
/// by iterating through the folder and checking each candidate.
pub async fn get_importable_instances(
    launcher_type: ImportLauncherType,
    base_path: PathBuf,
) -> crate::Result<Vec<ImportableInstance>> {
    match launcher_type {
        ImportLauncherType::ModrinthApp => {
            get_modrinth_app_instances(&base_path).await
        }
        ImportLauncherType::GDLauncher | ImportLauncherType::ATLauncher => {
            get_instances_subfolder_scan(&base_path, "instances", launcher_type)
                .await
        }
        ImportLauncherType::Curseforge => {
            get_instances_subfolder_scan(&base_path, "Instances", launcher_type)
                .await
        }
        ImportLauncherType::MultiMC => {
            let subpath = mmc::get_instances_subpath(base_path.join("multimc.cfg"))
                .await
                .unwrap_or_else(|| "instances".to_string());
            get_instances_subfolder_scan(&base_path, &subpath, launcher_type)
                .await
        }
        ImportLauncherType::PrismLauncher => {
            let subpath = mmc::get_instances_subpath(
                base_path.join("prismlauncher.cfg"),
            )
            .await
            .unwrap_or_else(|| "instances".to_string());
            get_instances_subfolder_scan(&base_path, &subpath, launcher_type)
                .await
        }
        ImportLauncherType::PCL2 | ImportLauncherType::PCL2CE => {
            get_pcl_instances(&base_path).await
        }
        ImportLauncherType::HMCL => get_hmcl_instances(&base_path).await,
        ImportLauncherType::Generic => get_generic_instances(&base_path).await,
        ImportLauncherType::Unknown => {
            get_unknown_launcher_instances(&base_path).await
        }
    }
}

/// Scans a launcher's instances subfolder, validating each candidate with the
/// launcher-specific check.
async fn get_instances_subfolder_scan(
    base_path: &Path,
    instances_subfolder: &str,
    launcher_type: ImportLauncherType,
) -> crate::Result<Vec<ImportableInstance>> {
    let instances_folder = base_path.join(instances_subfolder);
    let mut result = Vec::new();
    let mut dir = io::read_dir(&instances_folder).await.map_err(|_| {
        crate::ErrorKind::InputError(format!(
            "Invalid {launcher_type} launcher path, could not find '{instances_subfolder}' subfolder."
        ))
    })?;
    while let Some(entry) = dir
        .next_entry()
        .await
        .map_err(|e| IOError::with_path(e, &instances_folder))?
    {
        let path = entry.path();
        if path.is_dir()
            && is_valid_importable_instance(path.clone(), launcher_type).await
        {
            let name = path.file_name();
            if let Some(name) = name {
                let name = name.to_string_lossy().to_string();
                result.push(ImportableInstance {
                    path: path.to_string_lossy().to_string(),
                    name,
                });
            }
        }
    }
    Ok(result)
}

/// Collects Modrinth App profiles from its internal database.
async fn get_modrinth_app_instances(
    base_path: &Path,
) -> crate::Result<Vec<ImportableInstance>> {
    let names = modrinth_app::get_importable_instances(base_path.to_path_buf())
        .await?;
    Ok(names
        .into_iter()
        .map(|n| ImportableInstance {
            name: n.clone(),
            path: n,
        })
        .collect())
}

/// Collects PCL2 / PCL2CE instances from the registry and CE config,
/// de-duplicating paths that appear in both sources.
async fn get_pcl_instances(base_path: &Path) -> crate::Result<Vec<ImportableInstance>> {
    if !pe_info::folder_has_product(base_path, "Plain Craft Launcher") {
        return Ok(Vec::new());
    }
    let mut collector = InstanceCollector::new();
    // Try both PCL2 registry and PCLCE config — the user may have instances
    // registered in either or both sources.
    if pcl::read_pcl_registry().is_some() {
        collect_launcher_instances(&mut collector, pcl::get_pcl_instances())
            .await;
    }
    if pcl::config_exists() {
        collect_launcher_instances(&mut collector, pcl::get_pclce_instances())
            .await;
    }
    Ok(collector.instances)
}

/// Collects HMCL instances from its launcher config.
async fn get_hmcl_instances(base_path: &Path) -> crate::Result<Vec<ImportableInstance>> {
    if !hmcl::config_exists(base_path) {
        return Ok(Vec::new());
    }
    let mut collector = InstanceCollector::new();
    collect_launcher_instances(&mut collector, hmcl::get_instances(base_path))
        .await;
    Ok(collector.instances)
}

/// Scans a generic launcher folder for instance JSONs.
async fn get_generic_instances(base_path: &Path) -> crate::Result<Vec<ImportableInstance>> {
    Ok(scan_instances_at(base_path, None)
        .await
        .into_iter()
        .map(|(n, p)| ImportableInstance {
            name: n,
            path: p.to_string_lossy().to_string(),
        })
        .collect())
}

/// Probes every known launcher type in a folder and merges all found
/// instances, de-duplicating by resolved path.
async fn get_unknown_launcher_instances(
    base_path: &Path,
) -> crate::Result<Vec<ImportableInstance>> {
    let mut collector = InstanceCollector::new();

    // PCL2
    if pe_info::folder_has_product(base_path, "Plain Craft Launcher")
        && pcl::read_pcl_registry().is_some()
    {
        collect_launcher_instances(&mut collector, pcl::get_pcl_instances())
            .await;
    }

    // PCL2CE
    if pe_info::folder_has_product(base_path, "Plain Craft Launcher")
        && pcl::config_exists()
    {
        collect_launcher_instances(&mut collector, pcl::get_pclce_instances())
            .await;
    }

    // HMCL
    if hmcl::config_exists(base_path) {
        collect_launcher_instances(
            &mut collector,
            hmcl::get_instances(base_path),
        )
        .await;
    }

    // ModrinthApp: uses its internal SQLite database; query real physical
    // profile paths for accurate dedup.
    for (iname, ipath) in
        modrinth_app::get_importable_instances_with_paths(
            base_path.to_path_buf(),
        )
        .await
        .unwrap_or_default()
    {
        collector.push(iname, ipath);
    }

    collect_subfolder_launcher_instances(&mut collector, base_path).await;

    // Generic fallback: scan versions/ subdirectory and base path for
    // instance.json files (handles .minecraft and other unrecognized launchers).
    if collector.instances.is_empty() {
        for (name, path) in scan_instances_at(base_path, None).await {
            collector.instances.push(ImportableInstance {
                name,
                path: path.to_string_lossy().to_string(),
            });
        }
    }

    collector.instances.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(collector.instances)
}

/// Collects instances from launchers with a dedicated instances subfolder
/// (MultiMC, PrismLauncher, ATLauncher, GDLauncher, CurseForge).
async fn collect_subfolder_launcher_instances(
    collector: &mut InstanceCollector,
    base_path: &Path,
) {
    let other_types = [
        (ImportLauncherType::MultiMC, "multimc.cfg"),
        (ImportLauncherType::PrismLauncher, "prismlauncher.cfg"),
        (ImportLauncherType::ATLauncher, "instances"),
        (ImportLauncherType::GDLauncher, "instances"),
        (ImportLauncherType::Curseforge, "Instances"),
    ];
    for (lt, marker) in other_types {
        let subpath = match lt {
            ImportLauncherType::MultiMC | ImportLauncherType::PrismLauncher => {
                mmc::get_instances_subpath(base_path.join(marker))
                    .await
                    .unwrap_or_else(|| "instances".to_string())
            }
            ImportLauncherType::ATLauncher | ImportLauncherType::GDLauncher => {
                "instances".to_string()
            }
            ImportLauncherType::Curseforge => "Instances".to_string(),
            _ => unreachable!(),
        };
        if let Ok(found) =
            get_instances_subfolder_scan(base_path, &subpath, lt).await
        {
            for inst in found {
                collector.push(inst.name, PathBuf::from(inst.path));
            }
        }
    }
}

/// Runs `scan_instances_at` over every source folder and pushes de-duplicated
/// results into the collector.
async fn collect_launcher_instances(
    collector: &mut InstanceCollector,
    sources: Vec<(String, String)>,
) {
    for (name, dir) in sources {
        for (iname, ipath) in
            scan_instances_at(&PathBuf::from(dir), Some(&name)).await
        {
            collector.push(iname, ipath);
        }
    }
}

/// Collects importable instances while de-duplicating by resolved path.
struct InstanceCollector {
    instances: Vec<ImportableInstance>,
    seen: std::collections::HashSet<PathBuf>,
}

impl InstanceCollector {
    fn new() -> Self {
        Self {
            instances: Vec::new(),
            seen: std::collections::HashSet::new(),
        }
    }

    /// Pushes an instance unless a path with the same resolved path was
    /// already collected.
    fn push(&mut self, name: String, path: PathBuf) {
        if self.seen.insert(path.clone()) {
            self.instances.push(ImportableInstance {
                name,
                path: path.to_string_lossy().to_string(),
            });
        }
    }
}

async fn scan_instances_at(
    path: &Path,
    prefix: Option<&str>,
) -> Vec<(String, PathBuf)> {
    if !path.is_dir() {
        return Vec::new();
    }
    let mut instances = Vec::new();
    if instance_json::detect(path).is_some() {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "imported".to_string());
        instances.push((
            if let Some(pre) = prefix {
                format!("{pre}:{name}")
            } else {
                name
            },
            path.to_path_buf(),
        ));
    }
    let versions_dir = path.join("versions");
    if versions_dir.is_dir()
        && let Ok(mut dir) = io::read_dir(&versions_dir).await
    {
        while let Ok(Some(entry)) = dir.next_entry().await {
            if entry.path().is_dir()
                && instance_json::detect(&entry.path()).is_some()
                && let Some(name) = entry.path().file_name()
            {
                let name = name.to_string_lossy().to_string();
                let ipath = entry.path();
                instances.push((
                    if let Some(pre) = prefix {
                        format!("{pre}:versions/{name}")
                    } else {
                        format!("versions/{name}")
                    },
                    ipath,
                ));
            }
        }
    }
    tracing::debug!(
        "scan_instances_at: path={} prefix={:?} found={}",
        path.display(),
        prefix,
        instances.len()
    );
    instances
}

fn resolve_instance_path(base_path: &Path, instance_folder: &str) -> PathBuf {
    if let Some(rest) = instance_folder.strip_prefix("versions/") {
        return base_path.join("versions").join(rest);
    }
    if base_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .as_deref()
        == Some(instance_folder)
    {
        base_path.to_path_buf()
    } else {
        base_path.join(instance_folder)
    }
}

fn split_config_name(name: &str) -> (&str, &str) {
    name.split_once(':').unwrap_or((name, ""))
}

/// Everything needed to import one instance, grouped so launcher-specific
/// importers don't need to thread a dozen parameters through every call.
struct ImportJob {
    instance_id: String,
    base_path: PathBuf,
    instance_folder: String,
    /// Pre-resolved path from frontend scanning (PCL2/PCL2CE only).
    instance_path: Option<String>,
    reporter: InstallProgressReporter,
    symlink: bool,
}

/// Imports an instance from a generic folder with config name resolution.
async fn import_configured_instance(
    job: &ImportJob,
    details: InstallPhaseDetails,
    get_game_dir: impl FnOnce(&str) -> Option<String>,
) -> crate::Result<()> {
    let (config_name, rest) = split_config_name(&job.instance_folder);
    let game_dir = get_game_dir(config_name)
        .map(PathBuf::from)
        .unwrap_or_else(|| job.base_path.clone());
    let target = if rest.is_empty() { config_name } else { rest };
    let path = resolve_instance_path(&game_dir, target);
    generic::import_generic(
        path,
        &job.instance_id,
        job.reporter.clone(),
        details,
        job.symlink,
    )
    .await
}

pub(crate) async fn import_instance_with_reporter(
    instance_id: &str,
    launcher_type: ImportLauncherType,
    base_path: PathBuf,
    instance_folder: String,
    instance_path: Option<String>,
    reporter: InstallProgressReporter,
    symlink: bool,
) -> crate::Result<()> {
    import_instance_inner(
        ImportJob {
            instance_id: instance_id.to_string(),
            base_path,
            instance_folder,
            instance_path,
            reporter,
            symlink,
        },
        launcher_type,
    )
    .await
}

async fn import_instance_inner(
    job: ImportJob,
    launcher_type: ImportLauncherType,
) -> crate::Result<()> {
    let instance_id = job.instance_id.clone();
    tracing::debug!(
        "Importing instance from {} (symlink={}, launcher_type={launcher_type})",
        job.instance_folder,
        job.symlink
    );
    let details = InstallPhaseDetails::Import {
        launcher_type,
        instance_folder: job.instance_folder.clone(),
    };
    let res = if launcher_type == ImportLauncherType::Unknown {
        import_unknown_launcher(job).await
    } else {
        import_via_launcher(launcher_type, &job, details).await
    };

    // If import failed, delete the profile
    match res {
        Ok(_) => {}
        Err(e) => {
            tracing::warn!("Import failed: {:?}", e);
            let _ = crate::api::instance::remove(&instance_id).await;
            return Err(e);
        }
    }

    tracing::debug!("Completed import.");
    Ok(())
}

/// Runs the launcher-specific import step for a known launcher type.
async fn import_via_launcher(
    launcher_type: ImportLauncherType,
    job: &ImportJob,
    details: InstallPhaseDetails,
) -> crate::Result<()> {
    let ImportJob {
        instance_id,
        base_path,
        instance_folder,
        instance_path,
        reporter,
        symlink,
    } = job;

    match launcher_type {
        ImportLauncherType::MultiMC | ImportLauncherType::PrismLauncher => {
            mmc::import_mmc(
                base_path.clone(),       // path to base mmc folder
                instance_folder.clone(), // instance folder in mmc_base_path
                instance_id,
                reporter.clone(),
                details,
                *symlink,
            )
            .await
        }
        ImportLauncherType::ATLauncher => {
            atlauncher::import_atlauncher(
                base_path.clone(),       // path to atlauncher folder
                instance_folder.clone(), // instance folder in atlauncher
                instance_id,
                reporter.clone(),
                details,
                *symlink,
            )
            .await
        }
        ImportLauncherType::GDLauncher => {
            gdlauncher::import_gdlauncher(
                base_path.join("instances").join(instance_folder), // path to gdlauncher folder
                instance_id,
                reporter.clone(),
                details,
                *symlink,
            )
            .await
        }
        ImportLauncherType::Curseforge => {
            curseforge::import_curseforge(
                base_path.join("Instances").join(instance_folder), // path to curseforge folder
                instance_id,
                reporter.clone(),
                details,
                *symlink,
            )
            .await
        }
        ImportLauncherType::ModrinthApp => {
            modrinth_app::import_instance(
                base_path.clone(),
                instance_folder.clone(),
                instance_id,
                reporter.clone(),
                details,
                *symlink,
            )
            .await
        }
        ImportLauncherType::PCL2 | ImportLauncherType::PCL2CE => {
            if let Some(path) = instance_path {
                // Pre-resolved path from frontend scanning — skip re-resolution
                generic::import_generic(
                    PathBuf::from(path),
                    instance_id,
                    reporter.clone(),
                    details,
                    *symlink,
                )
                .await
            } else {
                // Legacy fallback: resolve from config/registry
                import_configured_instance(
                    job,
                    details,
                    |name| {
                        pcl::get_pcl_instance_path(name)
                            .or_else(|| pcl::get_pclce_instance_path(name))
                    },
                )
                .await
            }
        }
        ImportLauncherType::HMCL => {
            import_configured_instance(
                job,
                details,
                |name| hmcl::get_instance_path(&base_path, name),
            )
            .await
        }
        ImportLauncherType::Generic => {
            let path = resolve_instance_path(base_path, instance_folder);
            generic::import_generic(
                path,
                instance_id,
                reporter.clone(),
                details,
                *symlink,
            )
            .await
        }
        ImportLauncherType::Unknown => unreachable!("handled by import_instance_inner"),
    }
}

/// Tries to identify an unknown launcher folder by probing each known
/// launcher type, then dispatches the matching import.
async fn import_unknown_launcher(job: ImportJob) -> crate::Result<()> {
    let types = [
        ImportLauncherType::PCL2,
        ImportLauncherType::PCL2CE,
        ImportLauncherType::HMCL,
        ImportLauncherType::MultiMC,
        ImportLauncherType::PrismLauncher,
        ImportLauncherType::ATLauncher,
        ImportLauncherType::GDLauncher,
        ImportLauncherType::Curseforge,
        ImportLauncherType::ModrinthApp,
    ];
    for lt in types {
        if let Ok(instances) = Box::pin(get_importable_instances(
            lt,
            job.base_path.clone(),
        ))
        .await
            && instances.iter().any(|i| i.name == job.instance_folder)
        {
            let details = InstallPhaseDetails::Import {
                launcher_type: lt,
                instance_folder: job.instance_folder.clone(),
            };
            let mut job = job;
            // Unknown type — let the specific branch resolve the path.
            job.instance_path = None;
            return import_via_launcher(lt, &job, details).await;
        }
    }
    Err(crate::ErrorKind::InputError(
        "Could not determine launcher type for the given path".to_string(),
    )
    .into())
}

/// Returns the default path for the given launcher type
/// None if it can't be found or doesn't exist
pub fn get_default_launcher_path(
    r#type: ImportLauncherType,
) -> Option<PathBuf> {
    let path = match r#type {
        ImportLauncherType::MultiMC => {
            return find_multimc_path();
        }
        ImportLauncherType::PrismLauncher => {
            Some(dirs::data_dir()?.join("PrismLauncher"))
        }
        ImportLauncherType::ATLauncher => {
            Some(dirs::data_dir()?.join("ATLauncher"))
        }
        ImportLauncherType::GDLauncher => {
            Some(dirs::data_dir()?.join("gdlauncher_next"))
        }
        ImportLauncherType::Curseforge => {
            let home = dirs::home_dir()?;
            let primary = home.join("curseforge").join("minecraft");
            if primary.exists() {
                return Some(primary);
            }
            Some(dirs::document_dir()?.join("curseforge").join("minecraft"))
        }
        ImportLauncherType::ModrinthApp => {
            Some(dirs::data_dir()?.join("ModrinthApp"))
        }
        ImportLauncherType::PCL2 => {
            if pcl::read_pcl_registry().is_some() {
                dirs::data_dir()
            } else {
                None
            }
        }
        ImportLauncherType::PCL2CE => {
            if pcl::config_exists() {
                dirs::data_dir()
            } else {
                None
            }
        }
        ImportLauncherType::HMCL => None,
        ImportLauncherType::Generic => None,
        ImportLauncherType::Unknown => None,
    };
    let path = path?;
    if path.exists() { Some(path) } else { None }
}

/// Searches common locations for a MultiMC installation.
/// MultiMC stores data in its own application directory (not a standard data dir)
fn find_multimc_path() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    // Linux/macOS: ~/.local/share/multimc is the typical location
    if let Some(data_dir) = dirs::data_dir() {
        candidates.push(data_dir.join("multimc"));
        candidates.push(data_dir.join("MultiMC"));
    }

    // Windows: check common extraction locations
    #[cfg(target_os = "windows")]
    {
        if let Some(home) = dirs::home_dir() {
            candidates.push(home.join("MultiMC"));
            candidates.push(home.join("Desktop").join("MultiMC"));
            candidates.push(home.join("Downloads").join("MultiMC"));
        }
        candidates.push(PathBuf::from("C:\\MultiMC"));
        if let Some(program_files) =
            std::env::var_os("ProgramFiles").map(PathBuf::from)
        {
            candidates.push(program_files.join("MultiMC"));
        }
        if let Some(program_files_x86) =
            std::env::var_os("ProgramFiles(x86)").map(PathBuf::from)
        {
            candidates.push(program_files_x86.join("MultiMC"));
        }
    }

    // macOS: MultiMC is a .app bundle with data inside MultiMC.app/Data/
    #[cfg(target_os = "macos")]
    {
        candidates.push(PathBuf::from("/Applications/MultiMC.app/Data"));
        if let Some(home) = dirs::home_dir() {
            candidates.push(
                home.join("Applications").join("MultiMC.app").join("Data"),
            );
        }
    }

    candidates
        .into_iter()
        .find(|p| p.join("multimc.cfg").exists())
}

/// Checks if this PathBuf is a valid instance for the given launcher type

#[tracing::instrument]
pub async fn is_valid_importable_instance(
    instance_path: PathBuf,
    r#type: ImportLauncherType,
) -> bool {
    match r#type {
        ImportLauncherType::MultiMC | ImportLauncherType::PrismLauncher => {
            mmc::is_valid_mmc(instance_path).await
        }
        ImportLauncherType::ATLauncher => {
            atlauncher::is_valid_atlauncher(instance_path).await
        }
        ImportLauncherType::GDLauncher => {
            gdlauncher::is_valid_gdlauncher(instance_path).await
        }
        ImportLauncherType::Curseforge => {
            curseforge::is_valid_curseforge(instance_path).await
        }
        ImportLauncherType::ModrinthApp => instance_path.is_dir(),
        ImportLauncherType::PCL2
        | ImportLauncherType::PCL2CE
        | ImportLauncherType::HMCL
        | ImportLauncherType::Generic => instance_path.is_dir(),
        ImportLauncherType::Unknown => false,
    }
}

/// Caches an image file in the filesystem into the cache directory, and returns the path to the cached file.

#[tracing::instrument]
pub async fn recache_icon(
    icon_path: PathBuf,
) -> crate::Result<Option<PathBuf>> {
    let state = crate::State::get().await?;

    let bytes = tokio::fs::read(&icon_path).await;
    if let Ok(bytes) = bytes {
        let bytes = bytes::Bytes::from(bytes);
        let cache_dir = &state.directories.caches_dir();
        let semaphore = &state.io_semaphore;
        Ok(Some(
            fetch::write_cached_icon(
                &icon_path.to_string_lossy(),
                cache_dir,
                bytes,
                semaphore,
            )
            .await?,
        ))
    } else {
        // could not find icon (for instance, prism default icon, etc)
        Ok(None)
    }
}

pub(crate) async fn copy_dotminecraft_with_reporter(
    instance_id: &str,
    dotminecraft: PathBuf,
    io_semaphore: &IoSemaphore,
    reporter: InstallProgressReporter,
    details: InstallPhaseDetails,
) -> crate::Result<()> {
    let instance_path =
        crate::api::instance::get_full_path(instance_id).await?;

    let files = collect_dotminecraft_files(&dotminecraft).await?;

    let total = files.len() as u64;
    if total == 0 {
        reporter
            .update(
                InstallPhaseId::PreparingInstance,
                Some(InstallProgress {
                    current: 0,
                    total: 0,
                    secondary: None,
                }),
                details,
            )
            .await?;
        return Ok(());
    }

    copy_files_with_progress(
        &instance_path,
        files,
        io_semaphore,
        reporter,
        details,
    )
    .await
}

/// Collects all files under `.minecraft`, excluding launcher metadata files
/// at the source root (`<dirname>.json` and `<dirname>.jar`).
async fn collect_dotminecraft_files(
    dotminecraft: &Path,
) -> crate::Result<Vec<(PathBuf, PathBuf)>> {
    // Collect all files recursively
    let files = get_all_subfiles(dotminecraft, false).await?;

    // Filter out launcher metadata files at the source root:
    // <dirname>.json (instance config), <dirname>.jar (custom jar override)
    let dirname = dotminecraft
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let skip_json = format!("{dirname}.json");
    let skip_jar = format!("{dirname}.jar");

    Ok(files
        .into_iter()
        .filter_map(|abs_path| {
            let rel = abs_path.strip_prefix(dotminecraft).ok()?.to_path_buf();
            // Only filter at root level
            if rel.parent().is_some_and(|p| !p.as_os_str().is_empty()) {
                return Some((abs_path, rel));
            }
            let name = rel.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name != skip_json && name != skip_jar {
                Some((abs_path, rel))
            } else {
                None
            }
        })
        .collect())
}

/// Copies the collected files into the instance profile concurrently, bounded
/// by the I/O semaphore, reporting progress after every completed file.
async fn copy_files_with_progress(
    instance_path: &Path,
    files: Vec<(PathBuf, PathBuf)>,
    io_semaphore: &IoSemaphore,
    reporter: InstallProgressReporter,
    details: InstallPhaseDetails,
) -> crate::Result<()> {
    let total = files.len() as u64;

    // Build (src, dst) pairs, then copy concurrently bounded by IoSemaphore
    let mut copy_tasks: FuturesUnordered<_> = files
        .into_iter()
        .map(|(src, rel)| {
            let dst = instance_path.join(&rel);
            (src, dst)
        })
        .map(|(src, dst)| {
            async move {
                // Skip copying if destination file exists and is identical
                if tokio::fs::metadata(&dst).await.is_ok()
                    && let (Ok(src_meta), Ok(dst_meta)) = (
                        tokio::fs::metadata(&src).await,
                        tokio::fs::metadata(&dst).await,
                    )
                {
                    // If files have identical size and modification time, skip copying
                    if src_meta.len() == dst_meta.len()
                        && src_meta.modified().ok() == dst_meta.modified().ok()
                    {
                        return Ok::<_, crate::Error>(());
                    }
                }

                // Proceed with copy
                fetch::copy(&src, &dst, io_semaphore).await?;
                Ok(())
            }
        })
        .collect();

    let mut completed: u64 = 0;
    while let Some(result) = copy_tasks.next().await {
        result?;
        completed += 1;
        reporter
            .update(
                InstallPhaseId::PreparingInstance,
                Some(InstallProgress {
                    current: completed,
                    total,
                    secondary: None,
                }),
                details.clone(),
            )
            .await?;
    }

    // Final 100% report (ensures the bar fills even if reporter throttles the last update)
    reporter
        .update(
            InstallPhaseId::PreparingInstance,
            Some(InstallProgress {
                current: total,
                total,
                secondary: None,
            }),
            details,
        )
        .await?;

    Ok(())
}

pub(crate) async fn finish_import(
    instance_id: &str,
    dotminecraft: PathBuf,
    io_semaphore: &IoSemaphore,
    reporter: InstallProgressReporter,
    details: InstallPhaseDetails,
    symlink: bool,
) -> crate::Result<()> {
    if symlink {
        let instance_path =
            crate::api::instance::get_full_path(instance_id).await?;

        if instance_path.exists() {
            io::remove_dir_all(&instance_path).await?;
        }

        io::create_symlink(&dotminecraft, &instance_path).await?;

        crate::state::edit_instance(
            instance_id,
            crate::state::EditInstance {
                symlink_target: Some(Some(
                    dotminecraft.to_string_lossy().to_string(),
                )),
                ..Default::default()
            },
            &crate::state::State::get().await?.pool,
        )
        .await?;
    } else {
        copy_dotminecraft_with_reporter(
            instance_id,
            dotminecraft,
            io_semaphore,
            reporter.clone(),
            details,
        )
        .await?;
    }

    crate::launcher::install_minecraft_for_instance_id_with_reporter(
        instance_id,
        false,
        Some(reporter),
    )
    .await?;

    Ok(())
}

#[async_recursion::async_recursion]
#[tracing::instrument]
pub async fn get_all_subfiles(
    src: &Path,
    include_empty_dirs: bool,
) -> crate::Result<Vec<PathBuf>> {
    let meta = tokio::fs::symlink_metadata(src)
        .await
        .map_err(|e| IOError::with_path(e, src))?;

    // Symlink / reparse point: never follow, treat as leaf file.
    if crate::util::io::is_symlink_or_reparse(&meta) {
        return Ok(vec![src.to_path_buf()]);
    }

    if meta.is_file() {
        return Ok(vec![src.to_path_buf()]);
    }

    let mut files = Vec::new();
    let mut dir = io::read_dir(&src).await?;

    let mut has_files = false;
    while let Some(child) = dir
        .next_entry()
        .await
        .map_err(|e| IOError::with_path(e, src))?
    {
        has_files = true;
        let src_child = child.path();
        files.append(
            &mut get_all_subfiles(&src_child, include_empty_dirs).await?,
        );
    }

    if !has_files && include_empty_dirs {
        files.push(src.to_path_buf());
    }

    Ok(files)
}
