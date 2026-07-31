use crate::state::State;
use crate::util::io;
use crate::{ErrorKind, Result};
use std::path::{Path, PathBuf};

/// Import a world save from a source path into an instance's saves directory.
///
/// The `source_path` can be either:
/// - A directory containing a `level.dat` file (an existing world folder)
/// - A ZIP archive containing a world save (`level.dat` at the archive root,
///   or inside a single shared root folder such as `My World/level.dat`)
///
/// Returns the name of the imported world.
pub async fn import_world_save(
    _state: &State,
    instance_id: &str,
    source_path: &Path,
) -> Result<String> {
    let instance_id_str = instance_id.to_string();

    // Resolve the instance's saves directory.
    let instance_path =
        crate::api::instance::get_full_path(&instance_id_str).await?;
    let saves_dir = instance_path.join("saves");

    // Determine the world folder name and source type.
    let (world_name, source_is_zip) = if source_path.is_dir() {
        // Direct folder: use the folder name as the world name.
        let name = source_path
            .file_name()
            .ok_or_else(|| {
                ErrorKind::InputError(
                    "Cannot determine world name from source path".to_string(),
                )
            })?
            .to_string_lossy()
            .to_string();
        (name, false)
    } else if source_path.is_file() {
        // Check if it's a ZIP archive by examining the file signature.
        let is_zip = is_zip_file(source_path).await?;
        if is_zip {
            // ZIP file: use the file stem as the world name.
            let name = source_path
                .file_stem()
                .ok_or_else(|| {
                    ErrorKind::InputError(
                        "Cannot determine world name from ZIP file name"
                            .to_string(),
                    )
                })?
                .to_string_lossy()
                .to_string();
            (name, true)
        } else {
            return Err(ErrorKind::InputError(
                "Source file is not a valid ZIP archive or world folder"
                    .to_string(),
            )
            .into());
        }
    } else {
        return Err(ErrorKind::InputError(format!(
            "Source path does not exist: {}",
            source_path.display()
        ))
        .into());
    };

    // Check if the world already exists in the saves directory.
    let target_dir = saves_dir.join(&world_name);
    if target_dir.exists() {
        return Err(ErrorKind::InputError(format!(
            "World '{world_name}' already exists in this instance"
        ))
        .into());
    }

    // Create the saves directory if it doesn't exist.
    io::create_dir_all(&saves_dir).await?;

    if source_is_zip {
        // Extract ZIP archive to the target directory.
        extract_world_zip(source_path, &target_dir).await?;
    } else {
        // Copy the folder recursively.
        io::copy_dir(source_path, &target_dir).await?;
    }

    // Verify that the extracted/copied world has a level.dat file.
    if !target_dir.join("level.dat").exists() {
        // Clean up on failure.
        let _ = tokio::fs::remove_dir_all(&target_dir).await;
        return Err(ErrorKind::InputError(format!(
            "No level.dat found in the imported world save '{world_name}'"
        ))
        .into());
    }

    // Emit an instance synced event so the UI refreshes the worlds list.
    crate::event::emit::emit_instance(
        &instance_id_str,
        crate::event::InstancePayloadType::Synced,
    )
    .await?;

    tracing::info!(
        "Imported world save '{world_name}' into instance {instance_id_str}"
    );

    Ok(world_name)
}

/// Check if a file is a ZIP archive by reading its magic bytes.
async fn is_zip_file(path: &Path) -> Result<bool> {
    use tokio::io::AsyncReadExt;

    let mut file = tokio::fs::File::open(path)
        .await
        .map_err(|e| io::IOError::with_path(e, path))?;
    let mut magic = [0u8; 4];
    match file.read_exact(&mut magic).await {
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
            // File too small to be a ZIP
            return Ok(false);
        }
        Err(e) => {
            return Err(io::IOError::with_path(e, path).into());
        }
        _ => {}
    }
    // ZIP magic bytes: PK\x03\x04
    Ok(magic == [0x50, 0x4B, 0x03, 0x04])
}

/// Extract a ZIP archive containing a world save to the target directory.
///
/// Supports flat archives (`level.dat` at the root) and archives whose
/// entries all share a single root folder (`My World/level.dat`); the shared
/// root is stripped only when every entry lives inside it. Entry names are
/// normalized (backslashes become `/`) and validated so `..`, absolute paths
/// and drive letters can never write outside `target_dir`.
async fn extract_world_zip(zip_path: &Path, target_dir: &Path) -> Result<()> {
    let zip_path = zip_path.to_path_buf();
    let target_dir = target_dir.to_path_buf();

    tokio::task::spawn_blocking(move || {
        extract_world_zip_sync(&zip_path, &target_dir)
    })
    .await?
}

