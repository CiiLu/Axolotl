//! Managed dedicated Minecraft servers: manifests, downloads, and process control.
//! Each server lives in its own directory under the launcher's `servers` folder
//! and is described by an `axolotl-server.json` manifest.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sha1_smol::Sha1;
use std::sync::LazyLock;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use uuid::Uuid;

use crate::event::ServerPayloadType;
use crate::event::emit::emit_server;
use crate::state::{clear_log_buffer, push_log_line, remove_log_buffer};
use crate::util::io::{self, IOError};
use crate::{ErrorKind, Result, State};

const MANIFEST_FILE: &str = "axolotl-server.json";
const DEFAULT_JAR_NAME: &str = "server.jar";
/// Executable launcher jar downloaded from Fabric Meta; must match the
/// filename used by the frontend's `resolveServerJar('fabric')`.
const FABRIC_SERVER_JAR_NAME: &str = "fabric-server.jar";
const DEFAULT_MEMORY_MB: u32 = 2048;
const STOP_TIMEOUT_SECS: u64 = 60;
const DOWNLOAD_PROGRESS_STEP: u64 = 512 * 1024;

fn type_default_jar_name(server_type: &str) -> Option<String> {
    match server_type {
        "fabric" => Some(FABRIC_SERVER_JAR_NAME.to_string()),
        _ => None,
    }
}

/// Resolves the jar a server launches with: the manifest override, then the
/// server type default, then the generic default.
fn resolve_jar_name(manifest: &ServerManifest) -> String {
    manifest
        .jar_name
        .clone()
        .or_else(|| type_default_jar_name(&manifest.server_type))
        .unwrap_or_else(|| DEFAULT_JAR_NAME.to_string())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ServerManifest {
    pub id: String,
    pub name: String,
    pub server_type: String,
    pub game_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loader_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jar_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub java_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_mb: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon_path: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub jvm_args: Vec<String>,
    pub created_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_started_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub last_exit_crashed: bool,
}

#[derive(Serialize, Debug, Clone)]
pub struct ServerInfo {
    #[serde(flatten)]
    pub manifest: ServerManifest,
    pub path: String,
    pub running: bool,
    pub eula_exists: bool,
    pub eula_accepted: bool,
    pub port: Option<u16>,
}

struct ServerProcess {
    child: tokio::sync::Mutex<Child>,
    stdin: tokio::sync::Mutex<ChildStdin>,
    stop_requested: AtomicBool,
}

static SERVER_PROCESSES: LazyLock<DashMap<String, Arc<ServerProcess>>> =
    LazyLock::new(DashMap::new);

pub async fn list() -> Result<Vec<ServerInfo>> {
    let state = State::get().await?;
    let servers_dir = state.directories.servers_dir();
    if !servers_dir.exists() {
        return Ok(Vec::new());
    }

    let mut servers = Vec::new();
    let mut entries = tokio::fs::read_dir(&servers_dir)
        .await
        .map_err(|e| IOError::with_path(e, &servers_dir))?;
    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| IOError::with_path(e, &servers_dir))?
    {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Ok(manifest) = read_manifest(&path).await else {
            continue;
        };
        servers.push(build_server_info(&manifest, &path).await);
    }
    servers.sort_by(|a, b| {
        a.manifest
            .name
            .to_lowercase()
            .cmp(&b.manifest.name.to_lowercase())
    });
    Ok(servers)
}

pub async fn get(server_id: &str) -> Result<ServerInfo> {
    let path = server_path(server_id).await?;
    let manifest = read_manifest(&path).await?;
    Ok(build_server_info(&manifest, &path).await)
}

