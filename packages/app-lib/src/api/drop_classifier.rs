//! Unified content-type classifier for dropped / imported files.
//!
//! Determines what kind of Minecraft content a file or folder represents,
//! supporting launcher directories, mod JARs, resource packs, world saves,
//! litematic files, shader packs, and more.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::api::pack::detect::{LocalPackFormat, detect_local_pack_sync};
use crate::api::pack::import::ImportLauncherType;
use crate::mod_metadata::manifest::read_jar_manifest;
use crate::state::{ModrinthProjectId, ModrinthVersionId};

/// Maximum number of items allowed in a ZIP before we classify it as "ZIP
/// with many items" rather than "single file/folder wrapped in ZIP".
const ZIP_TOP_LEVEL_LIMIT: usize = 200;

/// Result of classifying a file path dropped / imported by the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DroppedItemType {
    /// A recognised third-party launcher root folder.
    Launcher {
        launcher_type: ImportLauncherType,
        base_path: PathBuf,
    },
    /// HMCL launcher with separate launcher and data directories.
    HmclLauncher {
        launcher_dir: PathBuf,
        data_dir: PathBuf,
    },
    /// A mod JAR file.
    Mod { file_path: PathBuf },
    /// A `.litematic` or `.schematic` file.
    Litematic { file_path: PathBuf },
    /// A resource pack or data pack.
    ResourcePack { file_path: PathBuf },
    /// A shader pack.
    ShaderPack { file_path: PathBuf },
    /// A Minecraft world save folder or archive.
    WorldSave { file_path: PathBuf },
    /// A shortcut / symlink that was resolved to another item type.
    ShortcutResolved {
        original: PathBuf,
        resolved_to: Box<DroppedItemType>,
    },
    /// A modpack archive (.mrpack, CurseForge, MultiMC, etc.).
    Modpack { file_path: PathBuf },
    /// Could not be classified.
    Unknown { reason: String },
}

/// Classify a dropped file or folder path into a `DroppedItemType`.
///
/// The classification follows a strict priority order: shortcut resolution,
/// ZIP / EXE / JAR detection, then directory and file fallbacks.
/// Returns `Unknown` instead of panicking on any error.
pub fn classify_dropped_item(path: &Path) -> DroppedItemType {
    classify_dropped_item_inner(path, 0)
}

/// Maximum number of shortcut hops followed before giving up. Guards against
/// shortcut / symlink cycles (e.g. `a.lnk` → `b.lnk` → `a.lnk`) that would
/// otherwise recurse until the stack overflows.
const MAX_SHORTCUT_HOPS: u32 = 8;

fn classify_dropped_item_inner(path: &Path, shortcut_depth: u32) -> DroppedItemType {
    if !path.exists() {
        let reason = "Path does not exist".to_string();
        tracing::warn!(
            "Classification failed for '{}': {reason}",
            path.display()
        );
        return DroppedItemType::Unknown { reason };
    }

    if let Some(resolved) =
        crate::util::resolve_shortcut::resolve_shortcut(path, 3)
        && resolved != path
        && shortcut_depth < MAX_SHORTCUT_HOPS
    {
        let inner = classify_dropped_item_inner(&resolved, shortcut_depth + 1);
        return DroppedItemType::ShortcutResolved {
            original: path.to_path_buf(),
            resolved_to: Box::new(inner),
        };
    }

    if is_zip_path(path) {
        return classify_zip_path(path);
    }

    if let Some(ext) = path.extension()
        && ext.eq_ignore_ascii_case("exe")
    {
        return classify_launcher_exe(path);
    }

    if let Some(ext) = path.extension()
        && ext.eq_ignore_ascii_case("disabled")
    {
        return classify_disabled(path);
    }

    if let Some(ext) = path.extension()
        && ext.eq_ignore_ascii_case("jar")
    {
        return classify_jar(path);
    }

    // Step 5: Directory.
    if path.is_dir() {
        let result = classify_folder(path);
        tracing::debug!(
            "classify_dropped_item: directory path={} result={:?}",
            path.display(),
            result
        );
        return result;
    }

    let result = classify_file(path);
    tracing::debug!(
        "classify_dropped_item: file path={} result={:?}",
        path.display(),
        result
    );
    result
}

/// Returns true when the path points to a ZIP-family archive (.zip / .mrpack).
fn is_zip_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            ext.eq_ignore_ascii_case("zip") || ext.eq_ignore_ascii_case("mrpack")
        })
}

