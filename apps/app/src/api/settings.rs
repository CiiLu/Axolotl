use crate::api::Result;
use tauri::Runtime;
use theseus::prelude::*;

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("settings")
        .invoke_handler(tauri::generate_handler![
            settings_get,
            settings_set,
            privacy_get,
            privacy_set,
            telemetry_set,
            discord_rpc_set,
            download_engine_set,
            cancel_directory_change
        ])
        .build()
}

// Get full settings
// invoke('plugin:settings|settings_get')
#[tauri::command]
pub async fn settings_get() -> Result<Settings> {
    let res = settings::get().await?;
    Ok(res)
}

// Set full settings
// invoke('plugin:settings|settings_set', settings)
#[tauri::command]
pub async fn settings_set(settings: Settings) -> Result<()> {
    settings::set(settings).await?;
    Ok(())
}

#[tauri::command]
pub async fn privacy_get() -> Result<PrivacySettings> {
    Ok(settings::get_privacy().await?)
}

#[tauri::command]
pub async fn privacy_set(privacy: PrivacySettings) -> Result<PrivacySettings> {
    Ok(settings::set_privacy(privacy).await?)
}

#[tauri::command]
pub async fn telemetry_set(enabled: bool) -> Result<PrivacySettings> {
    Ok(settings::set_telemetry(enabled).await?)
}

#[tauri::command]
pub async fn discord_rpc_set(enabled: bool) -> Result<PrivacySettings> {
    Ok(settings::set_discord_rpc(enabled).await?)
}

#[tauri::command]
pub async fn download_engine_set(engine: settings::DownloadEngine) -> Result<()> {
    settings::set_download_engine(engine).await?;
    Ok(())
}

#[tauri::command]
pub async fn cancel_directory_change<R: Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<()> {
    let identifier = &app.config().identifier;
    settings::cancel_directory_change(identifier).await?;
    Ok(())
}
