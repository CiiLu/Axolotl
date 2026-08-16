use crate::api::Result;
use async_zip::base::read::seek::ZipFileReader;
use image::ImageEncoder;
use serde::Serialize;
use std::io::Cursor;
use tauri::Runtime;
use tauri_plugin_dialog::DialogExt;
use theseus::instance::get_full_path;

pub fn init<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("files")
        .invoke_handler(tauri::generate_handler![
            file_extract_zip,
            file_save_as,
            file_read_dragged_file,
            screenshot_thumbnail,
        ])
        .build()
}

#[derive(Serialize)]
pub struct ExtractDryRunResult {
    modpack_name: Option<String>,
    conflicting_files: Vec<String>,
}

#[tauri::command]
pub async fn file_read_dragged_file(
    path: String,
) -> Result<tauri::ipc::Response> {
    let metadata = tokio::fs::metadata(&path).await?;
    if !metadata.is_file() {
        return Err(theseus::Error::from(theseus::ErrorKind::OtherError(
            "Dropped path is not a file".to_string(),
        ))
        .into());
    }

    // Raw binary payload: the frontend receives an ArrayBuffer instead of a
    // JSON number array, which would balloon multi-hundred-MB files to
    // gigabytes of transient memory on both sides of the IPC boundary.
    Ok(tauri::ipc::Response::new(tokio::fs::read(path).await?))
}

/// Decodes a screenshot and returns a downscaled thumbnail as raw image bytes,
/// so the webview never decodes high-resolution originals in the screenshots grid.
/// Files that already fit within `max_dimension` are returned unchanged.
#[tauri::command]
pub async fn screenshot_thumbnail(
    instance_id: &str,
    file_path: &str,
    max_dimension: u32,
) -> Result<tauri::ipc::Response> {
    let base = get_full_path(instance_id).await?;
    let canonical_source =
        tokio::fs::canonicalize(base.join(file_path)).await?;
    let canonical_base = tokio::fs::canonicalize(&base).await?;
    if !canonical_source.starts_with(&canonical_base) {
        return Err(theseus::Error::from(theseus::ErrorKind::OtherError(
            "file_path escapes the instance directory".to_string(),
        ))
        .into());
    }

    let bytes = tokio::fs::read(&canonical_source).await?;
    let max_dimension = max_dimension.max(1);
    let thumbnail = tokio::task::spawn_blocking(move || -> Result<Vec<u8>> {
        let (width, height) =
            image::ImageReader::new(Cursor::new(bytes.as_slice()))
                .with_guessed_format()
                .map_err(|error| thumbnail_error(error.into()))?
                .into_dimensions()
                .map_err(thumbnail_error)?;
        if width <= max_dimension && height <= max_dimension {
            return Ok(bytes);
        }

        let decoded = image::ImageReader::new(Cursor::new(bytes.as_slice()))
            .with_guessed_format()
            .map_err(|error| thumbnail_error(error.into()))?
            .decode()
            .map_err(thumbnail_error)?;
        let thumbnail = decoded.thumbnail(max_dimension, max_dimension);
        let mut output = Vec::new();
        if thumbnail.color().has_alpha() {
            let rgba = thumbnail.to_rgba8();
            image::codecs::png::PngEncoder::new(&mut output)
                .write_image(
                    rgba.as_raw(),
                    rgba.width(),
                    rgba.height(),
                    image::ExtendedColorType::Rgba8,
                )
                .map_err(thumbnail_error)?;
        } else {
            let rgb = thumbnail.to_rgb8();
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut output, 85)
                .write_image(
                    rgb.as_raw(),
                    rgb.width(),
                    rgb.height(),
                    image::ExtendedColorType::Rgb8,
                )
                .map_err(thumbnail_error)?;
        }
        Ok(output)
    })
    .await
    .map_err(|e| {
        theseus::Error::from(theseus::ErrorKind::OtherError(format!(
            "Screenshot thumbnail task failed: {e}"
        )))
    })??;

    Ok(tauri::ipc::Response::new(thumbnail))
}