fn extract_world_zip_sync(zip_path: &Path, target_dir: &Path) -> Result<()> {
    let file = std::fs::File::open(zip_path)
        .map_err(|e| io::IOError::with_path(e, zip_path))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| {
        ErrorKind::InputError(format!("Invalid ZIP archive: {e}"))
    })?;

    // First pass: sanitize every entry name. Entries that try to escape the
    // extraction directory are skipped instead of written.
    let mut entries: Vec<(usize, PathBuf, bool)> = Vec::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(|e| {
            ErrorKind::InputError(format!("Failed to read ZIP entry: {e}"))
        })?;

        let raw_name = entry.name().to_string();
        if raw_name.starts_with("__MACOSX") {
            continue;
        }
        let is_dir =
            entry.is_dir() || raw_name.replace('\\', "/").ends_with('/');
        let Some(safe_name) = sanitize_entry_name(&raw_name) else {
            tracing::warn!(
                "import_world_save: skipping unsafe ZIP entry '{raw_name}' (path traversal)"
            );
            continue;
        };
        if safe_name.as_os_str().is_empty() {
            continue;
        }
        entries.push((i, safe_name, is_dir));
    }

    // Strip a shared root folder only when every file entry is nested below
    // it. Root-level directory entries don't block stripping, and a flat
    // archive (level.dat at the root) keeps its paths untouched.
    let all_nested = entries
        .iter()
        .filter(|(_, _, is_dir)| !is_dir)
        .all(|(_, path, _)| path.components().count() > 1);
    let root = common_root(&entries);
    let strip_root = all_nested && root.is_some();

    for (index, safe_name, is_dir) in &entries {
        let relative = if strip_root {
            let mut components = safe_name.components();
            components.next();
            components.as_path().to_path_buf()
        } else {
            safe_name.clone()
        };
        let output_path = target_dir.join(relative);

        if *is_dir {
            std::fs::create_dir_all(&output_path)
                .map_err(|e| io::IOError::with_path(e, &output_path))?;
            continue;
        }
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| io::IOError::with_path(e, parent))?;
        }

        let mut entry = archive.by_index(*index).map_err(|e| {
            ErrorKind::InputError(format!("Failed to read ZIP entry: {e}"))
        })?;
        let mut output = std::fs::File::create(&output_path)
            .map_err(|e| io::IOError::with_path(e, &output_path))?;
        std::io::copy(&mut entry, &mut output)
            .map_err(|e| io::IOError::with_path(e, &output_path))?;
    }

    Ok(())
}

/// Returns the first path component shared by every entry, if any.
fn common_root(entries: &[(usize, PathBuf, bool)]) -> Option<&std::ffi::OsStr> {
    let mut root: Option<&std::ffi::OsStr> = None;
    for (_, path, _) in entries {
        let first = path.components().next()?.as_os_str();
        match root {
            None => root = Some(first),
            Some(existing) if existing != first => return None,
            _ => {}
        }
    }
    root
}

/// Normalize a ZIP entry name into a safe relative path that stays inside the
/// extraction directory. Returns `None` for absolute paths, drive letters or
/// entries containing `..` (zip-slip protection).
fn sanitize_entry_name(name: &str) -> Option<PathBuf> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;
    use tempfile::tempdir;

    fn write_zip(entries: &[(&str, &[u8])], zip_path: &Path) {
        let file = std::fs::File::create(zip_path).expect("create zip");
        let mut zip = zip::ZipWriter::new(file);
        for (name, bytes) in entries {
            zip.start_file(name, zip::write::FileOptions::<()>::default())
                .expect("start entry");
            zip.write_all(bytes).expect("write entry");
        }
        zip.finish().expect("finish zip");
    }

    fn extract_to_temp(
        entries: &[(&str, &[u8])],
    ) -> (tempfile::TempDir, tempfile::TempDir) {
        let dir = tempdir().expect("temp dir");
        let zip_path = dir.path().join("world.zip");
        write_zip(entries, &zip_path);

        let out_dir = tempdir().expect("temp out dir");
        extract_world_zip_sync(&zip_path, out_dir.path()).expect("extract");
        (dir, out_dir)
    }

    #[test]
    fn flat_zip_keeps_root_files() {
        let (_, out_dir) = extract_to_temp(&[
            ("level.dat", b"flat"),
            ("region/r.0.0.mca", b"mca"),
        ]);

        assert!(out_dir.path().join("level.dat").exists());
        assert!(out_dir.path().join("region/r.0.0.mca").exists());
    }

    #[test]
    fn single_root_zip_is_stripped() {
        let (_, out_dir) = extract_to_temp(&[
            ("My World/level.dat", b"rooted"),
            ("My World/region/r.0.0.mca", b"mca"),
        ]);

        assert!(out_dir.path().join("level.dat").exists());
        assert!(!out_dir.path().join("My World").exists());
    }

    #[test]
    fn backslash_entries_are_normalized() {
        let (_, out_dir) = extract_to_temp(&[
            ("My World\\level.dat", b"rooted"),
            ("My World\\region\\r.0.0.mca", b"mca"),
        ]);

        assert!(out_dir.path().join("level.dat").exists());
        assert!(!out_dir.path().join("My World").exists());
    }

    #[test]
    fn traversal_entries_are_rejected() {
        let (dir, out_dir) = extract_to_temp(&[
            ("../../evil.txt", b"escape"),
            ("C:/evil.txt", b"drive"),
            ("/evil.txt", b"absolute"),
        ]);

        assert!(!dir.path().join("evil.txt").exists());
        assert!(!out_dir.path().join("evil.txt").exists());
        assert_eq!(
            std::fs::read_dir(out_dir.path())
                .expect("read out dir")
                .count(),
            0
        );
    }

    #[test]
    fn directory_entries_are_created() {
        let (_, out_dir) = extract_to_temp(&[
            ("My World/", b""),
            ("My World/level.dat", b"rooted"),
        ]);

        assert!(out_dir.path().join("level.dat").exists());
        assert!(!out_dir.path().join("My World").exists());
    }
}
