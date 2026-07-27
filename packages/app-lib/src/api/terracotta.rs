use eyre::{Context, bail};
use tokio::sync::Mutex;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::LazyLock;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::process::{Child, Command};
use tokio::sync::oneshot;
use tracing::{error, info, warn};
use flate2::read::GzDecoder;
use futures::StreamExt;
use tokio::io::AsyncWriteExt;

static TERRACOTTA_STATE: LazyLock<Mutex<TerracottaState>> =
	LazyLock::new(|| Mutex::new(TerracottaState::default()));

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TerracottaState {
	pub status: TerracottaStatus,
	pub http_port: Option<u16>,
	pub room_code: Option<String>,
	pub server_port: Option<u16>,
	pub players: Vec<PlayerInfo>,
	pub download_progress: Option<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerracottaStatus {
	Idle,
	Starting,
	Downloading,
	HostScanning,
	HostReady,
	GuestConnecting,
	GuestReady,
	Error,
}

impl Default for TerracottaStatus {
	fn default() -> Self {
		Self::Idle
	}
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerInfo {
	pub machine_id: String,
	pub name: String,
	pub vendor: String,
	pub kind: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TerracottaMeta {
	pub version: String,
	pub compile_timestamp: String,
	pub easytier_version: String,
	pub yggdrasil_port: u16,
	pub target_tuple: String,
	pub target_os: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TerracottaApiState {
	state: String,
	#[serde(default)]
	index: Option<u32>,
	#[serde(default)]
	room: Option<String>,
	#[serde(default)]
	profile_index: Option<u32>,
	#[serde(default)]
	profiles: Option<Vec<TerracottaApiProfile>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TerracottaApiProfile {
	machine_id: String,
	name: String,
	vendor: String,
	kind: String,
}

struct TerracottaProcess {
	child: Child,
	abort_tx: Option<oneshot::Sender<()>>,
}

impl Drop for TerracottaProcess {
	fn drop(&mut self) {
		if let Some(tx) = self.abort_tx.take() {
			let _ = tx.send(());
		}
		let _ = self.child.start_kill();
	}
}

static PROCESS: LazyLock<Mutex<Option<TerracottaProcess>>> =
	LazyLock::new(|| Mutex::new(None));

pub fn terracotta_platform_key() -> &'static str {
	match (std::env::consts::OS, std::env::consts::ARCH) {
		("linux", "x86_64") => "linux-x86_64",
		("linux", "aarch64") => "linux-arm64",
		("linux", "riscv64") => "linux-riscv64",
		("linux", "loongarch64") => "linux-loongarch64",
		("macos", "x86_64") => "macos-x86_64",
		("macos", "aarch64") => "macos-arm64",
		("windows", "x86_64") => "windows-x86_64",
		("windows", "aarch64") => "windows-arm64",
		("freebsd", "x86_64") => "freebsd-x86_64",
		_ => "unsupported",
	}
}

fn terracotta_binary_name() -> &'static str {
	if cfg!(target_os = "windows") {
		"terracotta.exe"
	} else {
		"terracotta"
	}
}

fn terracotta_binary_path() -> PathBuf {
	let name = terracotta_binary_name();
	std::env::current_exe()
		.ok()
		.and_then(|p| p.parent().map(|d| d.to_path_buf()))
		.unwrap_or_else(|| PathBuf::from("."))
		.join("terracotta")
		.join(name)
}

fn terracotta_download_url(version: &str, platform: &str) -> String {
	format!(
		"https://github.com/burningtnt/Terracotta/releases/download/v{version}/terracotta-{version}-{platform}-pkg.tar.gz"
	)
}

fn terracotta_mirror_url(version: &str, platform: &str) -> String {
	format!(
		"https://gitee.com/burningtnt/Terracotta/releases/download/v{version}/terracotta-{version}-{platform}-pkg.tar.gz"
	)
}

fn resolve_terracotta_binary_path(bin_path: &PathBuf) -> PathBuf {
	let resolved_path = if bin_path.is_absolute() {
		bin_path.clone()
	} else {
		std::env::current_dir()
			.unwrap_or_else(|_| PathBuf::from("."))
			.join(bin_path)
	};

	if resolved_path.exists() && resolved_path.is_file() {
		resolved_path
	} else if resolved_path.is_dir() {
		resolved_path.join(terracotta_binary_name())
	} else {
		let bin_name = terracotta_binary_name();
		let parent_path = resolved_path
			.parent()
			.map(|p| p.join(bin_name))
			.unwrap_or_else(|| PathBuf::from(bin_name));
		resolved_path.with_file_name(bin_name)
	}
}

async fn get_latest_terracotta_version() -> eyre::Result<String> {
	let response = crate::util::fetch::INSECURE_REQWEST_CLIENT
		.get("https://api.github.com/repos/burningtnt/Terracotta/releases/latest")
		.header("Accept", "application/vnd.github+json")
		.header("User-Agent", crate::launcher_user_agent())
		.header("X-GitHub-Api-Version", "2022-11-28")
		.send()
		.await
		.wrap_err("failed to fetch latest terracotta release info")?;

	#[derive(Deserialize)]
	struct ReleaseInfo {
		tag_name: String,
	}

	let info: ReleaseInfo = response
		.json()
		.await
		.wrap_err("failed to parse terracotta release info")?;

	Ok(info.tag_name.trim_start_matches('v').to_string())
}

pub async fn download_terracotta(version: Option<String>) -> eyre::Result<()> {
	{
		let mut state = TERRACOTTA_STATE.lock().await;
		state.status = TerracottaStatus::Downloading;
		state.download_progress = Some(0);
	}

	let version = match version {
		Some(v) => v,
		None => get_latest_terracotta_version().await?,
	};

	let platform = terracotta_platform_key();
	if platform == "unsupported" {
		bail!(
			"no terracotta binary available for {}/{}",
			std::env::consts::OS,
			std::env::consts::ARCH
		);
	}

	let primary_url = terracotta_download_url(&version, platform);
	let mirror_url = terracotta_mirror_url(&version, platform);
	info!("downloading terracotta v{version} for {platform}");

	let response = match crate::util::fetch::INSECURE_REQWEST_CLIENT
		.get(&primary_url)
		.send()
		.await
	{
		Ok(r) if r.status().is_success() => r,
		Ok(r) => {
			warn!(
				"primary download returned HTTP {}, trying mirror",
				r.status()
			);
			crate::util::fetch::INSECURE_REQWEST_CLIENT
				.get(&mirror_url)
				.send()
				.await
				.wrap_err_with(|| format!("failed to download terracotta from mirror {mirror_url}"))?
		}
		Err(e) => {
			warn!("primary download failed: {e:#}, trying mirror");
			crate::util::fetch::INSECURE_REQWEST_CLIENT
				.get(&mirror_url)
				.send()
				.await
				.wrap_err_with(|| format!("failed to download terracotta from mirror {mirror_url}"))?
		}
	};

	if !response.status().is_success() {
		bail!(
			"terracotta download returned HTTP {}",
			response.status()
		);
	}

	let total_size = response.content_length().unwrap_or(0);
	let archive_path = std::env::temp_dir().join(format!(
		"terracotta-{version}-{platform}.tar.gz"
	));

	let mut file = tokio::fs::File::create(&archive_path)
		.await
		.wrap_err("failed to create temp file for terracotta download")?;

	let mut stream = response.bytes_stream();
	let mut downloaded: u64 = 0;

	while let Some(chunk) = stream.next().await {
		let chunk = chunk.wrap_err("download stream error")?;
		file.write_all(&chunk).await?;
		downloaded += chunk.len() as u64;

		if total_size > 0 {
			let pct = ((downloaded as f64 / total_size as f64) * 100.0) as u8;
			let mut state = TERRACOTTA_STATE.lock().await;
			state.download_progress = Some(pct);
		}
	}

	file.flush().await?;
	drop(file);

	info!("downloaded terracotta ({downloaded} bytes), extracting...");

	let target_dir = terracotta_binary_path()
		.parent()
		.map(|p| p.to_path_buf())
		.unwrap_or_else(|| PathBuf::from("terracotta"));

	tokio::fs::create_dir_all(&target_dir)
		.await
		.wrap_err("failed to create terracotta directory")?;

	let target_dir_clone = target_dir.clone();
	let archive_path_clone = archive_path.clone();
	tokio::task::spawn_blocking(move || -> eyre::Result<()> {
		let archive_file = std::fs::File::open(&archive_path_clone)
			.wrap_err("failed to open downloaded archive")?;
		let decoder = GzDecoder::new(archive_file);
		let mut archive = tar::Archive::new(decoder);

		for entry in archive.entries()? {
			let mut entry = entry?;
			let path = entry.path()?.to_path_buf();
			if let Some(name) = path.file_name() {
				if name == terracotta_binary_name() {
					entry.unpack_in(&target_dir_clone)?;
					return Ok(());
				}
			}
		}
		bail!("terracotta binary not found in downloaded archive")
	})
	.await??;

	let _ = tokio::fs::remove_file(&archive_path).await;

	#[cfg(unix)]
	{
		let bin_path = target_dir.join(terracotta_binary_name());
		if bin_path.exists() {
			use std::os::unix::fs::PermissionsExt;
			let metadata = std::fs::metadata(&bin_path)?;
			let mut perms = metadata.permissions();
			perms.set_mode(0o755);
			std::fs::set_permissions(&bin_path, perms)?;
		}
	}

	info!("terracotta v{version} installed to {}", target_dir.display());

	let mut state = TERRACOTTA_STATE.lock().await;
	state.status = TerracottaStatus::Idle;
	state.download_progress = None;
	Ok(())
}

async fn terracotta_client(
	port: u16,
) -> eyre::Result<&'static reqwest::Client> {
	static CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
		reqwest::Client::builder()
			.no_proxy()
			.build()
			.expect("terracotta client should build")
	});
	Ok(&CLIENT)
}

