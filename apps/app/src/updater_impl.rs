use crate::api::Result;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::http::HeaderValue;
use tauri::http::header::ACCEPT;
use tauri::{Manager, ResourceId, Runtime, Webview};
use tauri_plugin_http::reqwest;
use tauri_plugin_http::reqwest::ClientBuilder;
use tauri_plugin_updater::{Error, Update, UpdaterExt};
use theseus::{
    LoadingBarType, emit_loading, init_loading, launcher_user_agent,
};
use tokio::time::Instant;
use url::Url;

const MIawa_API_BASE: &str = "https://miawa.cn/api/v2";
const MIawa_HOST: &str = "https://miawa.cn";
const MIawa_API_TIMEOUT: std::time::Duration =
    std::time::Duration::from_secs(15);
const MIawa_DOWNLOAD_TIMEOUT: std::time::Duration =
    std::time::Duration::from_secs(300);

// ── Miawa API types ──────────────────────────────────────────────

#[derive(Deserialize)]
struct MiawaEnvelope<T> {
    data: T,
}

#[derive(Deserialize)]
struct MiawaLaunchers {
    axolotl: Vec<MiawaLauncherEntry>,
}

#[derive(Deserialize)]
struct MiawaLauncherEntry {
    tag_name: String,
    assets: Vec<MiawaAsset>,
}

#[derive(Deserialize)]
struct MiawaAsset {
    name: String,
    size: u64,
}

#[derive(Deserialize)]
struct MiawaPrepare {
    download_url: String,
}

// ── latest.json types (same format as GitHub) ────────────────────

#[derive(Deserialize, Serialize)]
struct LatestManifest {
    version: String,
    notes: Option<String>,
    #[serde(default)]
    platforms: std::collections::HashMap<String, PlatformEntry>,
}

#[derive(Deserialize, Serialize)]
struct PlatformEntry {
    url: String,
}

// ── Shared types ─────────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMetadata {
    rid: Option<ResourceId>,
    current_version: String,
    version: String,
    date: Option<String>,
    body: Option<String>,
    raw_json: serde_json::Value,
    mirror_download_url: Option<String>,
    update_size: Option<u64>,
}

pub enum PendingUpdateEntry {
    Plugin {
        update: Arc<Update>,
        data: Vec<u8>,
    },
    Mirror {
        data: Vec<u8>,
        version: String,
        current_version: String,
    },
}

#[derive(Default)]
pub struct PendingUpdateData(pub Mutex<Option<PendingUpdateEntry>>);

// ── Platform helper ──────────────────────────────────────────────

fn current_platform_key() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("windows", "x86_64") => "windows-x86_64",
        ("linux", "x86_64") => "linux-x86_64",
        ("linux", "aarch64") => "linux-aarch64",
        ("macos", "x86_64") => "darwin-x86_64",
        ("macos", "aarch64") => "darwin-aarch64",
        _ => "unknown",
    }
}

/// Compare two version strings, ignoring a leading `v`/`V` prefix and any
/// pre-release / build suffix. Returns the ordering of `a` relative to `b`.
fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let numeric_parts = |v: &str| -> Vec<u64> {
        v.trim()
            .trim_start_matches(['v', 'V'])
            .split(|c: char| !c.is_ascii_digit() && c != '.')
            .filter_map(|p| p.parse::<u64>().ok())
            .collect()
    };

    let an = numeric_parts(a);
    let bn = numeric_parts(b);

    for (x, y) in an.iter().zip(bn.iter()) {
        match x.cmp(y) {
            std::cmp::Ordering::Equal => continue,
            ord => return ord,
        }
    }

    an.len().cmp(&bn.len())
}

// ── Miawa API helpers ────────────────────────────────────────────

fn miawa_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(launcher_user_agent())
        .timeout(MIawa_API_TIMEOUT)
        .build()
        .expect("Failed to build Miawa HTTP client")
}