/// Classify a ZIP archive, checking known modpack formats before falling back
/// to entry-name probing.
fn classify_zip_path(path: &Path) -> DroppedItemType {
    // NOTE: .jar is deliberately excluded here — JAR files are handled by
    // manifest-based classification to properly distinguish mod JARs from
    // launcher JARs without going through extraction.
    if let Ok(detected) = detect_local_pack_sync(path) {
        match detected.format {
            LocalPackFormat::Mrpack
            | LocalPackFormat::CurseForge
            | LocalPackFormat::Mcbbs
            | LocalPackFormat::Hmcl
            | LocalPackFormat::MmcExport
            | LocalPackFormat::LauncherBundled => {
                return DroppedItemType::Modpack {
                    file_path: path.to_path_buf(),
                };
            }
            _ => {} // PlainArchive / InstanceFolder → entry-name probing
        }
    }
    classify_zip(path)
}

/// Classify a `.disabled` file by treating it as the underlying file type
/// (e.g. `mod.jar.disabled` → Mod, `pack.zip.disabled` → ZIP). The original
/// path is kept in the result — no path rewrite happens.
fn classify_disabled(path: &Path) -> DroppedItemType {
    let Some(stem) = path.file_stem() else {
        return classify_file(path);
    };
    let Some(stem_str) = stem.to_str() else {
        return classify_file(path);
    };
    let Some(underlying_ext) = stem_str.rsplit('.').next() else {
        return classify_file(path);
    };

    if underlying_ext.eq_ignore_ascii_case("jar") {
        // classify_jar reads the original file content (still valid).
        return classify_jar(path);
    }
    if underlying_ext.eq_ignore_ascii_case("zip")
        || underlying_ext.eq_ignore_ascii_case("mrpack")
    {
        // The file content is still a valid archive, so the ZIP pipeline runs
        // on the original path.
        return classify_zip_path(path);
    }

    // Other .disabled extensions fall through to file classification.
    classify_file(path)
}

// ─── ZIP archive classification ─────────────────────────────────────────────

/// Snapshot of a ZIP archive's top-level layout, gathered without extraction.
struct ZipListing {
    /// Distinct top-level entry names.
    top_level: Vec<ZipEntryKind>,
    /// Whether a `level.dat` entry exists anywhere in the archive.
    probe_has_level_dat: bool,
    /// Whether a `pack.mcmeta` entry exists anywhere in the archive.
    probe_has_pack_mcmeta: bool,
    /// Whether a `shaders/` entry exists anywhere in the archive.
    probe_has_shaders_dir: bool,
    /// True when the archive exceeds `ZIP_TOP_LEVEL_LIMIT` top-level entries.
    too_many: bool,
}

enum ZipEntryKind {
    RootFile(String),
    SubFile(String),
}

impl ZipEntryKind {
    fn name(&self) -> &str {
        match self {
            ZipEntryKind::RootFile(n) | ZipEntryKind::SubFile(n) => n,
        }
    }
}

/// Open a ZIP archive and collect its top-level layout and content markers
/// from entry names alone (no extraction).
fn read_zip_listing(path: &Path) -> Result<ZipListing, String> {
    let file = std::fs::File::open(path)
        .map_err(|_| "Cannot open ZIP file".to_string())?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|_| "File is not a valid ZIP archive".to_string())?;

    let mut listing = ZipListing {
        top_level: Vec::new(),
        probe_has_level_dat: false,
        probe_has_pack_mcmeta: false,
        probe_has_shaders_dir: false,
        too_many: false,
    };

    for i in 0..archive.len() {
        let Ok(entry) = archive.by_index_raw(i) else {
            continue;
        };

        let name = entry.name().to_string();
        if name.is_empty() || name.ends_with('/') {
            continue; // skip directory entries
        }

        // Probe known content markers (entry name only, no file content).
        // Markers can live at any depth: zipping a world or resource pack
        // folder itself nests `level.dat` / `pack.mcmeta` under the folder
        // name (e.g. "My World/level.dat").
        let file_name = name.rsplit('/').next().unwrap_or(&name);
        if file_name == "level.dat" {
            listing.probe_has_level_dat = true;
        }
        if file_name == "pack.mcmeta" {
            listing.probe_has_pack_mcmeta = true;
        }
        if name.split('/').any(|segment| segment == "shaders") {
            listing.probe_has_shaders_dir = true;
        }

        // Record the top-level component of the path.
        let top = match name.split_once('/') {
            Some((first, _)) => first,
            None => &name,
        };
        if listing.top_level.iter().any(|k| k.name() == top) {
            continue;
        }
        if listing.top_level.len() >= ZIP_TOP_LEVEL_LIMIT {
            listing.too_many = true;
            continue;
        }
        listing.top_level.push(if name.contains('/') {
            ZipEntryKind::SubFile(top.to_string())
        } else {
            ZipEntryKind::RootFile(top.to_string())
        });
    }

    Ok(listing)
}