async fn terracotta_get<T: serde::de::DeserializeOwned>(
	port: u16,
	path: &str,
) -> eyre::Result<T> {
	let client = terracotta_client(port).await?;
	let url = format!("http://127.0.0.1:{port}{path}");
	let resp = client
		.get(&url)
		.send()
		.await
		.wrap_err_with(|| format!("failed to connect to terracotta at {url}"))?;
	let status = resp.status();
	let body = resp
		.text()
		.await
		.wrap_err("failed to read terracotta response")?;
	if !status.is_success() {
		bail!(
			"terracotta returned {} for {}: {}",
			status.as_u16(),
			path,
			body
		);
	}
	let parsed: T = serde_json::from_str(&body)
		.wrap_err_with(|| format!("failed to parse terracotta response for {path}: {body}"))?;
	Ok(parsed)
}

async fn poll_terracotta_state(port: u16) {
	loop {
		tokio::time::sleep(std::time::Duration::from_millis(500)).await;
		match terracotta_get::<TerracottaApiState>(port, "/state").await {
			Ok(api_state) => {
				let mut state = TERRACOTTA_STATE.lock().await;
				state.http_port = Some(port);
				state.room_code = api_state.room.clone();
				state.status = match api_state.state.as_str() {
					"host-ok" => TerracottaStatus::HostReady,
					"host-scanning" | "host-starting" => {
						TerracottaStatus::HostScanning
					}
					"guest-ok" => TerracottaStatus::GuestReady,
					"guest-connecting" | "guest-starting" => {
						TerracottaStatus::GuestConnecting
					}
					"exception" => TerracottaStatus::Error,
					_ => TerracottaStatus::Idle,
				};
				if let Some(profiles) = api_state.profiles {
					state.players = profiles
						.into_iter()
						.map(|p| PlayerInfo {
							machine_id: p.machine_id,
							name: p.name,
							vendor: p.vendor,
							kind: p.kind,
						})
						.collect();
				}
			}
			Err(e) => {
				warn!("failed to poll terracotta state: {e:#}");
				let mut state = TERRACOTTA_STATE.lock().await;
				state.status = TerracottaStatus::Error;
				break;
			}
		}
	}
}