pub async fn create(
    name: &str,
    server_type: &str,
    game_version: &str,
    loader_version: Option<String>,
    java_path: Option<String>,
    memory_mb: Option<u32>,
) -> Result<ServerManifest> {
    let name = name.trim();
    if name.is_empty() {
        return Err(ErrorKind::InputError(
            "Server name cannot be empty".to_string(),
        )
        .as_error());
    }

    let state = State::get().await?;
    let id = Uuid::new_v4().to_string();
    let dir_name = format!("{}-{}", sanitize_folder_name(name), &id[..8]);
    let dir = state.directories.servers_dir().join(&dir_name);
    io::create_dir_all(&dir).await?;

    let manifest = ServerManifest {
        id: dir_name,
        name: name.to_string(),
        server_type: server_type.to_string(),
        game_version: game_version.to_string(),
        loader_version,
        jar_name: type_default_jar_name(server_type),
        java_path,
        memory_mb,
        icon_path: None,
        jvm_args: Vec::new(),
        created_at: Utc::now(),
        last_started_at: None,
        last_exit_crashed: false,
    };
    write_manifest(&dir, &manifest).await?;
    Ok(manifest)
}

/// Sets or clears the server icon. `None` resets to the default icon.
pub async fn set_icon(
    server_id: &str,
    icon_path: Option<String>,
) -> Result<ServerManifest> {
    let path = server_path(server_id).await?;
    let mut manifest = read_manifest(&path).await?;
    manifest.icon_path = icon_path;
    write_manifest(&path, &manifest).await?;
    Ok(manifest)
}

pub async fn update_settings(
    server_id: &str,
    name: Option<String>,
    java_path: Option<String>,
    memory_mb: Option<u32>,
    jvm_args: Option<Vec<String>>,
) -> Result<ServerManifest> {
    let path = server_path(server_id).await?;
    let mut manifest = read_manifest(&path).await?;
    if let Some(name) = name {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(ErrorKind::InputError(
                "Server name cannot be empty".to_string(),
            )
            .as_error());
        }
        manifest.name = name;
    }
    if let Some(java_path) = java_path {
        manifest.java_path = if java_path.is_empty() {
            None
        } else {
            Some(java_path)
        };
    }
    if let Some(memory_mb) = memory_mb {
        manifest.memory_mb = Some(memory_mb);
    }
    if let Some(jvm_args) = jvm_args {
        manifest.jvm_args = jvm_args;
    }
    write_manifest(&path, &manifest).await?;
    Ok(manifest)
}

pub async fn delete(server_id: &str) -> Result<()> {
    if SERVER_PROCESSES.contains_key(server_id) {
        return Err(ErrorKind::InputError(
            "Stop the server before deleting it".to_string(),
        )
        .as_error());
    }
    let path = server_path(server_id).await?;
    remove_log_buffer(server_id);
    tokio::fs::remove_dir_all(&path)
        .await
        .map_err(|e| IOError::with_path(e, &path))?;
    Ok(())
}

pub async fn read_file(server_id: &str, file: &str) -> Result<String> {
    let path = resolve_server_file(server_id, file).await?;
    let bytes = io::read(&path).await?;
    let text = String::from_utf8_lossy(&bytes).into_owned();
    Ok(text)
}

pub async fn write_file(
    server_id: &str,
    file: &str,
    contents: &str,
) -> Result<()> {
    let path = resolve_server_file(server_id, file).await?;
    io::write(&path, contents).await?;
    Ok(())
}