/// Classify a ZIP archive from entry names alone.
///
/// Known content markers are checked before the top-level entry limit, so a
/// large archive with an obvious marker (e.g. `level.dat` at the root) is
/// still classified correctly.
fn classify_zip(path: &Path) -> DroppedItemType {
    let listing = match read_zip_listing(path) {
        Ok(listing) => listing,
        Err(reason) => return DroppedItemType::Unknown { reason },
    };
    classify_zip_listing(&listing, path)
}

/// Decide the content type of a ZIP from its `ZipListing`.
fn classify_zip_listing(listing: &ZipListing, path: &Path) -> DroppedItemType {
    // Probe pass: return early when entry names alone are sufficient.
    // Priority order mirrors classify_folder_content.
    if listing.probe_has_level_dat {
        tracing::debug!(
            "ZIP probe hit: level.dat → WorldSave — {}",
            path.display()
        );
        return DroppedItemType::WorldSave {
            file_path: path.to_path_buf(),
        };
    }
    if listing.probe_has_pack_mcmeta {
        tracing::debug!(
            "ZIP probe hit: pack.mcmeta → ResourcePack — {}",
            path.display()
        );
        return DroppedItemType::ResourcePack {
            file_path: path.to_path_buf(),
        };
    }
    if listing.probe_has_shaders_dir {
        tracing::debug!(
            "ZIP probe hit: shaders/ → ShaderPack — {}",
            path.display()
        );
        return DroppedItemType::ShaderPack {
            file_path: path.to_path_buf(),
        };
    }
    // NOTE: versions/<id>/<id>.json is NOT an early-return here — extraction
    // lets classify_folder_content run the root .jar + .json scan for modded
    // instance detection.

    // Guard against huge archives only after the probe pass.
    if listing.too_many {
        return DroppedItemType::Unknown {
            reason: "ZIP archive has too many top-level entries".to_string(),
        };
    }
    if listing.top_level.is_empty() {
        return DroppedItemType::Unknown {
            reason: "Empty zip file".to_string(),
        };
    }

    // Force-analysis fallback: extraction + re-classification is a potentially
    // long operation and should not happen silently during classification.
    // Files that can't be identified from entry names alone should be handled
    // by the frontend (user prompt) via classify_zip_with_extraction().
    tracing::debug!(
        "ZIP probe inconclusive — extraction required for: {}",
        path.display()
    );
    DroppedItemType::Unknown {
        reason: "ZIP archive requires extraction to determine content type"
            .to_string(),
    }
}

/// Extracts a ZIP archive to a temporary directory and classifies its contents
/// by examining the extracted files and folders.
///
/// This is a potentially **long-running** operation — the caller MUST first
/// confirm with the user before calling this function.
pub fn classify_zip_with_extraction(path: &Path) -> DroppedItemType {
    let listing = match read_zip_listing(path) {
        Ok(listing) => listing,
        Err(reason) => return DroppedItemType::Unknown { reason },
    };
    if listing.too_many {
        return DroppedItemType::Unknown {
            reason: "ZIP archive has too many top-level entries".to_string(),
        };
    }

    let Ok(file) = std::fs::File::open(path) else {
        return DroppedItemType::Unknown {
            reason: "Cannot open ZIP file".to_string(),
        };
    };
    let Ok(mut archive) = zip::ZipArchive::new(file) else {
        return DroppedItemType::Unknown {
            reason: "File is not a valid ZIP archive".to_string(),
        };
    };

    // Create temporary directory for extraction.
    let temp_dir = match tempfile::tempdir() {
        Ok(d) => d,
        Err(e) => {
            return DroppedItemType::Unknown {
                reason: format!("Failed to create temporary directory: {e}"),
            };
        }
    };

    // Extract everything.
    extract_all(&mut archive, temp_dir.path());

    tracing::debug!(
        "classify_zip_with_extraction: extracted {} top-level items for {}",
        listing.top_level.len(),
        path.display()
    );

    // Classify the extracted contents.
    if listing.top_level.len() == 1 {
        // Single top-level item — classify it directly.
        let item_name = listing.top_level[0].name().to_string();
        let item_path = temp_dir.path().join(&item_name);
        classify_dropped_item(&item_path)
    } else {
        // Multiple items — classify as a folder.
        classify_folder_content(temp_dir.path())
    }
    // temp_dir is dropped here, cleaning up the extracted files automatically.
}