async fn miawa_get_latest_json(
    client: &reqwest::Client,
    tag_name: &str,
) -> std::result::Result<LatestManifest, String> {
    // Step 1: get prepared download URL for latest.json
    let prepare_resp: MiawaEnvelope<MiawaPrepare> = client
        .post(format!("{MIawa_API_BASE}/downloads/prepare"))
        .json(&serde_json::json!({ "file_path": format!("axolotl/{tag_name}/latest.json") }))
        .send()
        .await
        .map_err(|e| format!("Miawa prepare request failed: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse Miawa prepare response: {e}"))?;

    let latest_url = format!("{MIawa_HOST}{}", prepare_resp.data.download_url);

    // Step 2: fetch latest.json
    let manifest: LatestManifest = client
        .get(&latest_url)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch Miawa latest.json: {e}"))?
        .json()
        .await
        .map_err(|e| format!("Failed to parse Miawa latest.json: {e}"))?;

    Ok(manifest)
}

/// Full Miawa check flow. Returns (current_version, version, raw_json,
/// mirror_download_url, update_size) on success.
async fn miawa_check_update(
    current_version: &str,
) -> std::result::Result<
    Option<(String, String, serde_json::Value, String, u64)>,
    String,
> {
    let client = miawa_client();

    // Step 1: get launcher info
    let launchers_resp: MiawaEnvelope<MiawaLaunchers> = client
        .get(format!("{MIawa_API_BASE}/launchers"))
        .send()
        .await
        .map_err(|e| format!("Miawa launchers request failed: {e}"))?
        .json()
        .await
        .map_err(|e| {
            format!("Failed to parse Miawa launchers response: {e}")
        })?;

    let launcher = launchers_resp
        .data
        .axolotl
        .into_iter()
        .next()
        .ok_or_else(|| "Miawa returned empty axolotl list".to_string())?;

    let tag_name = &launcher.tag_name;

    // Step 2: fetch latest.json
    let manifest = miawa_get_latest_json(&client, tag_name).await?;

    // Step 3: compare versions. Use the `version` field from latest.json
    // (same semantics as tauri-plugin-updater), NOT the tag name.
    // The Miawa mirror can lag behind by tens of minutes, so a mirror
    // version that is OLDER than the installed version must be treated
    // as a mirror problem (fall back to CNB), never as an update.
    let ordering = compare_versions(&manifest.version, current_version);
    tracing::info!(
        current_version = %current_version,
        mirror_version = %manifest.version,
        ordering = ?ordering,
        "Miawa version comparison result"
    );
    match ordering {
        std::cmp::Ordering::Equal => {
            tracing::info!("No update available via Miawa mirror (up to date)");
            return Ok(None);
        }
        std::cmp::Ordering::Less => {
            return Err(format!(
                "Miawa mirror is behind the installed version (mirror: {}, installed: {})",
                manifest.version, current_version
            ));
        }
        std::cmp::Ordering::Greater => {}
    }
    tracing::info!("Miawa mirror reports a newer version");

    let platform_key = current_platform_key();
    let platform_entry =
        manifest.platforms.get(platform_key).ok_or_else(|| {
            format!(
                "Miawa latest.json has no entry for platform {platform_key}"
            )
        })?;

    // Step 3: extract candidate filename from the download URL in latest.json
    let original_url = Url::parse(&platform_entry.url)
        .map_err(|e| format!("Failed to parse platform download URL: {e}"))?;
    let url_filename = original_url
        .path_segments()
        .and_then(|s| s.last().filter(|s| !s.is_empty()))
        .ok_or_else(|| {
            "Could not extract filename from download URL".to_string()
        })?;

    // Step 4: find the real asset name on the mirror. Prefer an exact
    // match against the latest.json URL filename; on Windows fall back
    // to the platform suffix match used by the reference script
    // (e.g. "...x64-setup.exe"). This matters because the mirror may
    // name files differently than the latest.json URLs.
    let filename = launcher
        .assets
        .iter()
        .map(|a| a.name.as_str())
        .find(|name| *name == url_filename)
        .or_else(|| {
            if platform_key == "windows-x86_64" {
                launcher
                    .assets
                    .iter()
                    .map(|a| a.name.as_str())
                    .find(|name| name.ends_with("x64-setup.exe"))
            } else {
                None
            }
        })
        .ok_or_else(|| {
            format!(
                "Could not find a matching asset for platform {platform_key} on the Miawa mirror (candidate: {url_filename})"
            )
        })?;

    tracing::info!(
        "Miawa mirror asset resolved: {filename} (from latest.json URL: {url_filename})"
    );

    // Step 5: get mirror download URL for the actual installer
    let file_path = format!("axolotl/{tag_name}/{filename}");
    let prepare_resp: MiawaEnvelope<MiawaPrepare> = client
        .post(format!("{MIawa_API_BASE}/downloads/prepare"))
        .json(&serde_json::json!({ "file_path": file_path }))
        .send()
        .await
        .map_err(|e| {
            format!("Miawa prepare request for installer failed: {e}")
        })?
        .json()
        .await
        .map_err(|e| {
            format!("Failed to parse Miawa prepare response for installer: {e}")
        })?;

    let mirror_url = format!("{MIawa_HOST}{}", prepare_resp.data.download_url);

    // Step 6: find matching asset size
    let update_size = launcher
        .assets
        .iter()
        .find(|a| a.name == filename)
        .map(|a| a.size);

    let raw_json =
        serde_json::to_value(&manifest).unwrap_or(serde_json::Value::Null);

    Ok(Some((
        current_version.to_string(),
        manifest.version,
        raw_json,
        mirror_url,
        update_size.unwrap_or(0),
    )))
}