pub async fn download_file(
    server_id: &str,
    url: &str,
    filename: &str,
    expected_sha1: Option<String>,
) -> Result<()> {
    let dir = server_path(server_id).await?;
    let destination = safe_join(&dir, filename)?;
    let partial = destination.with_extension("part");

    let client = reqwest::Client::builder()
        .user_agent(crate::launcher_user_agent())
        .build()
        .map_err(|e| ErrorKind::NetworkError(e.to_string()))?;
    let response = client
        .get(url)
        .send()
        .await
        .and_then(|r| r.error_for_status())?;
    let total = response.content_length();
    let mut stream = response.bytes_stream();

    let mut file = tokio::fs::File::create(&partial)
        .await
        .map_err(|e| IOError::with_path(e, &partial))?;
    let mut hasher = Sha1::new();
    let mut downloaded: u64 = 0;
    let mut last_reported: u64 = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
            .await
            .map_err(|e| IOError::with_path(e, &partial))?;
        hasher.update(&chunk);
        downloaded += chunk.len() as u64;
        if downloaded - last_reported >= DOWNLOAD_PROGRESS_STEP {
            last_reported = downloaded;
            emit_server(
                server_id,
                ServerPayloadType::DownloadProgress { downloaded, total },
            )
            .await
            .ok();
        }
    }
    drop(file);

    if let Some(expected) = expected_sha1.as_deref() {
        let actual = hasher.digest().to_string();
        if !actual.eq_ignore_ascii_case(expected) {
            let _ = tokio::fs::remove_file(&partial).await;
            return Err(ErrorKind::NetworkError(format!(
				"Download checksum mismatch for {filename}: expected {expected}, got {actual}"
			))
			.as_error());
        }
    }

    tokio::fs::rename(&partial, &destination)
        .await
        .map_err(|e| IOError::with_path(e, &destination))?;
    emit_server(
        server_id,
        ServerPayloadType::DownloadProgress {
            downloaded,
            total: Some(downloaded.max(total.unwrap_or(0))),
        },
    )
    .await
    .ok();
    Ok(())
}

pub async fn start(
    server_id: &str,
    java_path: Option<String>,
    memory_mb: Option<u32>,
    jvm_args: Option<Vec<String>>,
) -> Result<()> {
    if SERVER_PROCESSES.contains_key(server_id) {
        return Err(ErrorKind::InputError(
            "Server is already running".to_string(),
        )
        .as_error());
    }

    let dir = server_path(server_id).await?;
    let mut manifest = read_manifest(&dir).await?;
    let jar_name = resolve_jar_name(&manifest);
    let jar_path = dir.join(&jar_name);
    if !jar_path.exists() {
        return Err(ErrorKind::LauncherError(format!(
            "Server jar not found: {jar_name}. Download the server files first."
        ))
        .as_error());
    }

    let java = java_path
        .or_else(|| manifest.java_path.clone())
        .unwrap_or_else(|| "java".to_string());
    let memory = memory_mb
        .or(manifest.memory_mb)
        .unwrap_or(DEFAULT_MEMORY_MB);

    let mut command = Command::new(&java);
    command.arg(format!("-Xmx{memory}M"));
    for arg in jvm_args.unwrap_or_else(|| manifest.jvm_args.clone()) {
        command.arg(arg);
    }
    command.arg("-jar").arg(&jar_name).arg("nogui");
    command.current_dir(&dir);
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());
    command.stdin(std::process::Stdio::piped());
    command.kill_on_drop(true);

    let mut child = command.spawn().map_err(|e| {
        ErrorKind::LauncherError(format!("Failed to start server process: {e}"))
            .as_error()
    })?;
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdin = child.stdin.take();

    manifest.last_started_at = Some(Utc::now());
    manifest.last_exit_crashed = false;
    write_manifest(&dir, &manifest).await?;

    clear_log_buffer(server_id);
    let process = Arc::new(ServerProcess {
        child: tokio::sync::Mutex::new(child),
        stdin: tokio::sync::Mutex::new(stdin.ok_or_else(|| {
            ErrorKind::LauncherError(
                "Server stdin could not be captured".to_string(),
            )
            .as_error()
        })?),
        stop_requested: AtomicBool::new(false),
    });
    SERVER_PROCESSES.insert(server_id.to_string(), process.clone());

    if let Some(stdout) = stdout {
        tokio::spawn(stream_server_output(server_id.to_string(), stdout));
    }
    if let Some(stderr) = stderr {
        tokio::spawn(stream_server_output(server_id.to_string(), stderr));
    }
    tokio::spawn(monitor_server_process(server_id.to_string(), dir, process));

    emit_server(server_id, ServerPayloadType::Started)
        .await
        .ok();
    Ok(())
}