fn extract_all(archive: &mut zip::ZipArchive<std::fs::File>, base_dir: &Path) {
    // First pass: collect entry metadata while the archive is mutable-borrowed.
    let entries: Vec<(String, bool)> = (0..archive.len())
        .filter_map(|i| {
            let entry = archive.by_index_raw(i).ok()?;
            let name = entry.name().to_string();
            if name.is_empty() {
                None
            } else {
                Some((name.clone(), name.ends_with('/')))
            }
        })
        .collect();

    // Second pass: extract. The collect() above has released the mutable
    // borrow, so we can call by_name() here.
    for (name, is_dir) in &entries {
        // Reject entries that would escape the extraction directory.
        let Some(safe_name) = sanitize_zip_entry_name(name) else {
            tracing::warn!(
                "extract_all: skipping unsafe ZIP entry '{}' (path traversal)",
                name
            );
            continue;
        };
        let out_path = base_dir.join(&safe_name);
        if *is_dir {
            let _ = std::fs::create_dir_all(&out_path);
        } else if let Some(parent) = out_path.parent() {
            let _ = std::fs::create_dir_all(parent);
            if let Ok(mut reader) = archive.by_name(name)
                && let Ok(mut writer) = std::fs::File::create(&out_path)
            {
                let _ = std::io::copy(&mut reader, &mut writer);
            }
        }
    }
}

/// Normalize a ZIP entry name into a safe relative path that stays inside the
/// extraction directory. Returns `None` for absolute paths or entries
/// containing `..` (zip-slip protection).
fn sanitize_zip_entry_name(name: &str) -> Option<PathBuf> {
    // The ZIP spec mandates `/` separators, but tolerate backslashes from
    // Windows-authored archives.
    let normalized = name.replace('\\', "/");
    let path = Path::new(&normalized);
    if path.is_absolute() {
        return None;
    }
    let mut safe = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Normal(part) => safe.push(part),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir
            | std::path::Component::RootDir
            | std::path::Component::Prefix(_) => return None,
        }
    }
    Some(safe)
}

// ─── Step 3: Launcher EXE ──────────────────────────────────────────────────

fn classify_launcher_exe(path: &Path) -> DroppedItemType {
    if let Some(parent) = path.parent() {
        match crate::api::pack::import::pe_info::folder_has_product_result(
            parent,
            "Plain Craft Launcher",
        ) {
            Ok(true) => {
                if crate::api::pack::import::config_exists() {
                    return DroppedItemType::Launcher {
                        launcher_type: ImportLauncherType::PCL2CE,
                        base_path: parent.to_path_buf(),
                    };
                }
                if crate::api::pack::import::read_pcl_registry().is_some() {
                    return DroppedItemType::Launcher {
                        launcher_type: ImportLauncherType::PCL2,
                        base_path: parent.to_path_buf(),
                    };
                }
                return DroppedItemType::Launcher {
                    launcher_type: ImportLauncherType::PCL2,
                    base_path: parent.to_path_buf(),
                };
            }
            Ok(false) => {}
            Err(_) => {}
        }

        match crate::api::pack::import::pe_info::folder_has_product_result(
            parent,
            "Hello Minecraft! Launcher",
        ) {
            Ok(true) => {
                return DroppedItemType::Launcher {
                    launcher_type: ImportLauncherType::HMCL,
                    base_path: parent.to_path_buf(),
                };
            }
            Ok(false) => {}
            Err(_) => {}
        }
    }

    DroppedItemType::Unknown {
        reason: format!("Unrecognised executable: {}", path.display()),
    }
}

// ─── Step 4: JAR file ──────────────────────────────────────────────────────