async fn miawa_download_update(
    url: &str,
    version: &str,
    current_version: &str,
) -> Result<Vec<u8>> {
    let progress = init_loading(
        LoadingBarType::LauncherUpdate {
            version: version.to_string(),
            current_version: current_version.to_string(),
        },
        1.0,
        "Downloading update from Miawa mirror...",
    )
    .await?;

    let client = reqwest::Client::builder()
        .user_agent(launcher_user_agent())
        .timeout(MIawa_DOWNLOAD_TIMEOUT)
        .build()
        .map_err(|e| {
            theseus::Error::from(theseus::ErrorKind::OtherError(format!(
                "Failed to build download client: {e}"
            )))
        })?;

    let mut response = client.get(url).send().await.map_err(|e| {
        theseus::Error::from(theseus::ErrorKind::OtherError(format!(
            "Miawa download request failed: {e}"
        )))
    })?;

    if !response.status().is_success() {
        return Err(theseus::Error::from(theseus::ErrorKind::OtherError(
            format!("Miawa download failed with status: {}", response.status()),
        ))
        .into());
    }

    let total_size = response.content_length().unwrap_or(0);
    tracing::info!(
        "Miawa mirror download starting (version {version}): {url} (content-length: {total_size})"
    );
    let mut data = Vec::new();
    let mut downloaded = 0u64;

    while let Some(chunk) = response.chunk().await.map_err(|e| {
        theseus::Error::from(theseus::ErrorKind::OtherError(format!(
            "Miawa download chunk failed: {e}"
        )))
    })? {
        data.extend_from_slice(&chunk);
        downloaded += chunk.len() as u64;
        if total_size > 0 {
            let _ = emit_loading(
                &progress,
                downloaded as f64 / total_size as f64,
                None,
            );
        }
    }

    // Ensure the progress bar completes even when the server did not
    // report a content length.
    let _ = emit_loading(&progress, 1.0, None);

    // Guard against truncated downloads.
    if total_size > 0 && downloaded != total_size {
        return Err(theseus::Error::from(theseus::ErrorKind::OtherError(
            format!(
                "Miawa download truncated: received {downloaded} of {total_size} bytes"
            ),
        ))
        .into());
    }

    validate_downloaded_update(&data)?;

    tracing::info!(
        "Downloaded update from Miawa mirror: {} bytes in total",
        data.len()
    );
    Ok(data)
}

/// Validate that a downloaded update payload looks like a real update
/// package instead of an HTML error page or other bogus content.
fn validate_downloaded_update(data: &[u8]) -> Result<()> {
    let head = String::from_utf8_lossy(&data[..data.len().min(512)]).to_lowercase();
    if head.trim_start().starts_with("<!doctype html")
        || head.trim_start().starts_with("<html")
        || head.trim_start().starts_with("<head")
    {
        return Err(theseus::Error::from(theseus::ErrorKind::OtherError(
            "Downloaded file looks like an HTML page instead of an update package"
                .to_string(),
        ))
        .into());
    }

    #[cfg(target_os = "windows")]
    {
        // The mirror may deliver either a raw installer (MZ) or a zip
        // archive (PK) wrapping the installer; anything else is bogus.
        let ok = data.starts_with(b"MZ") || data.starts_with(b"PK");
        if !ok {
            let first_two = data
                .iter()
                .take(2)
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(" ");
            return Err(theseus::Error::from(theseus::ErrorKind::OtherError(
                format!(
                    "Downloaded file is not a valid Windows update payload (expected MZ or PK header, got [{first_two}], {} bytes received)",
                    data.len()
                ),
            ))
            .into());
        }
    }

    Ok(())
}