pub async fn get_state() -> TerracottaState {
	TERRACOTTA_STATE.lock().await.clone()
}

pub async fn get_meta() -> eyre::Result<TerracottaMeta> {
	let state = TERRACOTTA_STATE.lock().await;
	let port = state
		.http_port
		.ok_or_else(|| eyre::eyre!("terracotta is not running"))?;
	drop(state);
	terracotta_get::<TerracottaMeta>(port, "/meta").await
}

pub async fn start_terracotta(
	binary_path: Option<String>,
	auto_download: bool,
) -> eyre::Result<()> {
	let bin_path = binary_path
		.map(PathBuf::from)
		.unwrap_or_else(terracotta_binary_path);

	let mut final_path = resolve_terracotta_binary_path(&bin_path);

	if !final_path.exists() && auto_download {
		info!("terracotta binary not found, attempting auto-download");
		download_terracotta(None).await?;
		final_path = resolve_terracotta_binary_path(&bin_path);
	}

	if !final_path.exists() {
		bail!(
			"terracotta binary not found at {} (platform: {}, expected name: {})",
			final_path.display(),
			terracotta_platform_key(),
			terracotta_binary_name()
		);
	}

	let mut process_guard = PROCESS.lock().await;
	if process_guard.is_some() {
		bail!("terracotta is already running");
	}

	let temp_dir = std::env::temp_dir();
	let port_file = temp_dir.join(format!(
		"terracotta_port_{}.json",
		std::process::id()
	));

	let mut child = Command::new(&final_path)
		.arg("--hmcl")
		.arg(port_file.to_str().unwrap())
		.stdin(Stdio::null())
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.kill_on_drop(true)
		.spawn()
		.wrap_err_with(|| {
			format!(
				"failed to start terracotta at {}",
				final_path.display()
			)
		})?;

	let (abort_tx, abort_rx) = oneshot::channel();

	let stderr = child.stderr.take();
	let pid = child.id().unwrap_or(0);

	if let Some(stderr) = stderr {
		tokio::spawn(async move {
			let reader = BufReader::new(stderr);
			let mut lines = reader.lines();
			let mut abort_rx = abort_rx;
			loop {
				tokio::select! {
					_ = &mut abort_rx => break,
					line = lines.next_line() => {
						match line {
							Ok(Some(text)) => {
								tracing::debug!(target: "terracotta", pid = pid, "{text}");
							}
							_ => break,
						}
					}
				}
			}
		});
	}

	let pid = child.id().unwrap_or(0);
	info!("started terracotta (pid {pid})");

	*process_guard = Some(TerracottaProcess {
		child,
		abort_tx: Some(abort_tx),
	});
	drop(process_guard);

	{
		let mut state = TERRACOTTA_STATE.lock().await;
		state.status = TerracottaStatus::Starting;
	}

	tokio::time::sleep(std::time::Duration::from_millis(1000)).await;

	for i in 0..20 {
		tokio::time::sleep(std::time::Duration::from_millis(500)).await;
		match std::fs::read_to_string(&port_file) {
			Ok(contents) => {
				#[derive(Deserialize)]
				struct PortInfo {
					port: u16,
				}
				let port_info: PortInfo = serde_json::from_str(&contents)
					.wrap_err("failed to parse terracotta port file")?;
				let _ = std::fs::remove_file(&port_file);

				info!(
					"terracotta started on port {}",
					port_info.port
				);

				let port = port_info.port;
				tokio::spawn(poll_terracotta_state(port));

				let mut state = TERRACOTTA_STATE.lock().await;
				state.http_port = Some(port);
				return Ok(());
			}
			Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
				if i == 0 {
					info!(
						"waiting for terracotta port file at {}",
						port_file.display()
					);
				}
			}
			Err(e) => {
				warn!("failed to read terracotta port file: {e}");
			}
		}
	}

	bail!("timed out waiting for terracotta to start");
}