fn classify_jar(path: &Path) -> DroppedItemType {
    let manifest = read_jar_manifest(path);

    if let Some(ref mf) = manifest {
        // HMCL launcher JAR.
        if mf.main_class.as_deref() == Some("org.jackhuang.hmcl.Main") {
            if let Some(parent) = path.parent()
                && let Some(data_dir) =
                    crate::api::pack::import::hmcl_config::find_hmcl_data_dir(
                        parent,
                    )
            {
                return DroppedItemType::HmclLauncher {
                    launcher_dir: parent.to_path_buf(),
                    data_dir,
                };
            }
            // Found HMCL main class but no data dir — still classify as launcher.
            return DroppedItemType::Launcher {
                launcher_type: ImportLauncherType::HMCL,
                base_path: path
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_default(),
            };
        }
    }

    // Otherwise, treat as a mod.
    DroppedItemType::Mod {
        file_path: path.to_path_buf(),
    }
}

// ─── Step 5: Folder classification ─────────────────────────────────────────

fn classify_folder(path: &Path) -> DroppedItemType {
    // Check launcher signatures in priority order.
    if path.join("multimc.cfg").exists() {
        return DroppedItemType::Launcher {
            launcher_type: ImportLauncherType::MultiMC,
            base_path: path.to_path_buf(),
        };
    }

    if path.join("prismlauncher.cfg").exists() {
        return DroppedItemType::Launcher {
            launcher_type: ImportLauncherType::PrismLauncher,
            base_path: path.to_path_buf(),
        };
    }

    // MultiMC/Prism: check for instances/<sub>/instance.cfg pattern.
    if let Ok(mut dir) = std::fs::read_dir(path.join("instances"))
        && dir.any(|e| {
            e.ok()
                .as_ref()
                .is_some_and(|e| e.path().join("instance.cfg").exists())
        })
    {
        // instance.cfg → MultiMC or Prism.
        return DroppedItemType::Launcher {
            launcher_type: ImportLauncherType::MultiMC,
            base_path: path.to_path_buf(),
        };
    }

    // HMCL portable mode.
    let hmcl_config = path
        .join(".hmcl")
        .join("config")
        .join("launcher-settings.json");
    if hmcl_config.exists()
        && let Some(data_dir) =
            crate::api::pack::import::hmcl_config::find_hmcl_data_dir(path)
    {
        return DroppedItemType::HmclLauncher {
            launcher_dir: path.to_path_buf(),
            data_dir,
        };
    }

    // Step 7: Content-type detection for folders.
    classify_folder_content(path)
}

// ─── Step 6: File classification (non-JAR, non-EXE, non-ZIP) ───────────────

fn classify_file(path: &Path) -> DroppedItemType {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "litematic" | "schematic" => DroppedItemType::Litematic {
            file_path: path.to_path_buf(),
        },
        _ => DroppedItemType::Unknown {
            reason: format!("Unrecognised file type: {}", path.display()),
        },
    }
}

// ─── Step 7: Content-type detection for folders/extracted ZIPs ─────────────

pub(crate) fn classify_folder_content(path: &Path) -> DroppedItemType {
    if let Some(result) = classify_world_save_folder(path) {
        return result;
    }
    if let Some(result) = classify_resource_pack_folder(path) {
        return result;
    }
    if let Some(result) = classify_shader_pack_folder(path) {
        return result;
    }
    if is_launcher_instance_folder(path) {
        return DroppedItemType::Launcher {
            launcher_type: ImportLauncherType::Generic,
            base_path: path.to_path_buf(),
        };
    }

    DroppedItemType::Unknown {
        reason: format!(
            "Unrecognised content: {}",
            path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string_lossy().to_string())
        ),
    }
}

/// Classify a folder as a world save when it contains a `level.dat` file.
fn classify_world_save_folder(path: &Path) -> Option<DroppedItemType> {
    path.join("level.dat")
        .exists()
        .then(|| DroppedItemType::WorldSave {
            file_path: path.to_path_buf(),
        })
}

/// Classify a folder as a resource pack when it contains a `pack.mcmeta`.
fn classify_resource_pack_folder(path: &Path) -> Option<DroppedItemType> {
    path.join("pack.mcmeta")
        .exists()
        .then(|| DroppedItemType::ResourcePack {
            file_path: path.to_path_buf(),
        })
}