/// Resolve the actual installer payload from downloaded bytes. The
/// mirror may hand us the installer directly (MZ header) or wrapped in
/// a zip archive (PK header); in the latter case the first `.exe` entry
/// is extracted. Only used on Windows.
#[cfg(target_os = "windows")]
fn extract_installer_payload(data: &[u8]) -> Result<Vec<u8>> {
    if !data.starts_with(b"PK") {
        return Ok(data.to_vec());
    }

    use std::io::Read;

    let cursor = std::io::Cursor::new(data);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| {
        theseus::Error::from(theseus::ErrorKind::OtherError(format!(
            "Failed to open downloaded update archive: {e}"
        )))
    })?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| {
            theseus::Error::from(theseus::ErrorKind::OtherError(format!(
                "Failed to read update archive entry: {e}"
            )))
        })?;
        let name = entry.name().to_string();
        if name.to_lowercase().ends_with(".exe") {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).map_err(|e| {
                theseus::Error::from(theseus::ErrorKind::OtherError(format!(
                    "Failed to extract {name} from update archive: {e}"
                )))
            })?;
            tracing::info!(
                "Extracted installer from update archive: {name} ({} bytes)",
                buf.len()
            );
            return Ok(buf);
        }
    }

    Err(theseus::Error::from(theseus::ErrorKind::OtherError(
        "Downloaded update archive contains no .exe file".to_string(),
    ))
    .into())
}

// ── Updater plugin helpers ───────────────────────────────────────

fn update_endpoints(source: &str) -> Result<Vec<Url>> {
    let endpoints = match source {
        "github" | "official" => vec![
            "https://github.com/Mystic-Stars/Axolotl/releases/latest/download/latest.json",
        ],
        "cnb" => vec![
            "https://cnb.cool/axlmc/Axolotl/-/git/raw/update/latest.json",
            "https://github.com/Mystic-Stars/Axolotl/releases/latest/download/latest.json",
        ],
        _ => {
            return Err(theseus::Error::from(theseus::ErrorKind::OtherError(
                format!("Unknown update source: {source}"),
            ))
            .into());
        }
    };

    endpoints
        .into_iter()
        .map(|endpoint| {
            Url::parse(endpoint).map_err(|error| {
                theseus::Error::from(theseus::ErrorKind::OtherError(
                    error.to_string(),
                ))
                .into()
            })
        })
        .collect()
}

async fn check_with_updater<R: Runtime>(
    webview: &Webview<R>,
    source: &str,
) -> Result<Option<UpdateMetadata>> {
    #[cfg(target_os = "windows")]
    let mut updater = webview
        .updater_builder()
        .endpoints(update_endpoints(source)?)?;
    #[cfg(not(target_os = "windows"))]
    let updater = webview
        .updater_builder()
        .endpoints(update_endpoints(source)?)?;

    #[cfg(target_os = "windows")]
    {
        let install_dir = std::env::current_exe()
            .map_err(|error| {
                theseus::Error::from(theseus::ErrorKind::OtherError(format!(
                    "Failed to resolve current executable: {error}"
                )))
            })?
            .parent()
            .ok_or_else(|| {
                theseus::Error::from(theseus::ErrorKind::OtherError(
                    "Current executable has no parent directory".to_string(),
                ))
            })?
            .to_path_buf();

        tracing::debug!(
            install_dir = %install_dir.display(),
            "Using current executable directory for Windows app updates"
        );
        updater = updater.installer_arg(format!(
            "/INSTALL_DIR=\"{}\"",
            install_dir.display()
        ));
    }

    let updater = updater.build()?;
    let Some(update) = updater.check().await? else {
        return Ok(None);
    };

    let metadata = UpdateMetadata {
        rid: Some(webview.resources_table().add(update.clone())),
        current_version: update.current_version.clone(),
        version: update.version.clone(),
        date: None,
        body: update.body.clone(),
        raw_json: update.raw_json,
        mirror_download_url: None,
        update_size: None,
    };

    Ok(Some(metadata))
}