pub async fn send_command(server_id: &str, command: &str) -> Result<()> {
    let process = SERVER_PROCESSES
        .get(server_id)
        .map(|entry| entry.value().clone())
        .ok_or_else(|| {
            ErrorKind::InputError("Server is not running".to_string())
                .as_error()
        })?;
    let mut stdin = process.stdin.lock().await;
    stdin
        .write_all(format!("{command}\n").as_bytes())
        .await
        .map_err(|e| {
            ErrorKind::LauncherError(format!("Failed to send command: {e}"))
                .as_error()
        })?;
    stdin.flush().await.map_err(|e| {
        ErrorKind::LauncherError(format!("Failed to send command: {e}"))
            .as_error()
    })?;
    Ok(())
}

pub async fn stop(server_id: &str) -> Result<()> {
    let process = SERVER_PROCESSES
        .get(server_id)
        .map(|entry| entry.value().clone())
        .ok_or_else(|| {
            ErrorKind::InputError("Server is not running".to_string())
                .as_error()
        })?;
    process.stop_requested.store(true, Ordering::SeqCst);
    let mut stdin = process.stdin.lock().await;
    let _ = stdin.write_all(b"stop\n").await;
    let _ = stdin.flush().await;

    let watchdog = process.clone();
    let server_id = server_id.to_string();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(STOP_TIMEOUT_SECS))
            .await;
        if let Some(current) = SERVER_PROCESSES.get(&server_id)
            && current.stop_requested.load(Ordering::SeqCst)
        {
            let _ = watchdog.child.lock().await.kill().await;
        }
    });
    Ok(())
}

pub async fn kill(server_id: &str) -> Result<()> {
    let process = SERVER_PROCESSES
        .get(server_id)
        .map(|entry| entry.value().clone())
        .ok_or_else(|| {
            ErrorKind::InputError("Server is not running".to_string())
                .as_error()
        })?;
    process.stop_requested.store(true, Ordering::SeqCst);
    let mut child = process.child.lock().await;
    child.kill().await?;
    Ok(())
}

pub async fn kill_port_process(port: u16) -> Result<()> {
    let pids = port_listener_pids(port).await?;
    if pids.is_empty() {
        return Err(ErrorKind::InputError(format!(
            "No process found listening on port {port}"
        ))
        .as_error());
    }
    for pid in pids {
        force_terminate_pid(pid).await?;
    }
    Ok(())
}

#[derive(Serialize, Debug, Clone)]
pub struct PortProcessInfo {
    pub pid: u32,
    pub name: Option<String>,
}

/// Returns the first process listening on the given TCP port, if any.
pub async fn port_process(port: u16) -> Result<Option<PortProcessInfo>> {
    let pids = port_listener_pids(port).await?;
    let Some(&pid) = pids.first() else {
        return Ok(None);
    };
    Ok(Some(PortProcessInfo {
        pid,
        name: process_name(pid).await,
    }))
}

#[cfg(not(target_os = "windows"))]
async fn port_listener_pids(port: u16) -> Result<Vec<u32>> {
    let output = Command::new("lsof")
        .args(["-t", "-i", &format!("tcp:{port}"), "-s", "tcp:listen"])
        .output()
        .await
        .map_err(|e| {
            ErrorKind::LauncherError(format!(
                "Failed to look up processes listening on port {port}: {e}"
            ))
            .as_error()
        })?;
    let mut pids: Vec<u32> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.trim().parse::<u32>().ok())
        .collect();
    pids.sort_unstable();
    pids.dedup();
    Ok(pids)
}