/// Classify a folder as a shader pack when it contains a `shaders/` directory.
fn classify_shader_pack_folder(path: &Path) -> Option<DroppedItemType> {
    path.join("shaders")
        .is_dir()
        .then(|| DroppedItemType::ShaderPack {
            file_path: path.to_path_buf(),
        })
}

/// Detect launcher instance markers:
/// - `versions/<id>/<id>.json` pattern (vanilla launcher instance)
/// - a root `.jar` + `.json` pair (modded instance)
/// - `.jar` files in `mods/` (bare instance folder)
fn is_launcher_instance_folder(path: &Path) -> bool {
    has_version_json(path)
        || (has_root_jar(path) && has_root_json(path))
        || has_mods_jar(path)
}

/// Whether `versions/<id>/<id>.json` exists for any subdirectory of `versions/`.
fn has_version_json(path: &Path) -> bool {
    let versions_dir = path.join("versions");
    if !versions_dir.is_dir() {
        return false;
    }
    match std::fs::read_dir(&versions_dir) {
        Ok(mut dir) => dir.any(|e| {
            e.ok().is_some_and(|entry| {
                let p = entry.path();
                let Some(id) = p.file_name().and_then(|n| n.to_str()) else {
                    return false;
                };
                let json_path = p.join(format!("{id}.json"));
                let exists = p.is_dir() && json_path.exists();
                tracing::debug!(
                    "classify_folder_content: versions subdir={} json={} exists={}",
                    id,
                    json_path.display(),
                    exists
                );
                exists
            })
        }),
        Err(e) => {
            tracing::debug!(
                "classify_folder_content: versions_dir={} read_dir_err={}",
                versions_dir.display(),
                e
            );
            false
        }
    }
}

/// Whether the directory contains a file with the given extension.
fn dir_has_extension(dir: &Path, extension: &str, label: &str) -> bool {
    match std::fs::read_dir(dir) {
        Ok(entries) => entries.flatten().any(|entry| {
            let p = entry.path();
            if !p.is_file() {
                return false;
            }
            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            let is_match = ext.eq_ignore_ascii_case(extension);
            tracing::debug!(
                "classify_folder_content: {label} path={} file={} ext={} is_match={}",
                dir.display(),
                p.display(),
                ext,
                is_match
            );
            is_match
        }),
        Err(e) => {
            tracing::debug!(
                "classify_folder_content: {label} path={} read_dir_err={}",
                dir.display(),
                e
            );
            false
        }
    }
}

fn has_root_jar(path: &Path) -> bool {
    dir_has_extension(path, "jar", "root_jar_check")
}

fn has_root_json(path: &Path) -> bool {
    dir_has_extension(path, "json", "root_json_check")
}

/// Whether `mods/` contains at least one `.jar` file.
fn has_mods_jar(path: &Path) -> bool {
    dir_has_extension(&path.join("mods"), "jar", "mods_jar_check")
}

// ─── HMCL data directory discovery ─────────────────────────────────────────

/// Result of looking up a mod file hash on Modrinth.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthLookupResult {
    pub hash: String,
    pub project_id: String,
    pub version_id: String,
    pub project_name: Option<String>,
    pub project_slug: Option<String>,
    pub version_number: Option<String>,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
}