// ── Install helper (for mirror downloads) ────────────────────────

pub fn install_mirror_update(data: &[u8], version: &str) -> Result<()> {
    // Resolve the actual installer payload. The mirror may deliver the
    // exe directly (MZ) or wrapped in a zip archive (PK); a bogus
    // payload must never be executed (it would produce errors like
    // "unsupported 16-bit application" on Windows).
    #[cfg(target_os = "windows")]
    let payload = extract_installer_payload(data)?;
    #[cfg(not(target_os = "windows"))]
    let payload = data.to_vec();

    let temp_dir = std::env::temp_dir();
    let ext = if cfg!(target_os = "windows") {
        "exe"
    } else if cfg!(target_os = "macos") {
        "dmg"
    } else {
        "AppImage"
    };
    let temp_file = temp_dir.join(format!("axolotl-update-{version}.{ext}"));

    std::fs::write(&temp_file, &payload).map_err(|e| {
        theseus::Error::from(theseus::ErrorKind::OtherError(format!(
            "Failed to write update to temp file: {e}"
        )))
    })?;

    #[cfg(target_os = "windows")]
    {
        let install_dir = std::env::current_exe()
            .map_err(|e| {
                theseus::Error::from(theseus::ErrorKind::OtherError(format!(
                    "Failed to resolve current executable: {e}"
                )))
            })?
            .parent()
            .ok_or_else(|| {
                theseus::Error::from(theseus::ErrorKind::OtherError(
                    "Current executable has no parent directory".to_string(),
                ))
            })?
            .to_path_buf();

        tracing::info!(
            "Launching Miawa mirror installer: {} (install dir: {})",
            temp_file.display(),
            install_dir.display()
        );

        std::process::Command::new(&temp_file)
            .arg("/S")
            .arg(format!("/INSTALL_DIR=\"{}\"", install_dir.display()))
            .spawn()
            .map_err(|e| {
                theseus::Error::from(theseus::ErrorKind::OtherError(format!(
                    "Failed to launch mirror installer: {e}"
                )))
            })?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        tracing::warn!(
            "Mirror update installation is only implemented for Windows; \
             update binary saved to {}",
            temp_file.display()
        );
        return Err(theseus::Error::from(theseus::ErrorKind::OtherError(
            "Automatic mirror update installation is not yet supported on this platform"
                .to_string(),
        ))
        .into());
    }

    #[cfg(target_os = "windows")]
    Ok(())
}

// ── Tauri commands ───────────────────────────────────────────────

#[tauri::command]
pub async fn check_app_update<R: Runtime>(
    webview: Webview<R>,
    source: String,
) -> Result<Option<UpdateMetadata>> {
    let current_version =
        webview.app_handle().package_info().version.to_string();

    match source.as_str() {
        "miawa" => {
            // 1. Try Miawa mirror
            match miawa_check_update(&current_version).await {
                Ok(Some((cur, ver, raw, mirror_url, size))) => {
                    tracing::info!("Update {ver} available via Miawa mirror");
                    return Ok(Some(UpdateMetadata {
                        rid: None,
                        current_version: cur,
                        version: ver,
                        date: None,
                        body: raw
                            .get("notes")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        raw_json: raw,
                        mirror_download_url: Some(mirror_url),
                        update_size: if size > 0 { Some(size) } else { None },
                    }));
                }
                Ok(None) => {
                    tracing::info!("App is up to date (checked via Miawa)");
                    return Ok(None);
                }
                Err(e) => {
                    tracing::warn!(
                        "Miawa check failed, falling back to CNB: {e}"
                    );
                }
            }

            // 2. Fallback: CNB
            match check_with_updater(&webview, "cnb").await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    tracing::warn!(
                        "CNB check failed, falling back to GitHub: {e}"
                    );
                }
            }

            // 3. Fallback: GitHub
            check_with_updater(&webview, "github").await
        }
        "cnb" => {
            // 1. Try CNB
            match check_with_updater(&webview, "cnb").await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    tracing::warn!(
                        "CNB check failed, falling back to GitHub: {e}"
                    );
                }
            }

            // 2. Fallback: GitHub
            check_with_updater(&webview, "github").await
        }
        "github" | "official" => check_with_updater(&webview, "github").await,
        _ => Err(theseus::Error::from(theseus::ErrorKind::OtherError(
            format!("Unknown update source: {source}"),
        ))
        .into()),
    }
}