pub async fn stop_terracotta() -> eyre::Result<()> {
	let mut process_guard = PROCESS.lock().await;
	if let Some(mut process) = process_guard.take() {
		if let Some(tx) = process.abort_tx.take() {
			let _ = tx.send(());
		}
		process.child.start_kill().ok();
		info!("stopped terracotta");
	}
	let mut state = TERRACOTTA_STATE.lock().await;
	*state = TerracottaState::default();
	Ok(())
}

pub async fn start_hosting(
	room_code: Option<String>,
	player_name: String,
) -> eyre::Result<()> {
	let state = TERRACOTTA_STATE.lock().await;
	let port = state
		.http_port
		.ok_or_else(|| eyre::eyre!("terracotta is not running"))?;
	drop(state);

	let room_param = room_code
		.as_deref()
		.unwrap_or("");

	let path = format!(
		"/state/scanning?room={}&player={}&public_nodes=",
		urlencoding::encode(room_param),
		urlencoding::encode(&player_name),
	);

	let client = terracotta_client(port).await?;
	let url = format!("http://127.0.0.1:{port}{path}");
	let resp = client.get(&url).send().await.wrap_err_with(|| {
		format!("failed to send hosting request to terracotta at {url}")
	})?;
	let status = resp.status();
	if !status.is_success() {
		let body = resp.text().await.unwrap_or_default();
		bail!(
			"terracotta hosting failed with status {}: {body}",
			status.as_u16()
		);
	}
	Ok(())
}

