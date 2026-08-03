use crate::State;
use crate::state::instances::adapters::{filesystem, sqlite};
use crate::state::instances::{Instance, InstanceFile};
use crate::state::{
    CachedEntry, ContentProvider, ContentProviderRef, ProjectType,
};
use crate::util::fetch;
use chrono::Utc;
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

pub(crate) async fn sync_content_files(
    instance_id: &str,
    state: &State,
) -> crate::Result<Vec<InstanceFile>> {
    let instance =
        sqlite::instance_rows::get_instance_by_id(instance_id, &state.pool)
            .await?
            .ok_or_else(|| {
                crate::ErrorKind::InputError("Unknown instance".to_string())
            })?;

    sync_instance_content_files(&instance, state).await
}

pub(crate) async fn sync_instance_content_files(
    instance: &Instance,
    state: &State,
) -> crate::Result<Vec<InstanceFile>> {
    cleanup_install_temporary_files(instance, state)?;
    let scanned = filesystem::scan_content_files(
        &state.directories.instances_dir(),
        &instance.path,
    )?;
    let cache_keys = scanned
        .iter()
        .map(|file| file.hash_cache_key.as_str())
        .collect::<Vec<_>>();
    let hashes = CachedEntry::get_file_hash_many(
        &cache_keys,
        None,
        &state.pool,
        &state.api_semaphore,
    )
    .await?;
    let hashes_by_key = hashes
        .into_iter()
        .map(|hash| {
            (
                format!(
                    "{}-{}",
                    hash.size,
                    hash.path.trim_end_matches(".disabled")
                ),
                hash,
            )
        })
        .collect::<HashMap<_, _>>();
    let existing_files =
        sqlite::content_rows::get_instance_files(&instance.id, &state.pool)
            .await?;
    let existing_files_by_path = existing_files
        .into_iter()
        .map(|file| (file.relative_path.clone(), file))
        .collect::<HashMap<_, _>>();

    let now = Utc::now();
    let mut files: Vec<InstanceFile> = Vec::new();

    for file in scanned {
        let hash_key = file.hash_cache_key.trim_end_matches(".disabled");
        let Some(hash) = hashes_by_key.get(hash_key) else {
            continue;
        };
        let existing_file = existing_files_by_path.get(&file.relative_path);

        files.push(InstanceFile {
            id: existing_file
                .map(|file| file.id.clone())
                .unwrap_or_else(instance_file_id),
            instance_id: instance.id.clone(),
            relative_path: file.relative_path,
            file_name: file.file_name,
            enabled: file.enabled,
            sha1: hash.hash.clone(),
            size: file.size,
            missing: false,
            added_at: existing_file.map(|file| file.added_at).unwrap_or(now),
            modified_at: now,
            local_mod_data: existing_file
                .and_then(|f| f.local_mod_data.clone()),
            icon_path: existing_file.and_then(|f| f.icon_path.clone()),
        });
    }

    // Extract local mod metadata (Mod JARs) and cached icons (Mod JARs and
    // resource packs) for files that don't have them yet. This also backfills
    // rows created before these features existed; `icon_path` distinguishes
    // not-attempted (NULL), no-icon (empty string), and cached (path).
    let instance_dir = state.directories.instances_dir().join(&instance.path);
    let icon_cache_dir = state.directories.caches_dir().join("icons");
    for file in &mut files {
        let Some(project_type) = project_type_for_file(file) else {
            continue;
        };
        let extract_metadata =
            project_type == ProjectType::Mod && file.local_mod_data.is_none();
        let extract_icon = file.icon_path.is_none()
            && matches!(
                project_type,
                ProjectType::Mod | ProjectType::ResourcePack
            );
        if !extract_metadata && !extract_icon {
            continue;
        }

        let path = instance_dir.join(&file.relative_path);

        // Resource packs are read entry-wise so large archives are not
        // materialized in memory just to fetch `pack.png`.
        if extract_icon && project_type == ProjectType::ResourcePack {
            let icon =
                crate::mod_metadata::icon::extract_resource_pack_icon(&path);
            file.icon_path = Some(
                cache_extracted_icon(icon, &file.sha1, &icon_cache_dir, state)
                    .await,
            );
            continue;
        }

        // Mods: one in-memory read serves both metadata and icon extraction.
        let bytes = match tokio::fs::read(&path).await {
            Ok(data) => bytes::Bytes::from(data),
            Err(_) => {
                // File temporarily inaccessible; skip silently.
                continue;
            }
        };

        if extract_metadata
            && let Some(meta) =
                crate::mod_metadata::extract_mod_metadata(&bytes)
            && let Ok(json) = serde_json::to_string(&meta)
        {
            file.local_mod_data = Some(json);
        }

        if extract_icon {
            let meta = file.local_mod_data.as_ref().and_then(|json| {
                serde_json::from_str::<crate::mod_metadata::LocalModMetadata>(
                    json,
                )
                .ok()
            });
            let icon = crate::mod_metadata::icon::extract_mod_icon(
                &bytes,
                meta.as_ref(),
            );
            file.icon_path = Some(
                cache_extracted_icon(icon, &file.sha1, &icon_cache_dir, state)
                    .await,
            );
        }
    }

    // The write below inserts foreign-keyed `instance_files` rows. Serialize it
    // with instance deletion and re-validate the parent row inside the
    // transaction so a concurrent delete fails with a clean error instead of
    // FK 787. Scanning and hashing above stay unlocked to keep concurrent
    // operations parallel.
    let _instance_lock = state.lock_instance_content(&instance.id).await;

    let mut tx = state.pool.begin_with("BEGIN IMMEDIATE").await?;
    sqlite::content_rows::ensure_instance_exists(&instance.id, &mut tx).await?;
    sqlite::content_rows::mark_instance_files_missing(&instance.id, &mut tx)
        .await?;

    // Upsert with a fresh id lookup inside the transaction. The ids assigned
    // during the scan may be stale if a concurrent operation (e.g. batch
    // disable renaming files to `.disabled`) moved a row after the snapshot;
    // reusing a stale id against the moved row would trip the UNIQUE
    // constraint on `instance_files.id` (code 1555).
    let mut synced_files: Vec<InstanceFile> = Vec::with_capacity(files.len());
    for file in &files {
        let synced =
            sqlite::content_rows::upsert_instance_file_from_parts_in_transaction(
                sqlite::content_rows::UpsertInstanceFile {
                    instance_id: &instance.id,
                    relative_path: &file.relative_path,
                    file_name: &file.file_name,
                    enabled: file.enabled,
                    sha1: &file.sha1,
                    size: file.size,
                    missing: false,
                    local_mod_data: file.local_mod_data.as_deref(),
                    icon_path: file.icon_path.as_deref(),
                },
                &mut tx,
            )
            .await?;
        synced_files.push(synced);
    }

    tx.commit().await?;

    Ok(synced_files)
}