/// Look up a mod file by SHA1 hash to find matching Modrinth project and version.
///
/// Computes the SHA1 hash of the given file and queries the Modrinth API
/// to find matching versions. Returns project and version information if found.
pub async fn lookup_mod_hash(
    path: &Path,
) -> crate::Result<Option<ModrinthLookupResult>> {
    let (_, hash) = crate::util::fetch::sha1_file_async(path).await?;

    let state = crate::State::get().await?;

    let files = crate::state::CachedEntry::get_file_many(
        &[&hash],
        Some(crate::state::CacheBehaviour::StaleWhileRevalidateSkipOffline),
        &state.pool,
        &state.api_semaphore,
    )
    .await?;

    if files.is_empty() {
        return Ok(None);
    }

    let file = &files[0];
    let version = crate::state::CachedEntry::get_version(
        &ModrinthVersionId::new(file.version_id.clone())?,
        Some(crate::state::CacheBehaviour::StaleWhileRevalidateSkipOffline),
        &state.pool,
        &state.api_semaphore,
    )
    .await?;

    let project = if let Some(v) = &version {
        crate::state::CachedEntry::get_project(
            &ModrinthProjectId::new(v.project_id.clone())?,
            Some(crate::state::CacheBehaviour::StaleWhileRevalidateSkipOffline),
            &state.pool,
            &state.api_semaphore,
        )
        .await?
    } else {
        None
    };

    Ok(Some(ModrinthLookupResult {
        hash,
        project_id: file.project_id.clone(),
        version_id: file.version_id.clone(),
        project_name: project.as_ref().map(|p| p.title.clone()),
        project_slug: project.as_ref().and_then(|p| p.slug.clone()),
        version_number: version.as_ref().map(|v| v.version_number.clone()),
        game_versions: version
            .as_ref()
            .map(|v| v.game_versions.clone())
            .unwrap_or_default(),
        loaders: version
            .as_ref()
            .map(|v| v.loaders.clone())
            .unwrap_or_default(),
    }))
}

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_nonexistent_path() {
        let result = classify_dropped_item(Path::new("/nonexistent/path"));
        assert!(
            matches!(result, DroppedItemType::Unknown { .. }),
            "nonexistent path should be Unknown"
        );
    }

    #[test]
    fn test_regular_mod_jar() {
        let dir = tempdir().expect("temp dir");
        let jar_path = dir.path().join("testmod.jar");

        // Create a minimal ZIP with a fabric.mod.json.
        let file = std::fs::File::create(&jar_path).expect("create jar");
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file(
            "fabric.mod.json",
            zip::write::FileOptions::<()>::default(),
        )
        .expect("start entry");
        zip.write_all(b"{}").expect("write");
        zip.finish().expect("finish");

        let result = classify_dropped_item(&jar_path);
        assert!(
            matches!(result, DroppedItemType::Mod { .. }),
            "jar with fabric mod should be classified as Mod: {result:?}"
        );
    }

    #[test]
    fn test_litematic_file() {
        let dir = tempdir().expect("temp dir");
        let path = dir.path().join("build.litematic");
        std::fs::write(&path, "fake litematic data").expect("write");

        let result = classify_dropped_item(&path);
        assert!(
            matches!(result, DroppedItemType::Litematic { .. }),
            "litematic file should be classified as Litematic"
        );
    }

    #[test]
    fn test_resource_pack_folder() {
        let dir = tempdir().expect("temp dir");
        let rp = dir.path().join("my_resource_pack");
        std::fs::create_dir(&rp).expect("create dir");
        std::fs::write(rp.join("pack.mcmeta"), "{}")
            .expect("write pack.mcmeta");

        let result = classify_dropped_item(&rp);
        assert!(
            matches!(result, DroppedItemType::ResourcePack { .. }),
            "folder with pack.mcmeta should be ResourcePack"
        );
    }

    #[test]
    fn test_world_save() {
        let dir = tempdir().expect("temp dir");
        let world = dir.path().join("New World");
        std::fs::create_dir(&world).expect("create dir");
        std::fs::write(world.join("level.dat"), "fake")
            .expect("write level.dat");

        let result = classify_dropped_item(&world);
        assert!(
            matches!(result, DroppedItemType::WorldSave { .. }),
            "folder with level.dat should be WorldSave"
        );
    }

    #[test]
    fn test_multimc_launcher_folder() {
        let dir = tempdir().expect("temp dir");
        std::fs::write(dir.path().join("multimc.cfg"), "").expect("write");

        let result = classify_dropped_item(dir.path());
        assert!(
            matches!(result, DroppedItemType::Launcher { launcher_type, .. } if launcher_type == ImportLauncherType::MultiMC),
            "folder with multimc.cfg should be MultiMC launcher"
        );
    }

    #[test]
    fn test_shader_pack_folder() {
        let dir = tempdir().expect("temp dir");
        let shaders = dir.path().join("shaders");
        std::fs::create_dir(&shaders).expect("create shaders dir");

        let result = classify_dropped_item(dir.path());
        assert!(
            matches!(result, DroppedItemType::ShaderPack { .. }),
            "folder with shaders/ should be ShaderPack"
        );
    }

    #[test]
    fn test_unknown_file() {
        let dir = tempdir().expect("temp dir");
        let path = dir.path().join("random.xyz");
        std::fs::write(&path, "data").expect("write");

        let result = classify_dropped_item(&path);
        assert!(
            matches!(result, DroppedItemType::Unknown { .. }),
            "unknown extension should be Unknown"
        );
    }

    #[test]
    fn test_zip_single_file() {
        let dir = tempdir().expect("temp dir");
        let zip_path = dir.path().join("test.zip");

        let file = std::fs::File::create(&zip_path).expect("create zip");
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("test.txt", zip::write::FileOptions::<()>::default())
            .expect("start entry");
        zip.write_all(b"hello").expect("write");
        zip.finish().expect("finish");

        let result = classify_dropped_item(&zip_path);
        // Single .txt file inside ZIP → after extraction, classify_file sees .txt → Unknown.
        assert!(
            matches!(result, DroppedItemType::Unknown { .. }),
            "zip with single .txt should resolve to Unknown: {result:?}"
        );
    }

    #[test]
    fn test_zip_with_mod_jar() {
        let dir = tempdir().expect("temp dir");
        let zip_path = dir.path().join("modpack.zip");

        let file = std::fs::File::create(&zip_path).expect("create zip");
        let mut zip = zip::ZipWriter::new(file);

        // Create a JAR at the archive root.
        zip.start_file(
            "testmod.jar",
            zip::write::FileOptions::<()>::default(),
        )
        .expect("start entry");
        // The extracted .jar has no readable manifest, so classify_jar falls
        // back to Mod (the default for JAR files).
        zip.write_all(b"fake jar content").expect("write");
        zip.finish().expect("finish");

        let result = classify_zip_with_extraction(&zip_path);
        // Force-analysis extracts the single item and classifies it as a Mod.
        assert!(
            matches!(result, DroppedItemType::Mod { .. }),
            "zip with a single testmod.jar should be classified as Mod: {result:?}"
        );
    }

    #[test]
    fn test_zip_nested_world_save() {
        let dir = tempdir().expect("temp dir");
        let zip_path = dir.path().join("world.zip");

        let file = std::fs::File::create(&zip_path).expect("create zip");
        let mut zip = zip::ZipWriter::new(file);
        // Entries under a single shared root folder, as produced by zipping
        // the world folder itself.
        zip.start_file(
            "My World/level.dat",
            zip::write::FileOptions::<()>::default(),
        )
        .expect("start entry");
        zip.write_all(b"fake").expect("write");
        zip.start_file(
            "My World/region/r.0.0.mca",
            zip::write::FileOptions::<()>::default(),
        )
        .expect("start entry");
        zip.write_all(b"mca").expect("write");
        zip.finish().expect("finish");

        let result = classify_dropped_item(&zip_path);
        assert!(
            matches!(result, DroppedItemType::WorldSave { .. }),
            "zip with a nested level.dat should be classified as WorldSave: {result:?}"
        );
    }

    #[test]
    fn test_zip_nested_resource_pack() {
        let dir = tempdir().expect("temp dir");
        let zip_path = dir.path().join("pack.zip");

        let file = std::fs::File::create(&zip_path).expect("create zip");
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file(
            "My Pack/pack.mcmeta",
            zip::write::FileOptions::<()>::default(),
        )
        .expect("start entry");
        zip.write_all(b"{}").expect("write");
        zip.finish().expect("finish");

        let result = classify_dropped_item(&zip_path);
        assert!(
            matches!(result, DroppedItemType::ResourcePack { .. }),
            "zip with a nested pack.mcmeta should be classified as ResourcePack: {result:?}"
        );
    }

    #[test]
    fn test_extract_all_rejects_path_traversal() {
        let dir = tempdir().expect("temp dir");
        let zip_path = dir.path().join("evil.zip");

        let file = std::fs::File::create(&zip_path).expect("create zip");
        let mut zip = zip::ZipWriter::new(file);
        // A malicious entry that tries to escape the extraction directory.
        zip.start_file(
            "../../evil.txt",
            zip::write::FileOptions::<()>::default(),
        )
        .expect("start entry");
        zip.write_all(b"pwned").expect("write");
        zip.finish().expect("finish");

        let out_dir = tempdir().expect("temp dir");
        let Ok(mut archive) = zip::ZipArchive::new(
            std::fs::File::open(&zip_path).expect("open zip"),
        ) else {
            panic!("zip should open");
        };
        extract_all(&mut archive, out_dir.path());

        assert!(
            !out_dir.path().join("evil.txt").exists()
                && !out_dir.path().join("..").join("evil.txt").exists(),
            "path traversal entry must not be extracted outside the target dir"
        );
    }
}
