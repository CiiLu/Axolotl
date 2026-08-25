//! Modpack server installation: materializing an `.mrpack` (files, overrides,
//! and the server launcher) into a managed server directory.
//!
//! The flow mirrors what a modpack client install does, but lands in the
//! dedicated servers folder: the archive is downloaded and unpacked, every file
//! listed in `modrinth.index.json` is fetched to its target path, client and
//! server overrides are applied, and the loader's server launcher jar is placed
//! next to them so the regular `servers.start` flow can boot it.

use std::collections::HashMap;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use futures::stream::{self, StreamExt};
use serde::Deserialize;

use crate::State;
use crate::api::pack::archive_util::{
    extract_archive_subdir, read_archive_entry_to_string,
};
use crate::api::pack::detect::decode_zip_entry_name;
use crate::event::ServerPayloadType;
use crate::event::emit::emit_server;
use crate::event::emit::loading_try_for_each_concurrent;
use crate::util::fetch::{
    DownloadRequest, FetchProgressFn, Integrity, ResourceClass,
    download_to_path,
};
use crate::util::io::IOError;
use crate::{ErrorKind, Result};

use super::files::download_to_dir;
use super::manifest::{
    ModpackInfo, read_manifest, server_path, write_manifest,
};

const MRPACK_MANIFEST_ENTRY: &str = "modrinth.index.json";
const MRPACK_FILENAME: &str = "pack.mrpack";
const OVERRIDES_DIR: &str = "overrides";
const SERVER_OVERRIDES_DIR: &str = "server/overrides";
/// How many modpack files download at once. Bounded by the global download
/// semaphore, which also leaves headroom for concurrent instance installs.
const MODPACK_DOWNLOAD_CONCURRENCY: usize = 8;

/// Shared aggregate progress across concurrent file downloads, rendered as one
/// smooth bar: completed bytes plus the in-flight file's partial bytes, kept
/// monotonic so the UI never regresses.
#[derive(Clone)]
struct AggregateProgress {
    bytes_done: Arc<AtomicU64>,
    reported: Arc<AtomicU64>,
    total: u64,
}

impl AggregateProgress {
    fn new(total: u64) -> Self {
        Self {
            bytes_done: Arc::new(AtomicU64::new(0)),
            reported: Arc::new(AtomicU64::new(0)),
            total,
        }
    }
}

/// `modrinth.index.json` document; only the fields the server installer needs
/// are modeled. `env` marks files that are client-only, server-only, or both.
#[derive(Deserialize)]
struct MrpackIndex {
    #[serde(default)]
    files: Vec<MrpackFile>,
}

#[derive(Deserialize, Clone)]
struct MrpackFile {
    path: String,
    #[serde(default)]
    hashes: HashMap<String, String>,
    #[serde(default)]
    env: Option<MrpackEnv>,
    #[serde(default)]
    downloads: Vec<String>,
    #[serde(default)]
    file_size: Option<u64>,
}

#[derive(Deserialize, Clone)]
struct MrpackEnv {
    #[serde(default)]
    server: Option<String>,
}