#[cfg(not(target_os = "windows"))]
async fn force_terminate_pid(pid: u32) -> Result<()> {
    let output = Command::new("kill")
        .args(["-9", &pid.to_string()])
        .output()
        .await
        .map_err(|e| {
            ErrorKind::LauncherError(format!(
                "Failed to terminate process {pid}: {e}"
            ))
            .as_error()
        })?;
    if !output.status.success() {
        return Err(ErrorKind::LauncherError(format!(
            "Failed to terminate process {pid}"
        ))
        .as_error());
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
async fn process_name(pid: u32) -> Option<String> {
    let output = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "comm="])
        .output()
        .await
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!name.is_empty()).then_some(name)
}

#[cfg(target_os = "windows")]
async fn port_listener_pids(port: u16) -> Result<Vec<u32>> {
    let output = Command::new("netstat")
        .args(["-ano", "-p", "tcp"])
        .output()
        .await
        .map_err(|e| {
            ErrorKind::LauncherError(format!(
                "Failed to look up processes listening on port {port}: {e}"
            ))
            .as_error()
        })?;
    let mut pids: Vec<u32> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let columns: Vec<&str> = line.split_whitespace().collect();
            if columns.len() < 5
                || !columns[3].eq_ignore_ascii_case("LISTENING")
            {
                return None;
            }
            let local_address = columns[1];
            let local_port = local_address.rsplit(':').next()?;
            (local_port == port.to_string())
                .then(|| columns[4].parse::<u32>().ok())?
        })
        .collect();
    pids.sort_unstable();
    pids.dedup();
    Ok(pids)
}

#[cfg(target_os = "windows")]
async fn force_terminate_pid(pid: u32) -> Result<()> {
    let output = Command::new("taskkill")
        .args(["/F", "/PID", &pid.to_string()])
        .output()
        .await
        .map_err(|e| {
            ErrorKind::LauncherError(format!(
                "Failed to terminate process {pid}: {e}"
            ))
            .as_error()
        })?;
    if !output.status.success() {
        return Err(ErrorKind::LauncherError(format!(
            "Failed to terminate process {pid}"
        ))
        .as_error());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
async fn process_name(pid: u32) -> Option<String> {
    let output = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .output()
        .await
        .ok()?;
    let line = String::from_utf8_lossy(&output.stdout).lines().next()?;
    if line.starts_with("INFO") {
        return None;
    }
    let name = line.split(',').next()?.trim_matches('"').to_string();
    (!name.is_empty()).then_some(name)
}

pub async fn get_log_buffer(server_id: &str) -> Result<Vec<String>> {
    Ok(crate::state::get_log_buffer(server_id))
}

pub async fn clear_log(server_id: &str) -> Result<()> {
    clear_log_buffer(server_id);
    Ok(())
}

async fn stream_server_output(
    server_id: String,
    reader: impl tokio::io::AsyncRead + Unpin,
) {
    let mut buf_reader = BufReader::new(reader);
    let mut line = String::new();
    loop {
        line.clear();
        match buf_reader.read_line(&mut line).await {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let trimmed = line.trim_end_matches(['\r', '\n']);
                let cleaned = strip_ansi(trimmed);
                if cleaned.is_empty() {
                    continue;
                }
                push_log_line(&server_id, cleaned.clone());
                emit_server(
                    &server_id,
                    ServerPayloadType::Log { line: cleaned },
                )
                .await
                .ok();
            }
        }
    }
}

/// Removes ANSI escape sequences (SGR colors, cursor control, OSC titles) that
/// servers emit when they assume an interactive terminal is attached.
fn strip_ansi(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.char_indices().peekable();
    while let Some((_, character)) = chars.next() {
        if character != '\u{1b}' {
            output.push(character);
            continue;
        }
        match chars.peek().map(|&(_, c)| c) {
            // CSI sequence: parameter bytes, then a final byte in @..~
            Some('[') => {
                chars.next();
                for (_, c) in chars.by_ref() {
                    if ('\u{40}'..='\u{7e}').contains(&c) {
                        break;
                    }
                }
            }
            // OSC sequence: terminated by BEL or ST (ESC \)
            Some(']') => {
                chars.next();
                let mut saw_escape = false;
                for (_, c) in chars.by_ref() {
                    if c == '\u{7}' || (saw_escape && c == '\\') {
                        break;
                    }
                    saw_escape = c == '\u{1b}';
                }
            }
            // Stray escape byte without a recognized sequence
            _ => {}
        }
    }
    output
}

async fn monitor_server_process(
    server_id: String,
    dir: PathBuf,
    process: Arc<ServerProcess>,
) {
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        let exit_status = {
            let mut child = process.child.lock().await;
            match child.try_wait() {
                Ok(Some(status)) => Some(status),
                Ok(None) => continue,
                Err(_) => None,
            }
        };

        SERVER_PROCESSES.remove(&server_id);
        let stop_requested = process.stop_requested.load(Ordering::SeqCst);
        let eula_accepted = read_eula_accepted(&dir).await;
        let crashed = exit_status
            .map(|status| !status.success() && !stop_requested && eula_accepted)
            .unwrap_or(false);

        if let Ok(mut manifest) = read_manifest(&dir).await {
            manifest.last_exit_crashed = crashed;
            let _ = write_manifest(&dir, &manifest).await;
        }

        emit_server(&server_id, ServerPayloadType::Stopped { crashed })
            .await
            .ok();
        return;
    }
}

