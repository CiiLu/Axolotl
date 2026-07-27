use crate::api::Result;
use serde::{Deserialize, Serialize};
use theseus::Error as TheseusError;

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
	tauri::plugin::Builder::new("terracotta")
		.invoke_handler(tauri::generate_handler![
			terracotta_get_state,
			terracotta_get_meta,
			terracotta_start,
			terracotta_stop,
			terracotta_host,
			terracotta_join,
			terracotta_reset,
			terracotta_parse_room_code,
			terracotta_get_platform_key,
			terracotta_get_download_url,
		])
		.build()
}

#[tauri::command]
pub async fn terracotta_get_state() -> Result<theseus::terracotta::TerracottaState> {
	Ok(theseus::terracotta::get_state().await)
}

#[derive(Serialize)]
pub struct TerracottaMetaResponse {
	pub version: String,
	pub compile_timestamp: String,
	pub easytier_version: String,
	pub yggdrasil_port: u16,
	pub target_tuple: String,
	pub target_os: String,
}

#[tauri::command]
pub async fn terracotta_get_meta() -> Result<TerracottaMetaResponse> {
	let meta = theseus::terracotta::get_meta()
		.await
		.map_err(TheseusError::from)?;
	Ok(TerracottaMetaResponse {
		version: meta.version,
		compile_timestamp: meta.compile_timestamp,
		easytier_version: meta.easytier_version,
		yggdrasil_port: meta.yggdrasil_port,
		target_tuple: meta.target_tuple,
		target_os: meta.target_os,
	})
}

#[derive(Deserialize)]
pub struct TerracottaStartArgs {
	#[serde(default)]
	pub binary_path: Option<String>,
	#[serde(default)]
	pub auto_download: bool,
}

#[tauri::command]
pub async fn terracotta_start(args: TerracottaStartArgs) -> Result<()> {
	theseus::terracotta::start_terracotta(args.binary_path, args.auto_download)
		.await
		.map_err(TheseusError::from)?;
	Ok(())
}

#[tauri::command]
pub async fn terracotta_stop() -> Result<()> {
	theseus::terracotta::stop_terracotta()
		.await
		.map_err(TheseusError::from)?;
	Ok(())
}

#[derive(Deserialize)]
pub struct TerracottaHostArgs {
	#[serde(default)]
	pub room_code: Option<String>,
	pub player_name: String,
}

#[tauri::command]
pub async fn terracotta_host(args: TerracottaHostArgs) -> Result<()> {
	theseus::terracotta::start_hosting(args.room_code, args.player_name)
		.await
		.map_err(TheseusError::from)?;
	Ok(())
}

#[derive(Deserialize)]
pub struct TerracottaJoinArgs {
	pub room_code: String,
	pub player_name: String,
}

#[tauri::command]
pub async fn terracotta_join(args: TerracottaJoinArgs) -> Result<()> {
	theseus::terracotta::start_joining(args.room_code, args.player_name)
		.await
		.map_err(TheseusError::from)?;
	Ok(())
}

#[tauri::command]
pub async fn terracotta_reset() -> Result<()> {
	theseus::terracotta::reset_state()
		.await
		.map_err(TheseusError::from)?;
	Ok(())
}

#[derive(Deserialize)]
pub struct TerracottaParseRoomCodeArgs {
	pub room_code: String,
}

#[tauri::command]
pub async fn terracotta_parse_room_code(
	args: TerracottaParseRoomCodeArgs,
) -> Result<String> {
	theseus::terracotta::parse_room_code(&args.room_code)
		.await
		.map_err(TheseusError::from)
}

#[tauri::command]
pub async fn terracotta_get_platform_key() -> Result<String> {
	Ok(theseus::terracotta::terracotta_platform_key().to_string())
}

#[derive(Deserialize)]
pub struct TerracottaDownloadUrlArgs {
	pub version: String,
}

#[tauri::command]
pub async fn terracotta_get_download_url(args: TerracottaDownloadUrlArgs) -> Result<String> {
	let key = theseus::terracotta::terracotta_platform_key();
	Ok(format!(
		"https://github.com/burningtnt/Terracotta/releases/download/v{version}/terracotta-{version}-{key}-pkg.tar.gz",
		version = args.version,
		key = key,
	))
}