async fn cache_extracted_icon(
    icon: Option<(String, Vec<u8>)>,
    sha1: &str,
    icon_cache_dir: &Path,
    state: &State,
) -> String {
    let Some((entry_name, icon_bytes)) = icon else {
        return String::new();
    };

    let extension = icon_extension(&entry_name);
    let cache_path = icon_cache_dir.join(format!("{sha1}.{extension}"));
    match fetch::write(&cache_path, &icon_bytes, &state.io_semaphore).await {
        Ok(()) => crate::util::io::canonicalize(&cache_path)
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|_| cache_path.to_string_lossy().into_owned()),
        Err(_) => String::new(),
    }
}

fn icon_extension(entry_name: &str) -> &str {
    let extension = Path::new(entry_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default();
    if matches!(
        extension.to_ascii_lowercase().as_str(),
        "png" | "jpg" | "jpeg"
    ) {
        extension
    } else {
        "png"
    }
}

fn cleanup_install_temporary_files(
    instance: &Instance,
    state: &State,
) -> crate::Result<()> {
    let instance_dir = state.directories.instances_dir().join(&instance.path);
    for project_type in ProjectType::iterator() {
        let folder = instance_dir.join(project_type.get_folder());
        if !folder.is_dir() {
            continue;
        }
        for entry in std::fs::read_dir(&folder)
            .map_err(crate::util::io::IOError::from)?
        {
            let path = entry.map_err(crate::util::io::IOError::from)?.path();
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            if (name.ends_with(".installing")
                || name.ends_with(".installing.previous")
                || name.ends_with(".installing.download"))
                && path.is_file()
            {
                std::fs::remove_file(path)
                    .map_err(crate::util::io::IOError::from)?;
            }
        }
    }
    Ok(())
}

pub(crate) fn project_type_for_file(
    file: &InstanceFile,
) -> Option<ProjectType> {
    filesystem::project_type_from_relative_path(&file.relative_path)
}

/// Whether a file may receive Modrinth update suggestions.
///
/// Files installed from Modrinth (origin `Modrinth`) always qualify. Untracked
/// or locally recorded files (no origin) qualify as long as no CurseForge
/// reference ties them to a different provider; CurseForge-origin files are
/// handled by the CurseForge update path instead.
pub(crate) fn modrinth_update_enabled(
    origin_provider: Option<ContentProvider>,
    provider_refs: &[ContentProviderRef],
) -> bool {
    match origin_provider {
        Some(ContentProvider::Modrinth) => true,
        Some(ContentProvider::CurseForge) => false,
        None => provider_refs.iter().all(|reference| {
            matches!(reference, ContentProviderRef::Modrinth { .. })
        }),
    }
}

fn instance_file_id() -> String {
    format!("instance-file:{}", Uuid::new_v4())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{
        CurseForgeFileId, CurseForgeProjectId, ModrinthProjectId,
        ModrinthVersionId,
    };

    fn modrinth_ref() -> ContentProviderRef {
        ContentProviderRef::Modrinth {
            project_id: ModrinthProjectId::new("project").unwrap(),
            version_id: Some(ModrinthVersionId::new("version").unwrap()),
        }
    }

    fn curseforge_ref() -> ContentProviderRef {
        ContentProviderRef::CurseForge {
            project_id: CurseForgeProjectId::new(42).unwrap(),
            file_id: Some(CurseForgeFileId::new(7).unwrap()),
        }
    }

    #[test]
    fn untracked_files_qualify_for_modrinth_updates() {
        assert!(modrinth_update_enabled(None, &[]));
        assert!(modrinth_update_enabled(None, &[modrinth_ref()]));
    }

    #[test]
    fn curseforge_tracked_files_do_not_qualify_for_modrinth_updates() {
        assert!(!modrinth_update_enabled(
            Some(ContentProvider::CurseForge),
            &[curseforge_ref()],
        ));
        assert!(!modrinth_update_enabled(None, &[curseforge_ref()]));
    }

    #[test]
    fn modrinth_origin_always_qualifies() {
        assert!(modrinth_update_enabled(
            Some(ContentProvider::Modrinth),
            &[curseforge_ref(), modrinth_ref()],
        ));
    }
}