#[tauri::command]
pub async fn get_update_size<R: Runtime>(
    webview: Webview<R>,
    rid: Option<ResourceId>,
    update_size: Option<u64>,
) -> Result<Option<u64>> {
    // If size is already known (e.g. from Miawa assets), return it directly
    if let Some(size) = update_size {
        if size > 0 {
            return Ok(Some(size));
        }
    }

    // Otherwise, HEAD request via the updater plugin
    let Some(rid) = rid else {
        return Ok(None);
    };

    let update = webview.resources_table().get::<Update>(rid)?;

    let mut headers = update.headers.clone();
    if !headers.contains_key(ACCEPT) {
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/octet-stream"),
        );
    }

    let mut request = ClientBuilder::new().user_agent(launcher_user_agent());
    if let Some(timeout) = update.timeout {
        request = request.timeout(timeout);
    }
    if let Some(ref proxy) = update.proxy {
        let proxy = reqwest::Proxy::all(proxy.as_str())?;
        request = request.proxy(proxy);
    }
    let response = request
        .build()?
        .head(update.download_url.clone())
        .headers(headers)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(Error::Network(format!(
            "Download request failed with status: {}",
            response.status()
        ))
        .into());
    }

    let content_length = response
        .headers()
        .get("Content-Length")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok());

    Ok(content_length)
}

#[tauri::command]
pub async fn enqueue_update_for_installation<R: Runtime>(
    webview: Webview<R>,
    rid: Option<ResourceId>,
    mirror_download_url: Option<String>,
) -> Result<()> {
    let pending_data = webview.state::<PendingUpdateData>().inner();

    if let Some(url) = mirror_download_url {
        // ── Path A: download from Miawa mirror ──
        // We need version info; extract from the URL or use app version
        let current_version =
            webview.app_handle().package_info().version.to_string();

        // Extract version from the URL path (axolotl/vX.Y.Z/filename)
        let version = url
            .split("/axolotl/")
            .nth(1)
            .and_then(|s| s.split('/').next())
            .unwrap_or("unknown")
            .to_string();

        let data =
            miawa_download_update(&url, &version, &current_version).await?;

        tracing::info!("Mirror update downloaded, storing for installation");
        pending_data
            .0
            .lock()
            .unwrap()
            .replace(PendingUpdateEntry::Mirror {
                data,
                version,
                current_version,
            });

        return Ok(());
    }

    // ── Path B: download via tauri-plugin-updater ──
    let Some(rid) = rid else {
        return Err(theseus::Error::from(theseus::ErrorKind::OtherError(
            "No download source provided: both rid and mirror_download_url are None"
                .to_string(),
        ))
        .into());
    };

    let update = webview.resources_table().get::<Update>(rid)?;

    let progress = init_loading(
        LoadingBarType::LauncherUpdate {
            version: update.version.clone(),
            current_version: update.current_version.clone(),
        },
        1.0,
        "Downloading update...",
    )
    .await?;

    let download_start = Instant::now();
    let update_data = update
        .download(
            |chunk_size, total_size| {
                let Some(total_size) = total_size else {
                    return;
                };
                if let Err(e) = emit_loading(
                    &progress,
                    chunk_size as f64 / total_size as f64,
                    None,
                ) {
                    tracing::error!(
                        "Failed to update download progress bar: {e}"
                    );
                }
            },
            || {},
        )
        .await?;
    let download_duration = download_start.elapsed();
    tracing::info!("Downloaded update in {download_duration:?}");

    pending_data
        .0
        .lock()
        .unwrap()
        .replace(PendingUpdateEntry::Plugin {
            update,
            data: update_data,
        });

    Ok(())
}

#[tauri::command]
pub fn remove_enqueued_update<R: Runtime>(webview: Webview<R>) {
    let pending_data = webview.state::<PendingUpdateData>().inner();
    pending_data.0.lock().unwrap().take();
}