/// Installs a modpack into an existing managed server.
///
/// `mrpack_url` / `mrpack_sha1` identify the `.mrpack` archive to download;
/// `jar_url` / `jar_filename` / `jar_sha1` describe the loader's server
/// launcher jar that boots the modpack. The optional `modpack_*` fields are
/// recorded on the manifest so the UI can badge and link the server back to
/// its source project.
#[allow(clippy::too_many_arguments)]
pub async fn install_modpack(
    server_id: &str,
    mrpack_url: &str,
    mrpack_sha1: Option<String>,
    jar_url: &str,
    jar_filename: &str,
    jar_sha1: Option<String>,
    modpack_project_id: Option<String>,
    modpack_version_id: Option<String>,
    modpack_title: Option<String>,
    modpack_icon_url: Option<String>,
) -> Result<()> {
    let dir = server_path(server_id).await?;
    let mut manifest = read_manifest(&dir).await?;
    let state = State::get().await?;

    log(server_id, "Downloading modpack archive").await?;
    download_with_engine(
        server_id,
        &state,
        mrpack_url,
        &dir.join(MRPACK_FILENAME),
        mrpack_sha1,
        ResourceClass::Modpack,
        AggregateProgress::new(0),
    )
    .await?;
    let archive_path = dir.join(MRPACK_FILENAME);

    let manifest_entry = find_manifest_entry(&archive_path).await?;
    let base_folder = base_folder(&manifest_entry);
    let index = parse_index(&archive_path, &manifest_entry).await?;

    let installable_files: Vec<&MrpackFile> = index
        .files
        .iter()
        .filter(|file| is_server_installable(file))
        .collect();
    let total_bytes: u64 = installable_files
        .iter()
        .filter_map(|file| file.file_size)
        .sum();

    log(
        server_id,
        &format!("Downloading {} modpack file(s)", installable_files.len()),
    )
    .await?;

    let files: Vec<(String, String, Option<String>, u64)> = installable_files
        .iter()
        .filter_map(|file| {
            file.downloads.first().map(|url| {
                (
                    file.path.clone(),
                    url.clone(),
                    file.hashes.get("sha1").cloned(),
                    file.file_size.unwrap_or(0),
                )
            })
        })
        .collect();

    let progress = AggregateProgress::new(total_bytes);
    let state_ref = state.clone();
    let dir_ref = dir.clone();
    let server_id_ref = server_id.to_string();
    let num_files = files.len();
    loading_try_for_each_concurrent(
        stream::iter(files).map(Ok::<_, crate::Error>),
        Some(MODPACK_DOWNLOAD_CONCURRENCY),
        None,
        0.0,
        num_files,
        None,
        move |(path, url, sha1, size)| {
            let server_id = server_id_ref.clone();
            let dir = dir_ref.clone();
            let state = state_ref.clone();
            let progress = progress.clone();
            async move {
                log(&server_id, &format!("Downloading {path}")).await?;
                let destination = dir.join(&path);
                if let Some(parent) = destination.parent() {
                    crate::util::io::create_dir_all(parent).await?;
                }
                download_with_engine(
                    &server_id,
                    &state,
                    &url,
                    &destination,
                    sha1,
                    ResourceClass::Modpack,
                    progress.clone(),
                )
                .await?;
                progress.bytes_done.fetch_add(size, Ordering::Relaxed);
                Ok(())
            }
        },
    )
    .await?;

    log(&server_id, "Applying modpack overrides").await?;
    extract_archive_subdir(
        archive_path.clone(),
        format!("{base_folder}{OVERRIDES_DIR}/"),
        dir.clone(),
    )
    .await?;
    extract_archive_subdir(
        archive_path,
        format!("{base_folder}{SERVER_OVERRIDES_DIR}/"),
        dir.clone(),
    )
    .await?;

    log(
        &server_id,
        &format!("Downloading server launcher ({jar_filename})"),
    )
    .await?;
    download_to_dir(&server_id, &dir, jar_url, jar_filename, jar_sha1).await?;

    manifest.jar_name = Some(jar_filename.to_string());
    if let (Some(project_id), Some(version_id), Some(title)) =
        (modpack_project_id, modpack_version_id, modpack_title)
    {
        manifest.modpack = Some(ModpackInfo {
            project_id,
            version_id,
            title,
            icon_url: modpack_icon_url,
        });
    }
    write_manifest(&dir, &manifest).await?;

    Ok(())
}