async fn read_eula_accepted(dir: &Path) -> bool {
    match tokio::fs::read_to_string(dir.join("eula.txt")).await {
        Ok(text) => text
            .lines()
            .find_map(|line| line.split_once('='))
            .filter(|(key, _)| key.trim() == "eula")
            .is_some_and(|(_, value)| {
                value.trim().eq_ignore_ascii_case("true")
            }),
        Err(_) => false,
    }
}

async fn build_server_info(
    manifest: &ServerManifest,
    path: &Path,
) -> ServerInfo {
    let eula_text = tokio::fs::read_to_string(path.join("eula.txt"))
        .await
        .unwrap_or_default();
    let eula_exists = !eula_text.is_empty();
    let eula_accepted = eula_text
        .lines()
        .find_map(|line| line.split_once('='))
        .filter(|(key, _)| key.trim() == "eula")
        .is_some_and(|(_, value)| value.trim().eq_ignore_ascii_case("true"));
    let port = tokio::fs::read_to_string(path.join("server.properties"))
        .await
        .ok()
        .and_then(|text| {
            text.lines().find_map(|line| {
                let (key, value) = line.split_once('=')?;
                (key.trim() == "server-port")
                    .then(|| value.trim().parse::<u16>().ok())?
            })
        });
    ServerInfo {
        manifest: manifest.clone(),
        path: path.to_string_lossy().into_owned(),
        running: SERVER_PROCESSES.contains_key(&manifest.id),
        eula_exists,
        eula_accepted,
        port,
    }
}

async fn server_path(server_id: &str) -> Result<PathBuf> {
    if server_id.contains(['/', '\\'])
        || server_id.contains("..")
        || server_id.is_empty()
    {
        return Err(ErrorKind::InputError(format!(
            "Invalid server id: {server_id}"
        ))
        .as_error());
    }
    let state = State::get().await?;
    let path = state.directories.server_dir(server_id);
    if !path.is_dir() {
        return Err(ErrorKind::InputError(format!(
            "Unknown server: {server_id}"
        ))
        .as_error());
    }
    Ok(path)
}

async fn resolve_server_file(server_id: &str, file: &str) -> Result<PathBuf> {
    let dir = server_path(server_id).await?;
    safe_join(&dir, file)
}

fn safe_join(dir: &Path, file: &str) -> Result<PathBuf> {
    if file.is_empty()
        || file.contains('\\')
        || file.starts_with('/')
        || file
            .split('/')
            .any(|segment| segment == ".." || segment.is_empty())
    {
        return Err(ErrorKind::InputError(format!(
            "Invalid file name: {file}"
        ))
        .as_error());
    }
    Ok(dir.join(file))
}

