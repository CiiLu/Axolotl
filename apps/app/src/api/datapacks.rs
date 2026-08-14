use crate::api::Result;
use either::Either;
use tauri::{AppHandle, Manager, Runtime};
use theseus::worlds::{WorldDatapack, WorldWithDatapacks};

pub fn init<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("datapacks")
        .invoke_handler(tauri::generate_handler![list_datapacks, delete_datapack])
        .build()
}

#[tauri::command]
pub async fn list_datapacks<R: Runtime>(
    app_handle: AppHandle<R>,
    instance_id: &str,
) -> Result<Vec<WorldWithDatapacks>> {
    let mut result = theseus::worlds::list_world_datapacks(instance_id).await?;
    for world in &mut result {
        for datapack in &mut world.datapacks {
            adapt_datapack_icon(&app_handle, datapack);
        }
    }
    Ok(result)
}

#[tauri::command]
pub async fn delete_datapack(
    instance_id: String,
    world_path: String,
    file_name: String,
) -> Result<()> {
    theseus::worlds::delete_world_datapack(&instance_id, &world_path, &file_name).await?;
    Ok(())
}

fn adapt_datapack_icon<R: Runtime>(
    app_handle: &AppHandle<R>,
    datapack: &mut WorldDatapack,
) {
    if let Some(Either::Left(icon_path)) = &datapack.icon {
        let icon_path = icon_path.clone();
        if let Ok(new_url) = super::utils::tauri_convert_file_src(&icon_path) {
            datapack.icon = Some(Either::Right(new_url));
            if let Err(error) = app_handle.asset_protocol_scope().allow_file(&icon_path) {
                tracing::warn!(
                    "Failed to allow file access for datapack icon {}: {}",
                    icon_path.display(),
                    error
                );
            }
        } else {
            tracing::warn!(
                "Encountered invalid icon path for datapack: {}",
                icon_path.display()
            );
            datapack.icon = None;
        }
    }
}