fn thumbnail_error(error: image::ImageError) -> theseus::Error {
    theseus::Error::from(theseus::ErrorKind::OtherError(format!(
        "Failed to process screenshot: {error}"
    )))
}

#[tauri::command]
pub async fn file_extract_zip(
    instance_id: &str,
    file_path: &str,
    override_conflicts: bool,
    dry_run: bool,
) -> Result<Option<ExtractDryRunResult>> {
    let base = get_full_path(instance_id).await?;
    let zip_path = base.join(file_path);
    let canonical_zip = tokio::fs::canonicalize(&zip_path).await?;
    let canonical_base = tokio::fs::canonicalize(&base).await?;
    if !canonical_zip.starts_with(&canonical_base) {
        return Err(theseus::Error::from(theseus::ErrorKind::OtherError(
            "file_path escapes the instance directory".to_string(),
        ))
        .into());
    }
    let extract_dir = zip_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| base.clone());

    let file_bytes = tokio::fs::read(&zip_path).await?;
    let reader = Cursor::new(file_bytes);

    let zip_reader = ZipFileReader::with_tokio(reader).await.map_err(|e| {
        theseus::Error::from(theseus::ErrorKind::OtherError(format!(
            "Failed to read zip file: {e}"
        )))
    })?;

    let entries: Vec<(usize, String)> = zip_reader
        .file()
        .entries()
        .iter()
        .enumerate()
        .filter_map(|(i, entry)| {
            let name = entry.filename().as_str().ok()?.to_string();
            if name.ends_with('/') {
                None
            } else {
                Some((i, name))
            }
        })
        .collect();

    if dry_run {
        let mut conflicting_files = Vec::new();
        let canonical_extract = tokio::fs::canonicalize(&extract_dir).await?;
        for (_, name) in &entries {
            let target = extract_dir.join(name);
            if let Some(parent) = target.parent() {
                let normalized = parent
                    .canonicalize()
                    .unwrap_or_else(|_| extract_dir.join(parent));
                if !normalized.starts_with(&canonical_extract) {
                    continue;
                }
            }
            if target.exists() {
                conflicting_files.push(name.clone());
            }
        }
        return Ok(Some(ExtractDryRunResult {
            modpack_name: None,
            conflicting_files,
        }));
    }

    let canonical_extract_dir = tokio::fs::canonicalize(&extract_dir).await?;
    let mut zip_reader = zip_reader;
    for (index, name) in &entries {
        let target = extract_dir.join(name);

        if !override_conflicts && target.exists() {
            continue;
        }

        if let Some(parent) = target.parent() {
            tokio::fs::create_dir_all(parent).await?;
            let canonical_parent = tokio::fs::canonicalize(parent).await?;
            if !canonical_parent.starts_with(&canonical_extract_dir) {
                continue;
            }
        }

        let mut file_bytes = Vec::new();
        let mut entry_reader =
            zip_reader.reader_with_entry(*index).await.map_err(|e| {
                theseus::Error::from(theseus::ErrorKind::OtherError(format!(
                    "Failed to read zip entry: {e}"
                )))
            })?;
        entry_reader
            .read_to_end_checked(&mut file_bytes)
            .await
            .map_err(|e| {
                theseus::Error::from(theseus::ErrorKind::OtherError(format!(
                    "Failed to extract zip entry: {e}"
                )))
            })?;

        tokio::fs::write(&target, &file_bytes).await?;
    }

    Ok(None)
}

#[tauri::command]
pub async fn file_save_as<R: Runtime>(
    app: tauri::AppHandle<R>,
    instance_id: &str,
    file_path: &str,
) -> Result<()> {
    let base = get_full_path(instance_id).await?;
    let source = base.join(file_path);
    let file_name = source
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_file_name(&file_name)
        .save_file(|path| {
            let _ = tx.send(path);
        });

    if let Some(dest) = rx.await.unwrap_or(None) {
        let dest_path = std::path::PathBuf::try_from(dest).map_err(|e| {
            theseus::Error::from(theseus::ErrorKind::OtherError(format!(
                "Invalid save path: {e}"
            )))
        })?;
        tokio::fs::copy(&source, &dest_path).await?;
    }

    Ok(())
}