/// Downloads a single file through the shared launcher download engine
/// (mirrors, retries, integrity, range-segmented multi-connection transfer for
/// large files, background-friendly concurrency) instead of a bespoke HTTP
/// client. Progress is reported as server events through the shared aggregate.
async fn download_with_engine(
    server_id: &str,
    state: &State,
    url: &str,
    destination: &Path,
    sha1: Option<String>,
    resource: ResourceClass,
    progress: AggregateProgress,
) -> Result<()> {
    let mut request = DownloadRequest::new(url, resource);
    if let Some(sha1) = &sha1 {
        request = request.with_integrity(Integrity::sha1(sha1.clone()));
    }
    request = request.with_segmented_download(true);

    let server_id = server_id.to_string();
    let progress = progress.clone();
    let mut progress_fn: Box<FetchProgressFn<'_>> = Box::new(
        move |downloaded: u64,
              file_total: u64|
              -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
            let server_id = server_id.clone();
            let progress = progress.clone();
            Box::pin(async move {
                let current = progress
                    .bytes_done
                    .load(Ordering::Relaxed)
                    .saturating_add(downloaded);
                let shown = progress
                    .reported
                    .fetch_max(current, Ordering::Relaxed)
                    .max(current);
                emit_server(
                    &server_id,
                    ServerPayloadType::DownloadProgress {
                        downloaded: shown,
                        total: if progress.total > 0 {
                            Some(progress.total)
                        } else if file_total > 0 {
                            Some(file_total)
                        } else {
                            None
                        },
                    },
                )
                .await
                .ok();
                Ok(())
            })
        },
    );

    download_to_path(
        request,
        destination,
        &state.download_semaphore,
        &state.pool,
        Some(progress_fn.as_mut()),
    )
    .await?;
    Ok(())
}

fn is_server_installable(file: &MrpackFile) -> bool {
    file.env.as_ref().and_then(|env| env.server.as_deref())
        != Some("unsupported")
}

/// Locates the `modrinth.index.json` entry inside the archive, tolerating packs
/// whose contents are nested under a single base folder.
async fn find_manifest_entry(archive_path: &Path) -> Result<String> {
    let archive_path = archive_path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        let file = std::fs::File::open(&archive_path)
            .map_err(|error| IOError::with_path(error, &archive_path))?;
        let mut archive = zip::ZipArchive::new(file).map_err(|error| {
            ErrorKind::InputError(format!(
                "Modpack archive is invalid: {error}"
            ))
            .as_error()
        })?;
        for index in 0..archive.len() {
            let entry = archive.by_index_raw(index).map_err(|error| {
                ErrorKind::InputError(format!(
                    "Failed to read modpack archive entry: {error}"
                ))
                .as_error()
            })?;
            let name =
                decode_zip_entry_name(entry.name_raw()).replace('\\', "/");
            if name == MRPACK_MANIFEST_ENTRY
                || name.ends_with(&format!("/{MRPACK_MANIFEST_ENTRY}"))
            {
                return Ok(name);
            }
        }
        Err(ErrorKind::InputError(
            "Modpack archive is missing modrinth.index.json".to_string(),
        )
        .as_error())
    })
    .await?
}

fn base_folder(manifest_entry: &str) -> String {
    manifest_entry
        .strip_suffix(MRPACK_MANIFEST_ENTRY)
        .unwrap_or_default()
        .to_string()
}

async fn parse_index(
    archive_path: &Path,
    manifest_entry: &str,
) -> Result<MrpackIndex> {
    let contents = read_archive_entry_to_string(
        archive_path.to_path_buf(),
        manifest_entry.to_string(),
    )
    .await?;
    serde_json::from_str(&contents).map_err(|error| {
        ErrorKind::InputError(format!(
            "Failed to parse modrinth.index.json: {error}"
        ))
        .as_error()
    })
}

async fn log(server_id: &str, line: &str) -> Result<()> {
    emit_server(
        server_id,
        ServerPayloadType::Log {
            line: line.to_string(),
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn server_only_files_are_installable() {
        let installable = MrpackFile {
            path: "mods/a.jar".to_string(),
            hashes: HashMap::new(),
            env: None,
            downloads: vec!["https://example.com/a.jar".to_string()],
            file_size: Some(10),
        };
        assert!(is_server_installable(&installable));

        let optional = MrpackFile {
            env: Some(MrpackEnv {
                server: Some("optional".to_string()),
            }),
            ..installable.clone()
        };
        assert!(is_server_installable(&optional));

        let client_only = MrpackFile {
            env: Some(MrpackEnv {
                server: Some("unsupported".to_string()),
            }),
            ..installable.clone()
        };
        assert!(!is_server_installable(&client_only));
    }

    #[test]
    fn base_folder_derivation() {
        assert_eq!(base_folder("modrinth.index.json"), "");
        assert_eq!(base_folder("my-pack/modrinth.index.json"), "my-pack/");
    }
}