pub async fn start_joining(
	room_code: String,
	player_name: String,
) -> eyre::Result<()> {
	let state = TERRACOTTA_STATE.lock().await;
	let port = state
		.http_port
		.ok_or_else(|| eyre::eyre!("terracotta is not running"))?;
	drop(state);

	let path = format!(
		"/state/guesting?room={}&player={}&public_nodes=",
		urlencoding::encode(&room_code),
		urlencoding::encode(&player_name),
	);

	let client = terracotta_client(port).await?;
	let url = format!("http://127.0.0.1:{port}{path}");
	let resp = client.get(&url).send().await.wrap_err_with(|| {
		format!("failed to send joining request to terracotta at {url}")
	})?;
	let status = resp.status();
	if !status.is_success() {
		let body = resp.text().await.unwrap_or_default();
		bail!(
			"terracotta joining failed with status {}: {body}",
			status.as_u16()
		);
	}
	Ok(())
}

pub async fn reset_state() -> eyre::Result<()> {
	let state = TERRACOTTA_STATE.lock().await;
	let port = state
		.http_port
		.ok_or_else(|| eyre::eyre!("terracotta is not running"))?;
	drop(state);

	terracotta_get::<serde_json::Value>(port, "/state/ide").await?;

	let mut state = TERRACOTTA_STATE.lock().await;
	state.status = TerracottaStatus::Idle;
	state.room_code = None;
	state.server_port = None;
	state.players.clear();
	Ok(())
}

pub async fn parse_room_code(code: &str) -> eyre::Result<String> {
	if code.starts_with("U/") || code.starts_with("u/") {
		let inner = &code[2..];
		if inner.len() == 19 && inner.chars().filter(|&c| c == '-').count() == 3 {
			let segments: Vec<&str> = inner.split('-').collect();
			if segments.len() == 4
				&& segments.iter().all(|s| s.len() == 4)
				&& segments.iter().all(|s| {
					s.chars()
						.all(|c| c.is_ascii_alphanumeric())
				})
			{
				return Ok(format!("U/{inner}"));
			}
		}
	}
	bail!(
		"invalid room code format: {code}. Expected format: U/XXXX-XXXX-XXXX-XXXX"
	)
}

pub async fn get_logs() -> eyre::Result<String> {
	let state = TERRACOTTA_STATE.lock().await;
	let port = state
		.http_port
		.ok_or_else(|| eyre::eyre!("terracotta is not running"))?;
	drop(state);

	let client = terracotta_client(port).await?;
	let url = format!("http://127.0.0.1:{port}/log?fetch=");
	let resp = client
		.get(&url)
		.send()
		.await
		.wrap_err("failed to fetch terracotta logs")?;
	let body = resp
		.text()
		.await
		.wrap_err("failed to read terracotta logs")?;
	Ok(body)
}