async fn read_manifest(dir: &Path) -> Result<ServerManifest> {
    let bytes = io::read(dir.join(MANIFEST_FILE)).await?;
    serde_json::from_slice(&bytes).map_err(|e| {
        ErrorKind::FSError(format!("Failed to parse server manifest: {e}"))
            .as_error()
    })
}

async fn write_manifest(dir: &Path, manifest: &ServerManifest) -> Result<()> {
    let contents = serde_json::to_string_pretty(manifest)?;
    io::write(dir.join(MANIFEST_FILE), contents).await?;
    Ok(())
}

fn sanitize_folder_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = sanitized.trim_matches('-');
    if trimmed.is_empty() {
        "server".to_string()
    } else {
        trimmed.chars().take(32).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_folder_name_replaces_unsafe_characters() {
        assert_eq!(sanitize_folder_name("My Server!"), "My-Server");
        assert_eq!(sanitize_folder_name("../etc"), "etc");
        assert_eq!(sanitize_folder_name("///"), "server");
    }

    #[test]
    fn safe_join_rejects_traversal() {
        let dir = Path::new("/tmp/servers/a");
        assert!(safe_join(dir, "server.properties").is_ok());
        assert!(safe_join(dir, "../secret").is_err());
        assert!(safe_join(dir, "/etc/passwd").is_err());
        assert!(safe_join(dir, "a//b").is_err());
        assert!(safe_join(dir, "").is_err());
    }

    #[test]
    fn server_manifest_round_trips() {
        let manifest = ServerManifest {
            id: "test-12345678".to_string(),
            name: "Test".to_string(),
            server_type: "vanilla".to_string(),
            game_version: "1.21.4".to_string(),
            loader_version: None,
            jar_name: None,
            java_path: None,
            memory_mb: Some(2048),
            icon_path: None,
            jvm_args: Vec::new(),
            created_at: Utc::now(),
            last_started_at: None,
            last_exit_crashed: false,
        };
        let json = serde_json::to_string(&manifest).unwrap();
        let parsed: ServerManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, manifest.id);
        assert_eq!(parsed.name, manifest.name);
    }

    #[test]
    fn resolves_jar_name_from_type_then_manifest() {
        let mut manifest = ServerManifest {
            id: "test-12345678".to_string(),
            name: "Test".to_string(),
            server_type: "fabric".to_string(),
            game_version: "26.2".to_string(),
            loader_version: Some("0.19.3".to_string()),
            jar_name: None,
            java_path: None,
            memory_mb: None,
            icon_path: None,
            jvm_args: Vec::new(),
            created_at: Utc::now(),
            last_started_at: None,
            last_exit_crashed: false,
        };
        assert_eq!(resolve_jar_name(&manifest), FABRIC_SERVER_JAR_NAME);

        manifest.server_type = "vanilla".to_string();
        assert_eq!(resolve_jar_name(&manifest), DEFAULT_JAR_NAME);

        manifest.jar_name = Some("custom.jar".to_string());
        assert_eq!(resolve_jar_name(&manifest), "custom.jar");
    }

    #[test]
    fn strips_ansi_escape_sequences_from_server_output() {
        let line = "[16:02:30 INFO]: \u{1b}[38;2;255;170;0m/mspt: \u{1b}[38;2;255;255;255mView server tick times\u{1b}[0m";
        assert_eq!(
            strip_ansi(line),
            "[16:02:30 INFO]: /mspt: View server tick times"
        );

        assert_eq!(strip_ansi("\u{1b}]0;Server console\u{7}ready"), "ready");
        assert_eq!(strip_ansi("\u{1b}]0;Server console\u{1b}\\done"), "done");
        assert_eq!(strip_ansi("plain text stays"), "plain text stays");
        assert_eq!(strip_ansi("h\u{e9}llo \u{1b}[31mred"), "h\u{e9}llo red");
    }
}
