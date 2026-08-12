use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use super::{ImportOverrides, instance_json};
use crate::{
    State,
    install::{InstallPhaseDetails, InstallProgressReporter},
    launcher::get_loader_version_from_profile,
    pack::{
        import::finish_import,
        install_from::{self, CreatePackDescription, PackDependency},
    },
    state::ModLoader,
};

/// Import a generic launcher instance folder into an Axolotl profile.
///
/// Runs in four stages: resolve the source folder, validate that it contains
/// a detectable Minecraft version, register the instance metadata, then copy
/// (or symlink) the files into the profile.
pub async fn import_generic(
    instance_folder: PathBuf,
    instance_id: &str,
    reporter: InstallProgressReporter,
    details: InstallPhaseDetails,
    symlink: bool,
    overrides: &ImportOverrides,
) -> crate::Result<()> {
    let (name, dotminecraft) = resolve_dotminecraft(&instance_folder);
    let info = detect_instance_info(&dotminecraft, overrides).await?;
    register_instance(instance_id, &name, &info).await?;
    copy_instance_files(instance_id, &dotminecraft, reporter, details, symlink)
        .await
}

/// Stage 1 — resolve the name and the `.minecraft` directory of an imported
/// instance folder. Falls back to the folder itself when there is no nested
/// `.minecraft` subdirectory.
fn resolve_dotminecraft(instance_folder: &Path) -> (String, PathBuf) {
    let name = instance_folder
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "imported".to_string());

    let dotminecraft = instance_folder.join(".minecraft");
    if dotminecraft.is_dir() {
        tracing::debug!(
            "import_generic: using .minecraft subdir at {}",
            dotminecraft.display()
        );
        (name, dotminecraft)
    } else {
        tracing::debug!(
            "import_generic: using folder directly at {}",
            instance_folder.display()
        );
        (name, instance_folder.to_path_buf())
    }
}

/// Stage 2 — validate the folder contains a Minecraft version JSON.
async fn detect_instance_info(
    dotminecraft: &Path,
    overrides: &ImportOverrides,
) -> crate::Result<instance_json::InstanceInfo> {
    tracing::debug!(
        "import_generic: about to detect instance_json at dotminecraft={}",
        dotminecraft.display()
    );
    let Some(mut info) = instance_json::detect(dotminecraft) else {
        let Some(game_version) = overrides
            .game_version
            .as_ref()
            .filter(|version| !version.trim().is_empty())
        else {
            tracing::warn!(
                "import_generic: instance_json::detect returned None for {}",
                dotminecraft.display()
            );
            return Err(crate::ErrorKind::InputError(
                "Could not detect Minecraft version. Make sure the folder contains a valid version JSON."
                    .into(),
            )
            .into());
        };
        return Ok(instance_json::InstanceInfo {
            vanilla_name: game_version.clone(),
            loader: overrides
                .loader
                .filter(|loader| *loader != ModLoader::Vanilla)
                .map(|loader| loader.as_str().to_string()),
            loader_version: overrides
                .loader_version
                .clone()
                .filter(|version| !version.is_empty() && version != "latest"),
        });
    };

    if let Some(game_version) = overrides
        .game_version
        .as_ref()
        .filter(|version| !version.trim().is_empty())
    {
        info.vanilla_name.clone_from(game_version);
    }
    if let Some(loader) = overrides.loader {
        if loader == ModLoader::Vanilla {
            info.loader = None;
            info.loader_version = None;
        } else {
            info.loader = Some(loader.as_str().to_string());
            info.loader_version = None;
        }
    }
    if let Some(loader_version) = overrides
        .loader_version
        .as_ref()
        .filter(|version| !version.is_empty() && *version != "latest")
    {
        info.loader_version = Some(loader_version.clone());
    }

    Ok(info)
}

/// Stage 3 — register the instance metadata (name, game version, loaders)
/// with the app database.
async fn register_instance(
    instance_id: &str,
    name: &str,
    info: &instance_json::InstanceInfo,
) -> crate::Result<()> {
    tracing::debug!(
        "import_generic: detect result: vanilla_name={} loader={:?} loader_version={:?}",
        info.vanilla_name,
        info.loader,
        info.loader_version
    );

    let description = CreatePackDescription {
        icon: None,
        override_title: Some(name.to_string()),
        project_id: None,
        version_id: None,
        instance_id: instance_id.to_string(),
        source_filename: None,
    };
    let dependencies = build_dependencies(info).await;

    tracing::debug!(
        "import_generic: setting instance info with dependencies={:?}",
        dependencies
    );
    install_from::set_instance_information(
        instance_id.to_string(),
        &description,
        "Imported from folder",
        None,
        &dependencies,
        false,
    )
    .await
}

/// Builds the dependency map from the detected game version and loader,
/// resolving the loader version from the metadata API when it is missing.
async fn build_dependencies(
    info: &instance_json::InstanceInfo,
) -> HashMap<PackDependency, String> {
    let mut dependencies =
        HashMap::from([(PackDependency::Minecraft, info.vanilla_name.clone())]);
    let Some(ref loader) = info.loader else {
        tracing::debug!("import_generic: no loader detected, will be Vanilla");
        return dependencies;
    };

    let dep = loader_dependency(loader);
    let loader_version = resolve_loader_version(info).await;

    tracing::debug!(
        "import_generic: loader={} dep={:?} version={:?}",
        loader,
        dep,
        loader_version
    );
    match (dep, loader_version) {
        (Some(dep), Some(version)) => {
            dependencies.insert(dep, version);
        }
        (Some(dep), None) => {
            tracing::warn!(
                "import_generic: loader={} ({dep:?}) detected but no loader version could be resolved",
                loader
            );
        }
        (None, _) => {
            tracing::warn!(
                "import_generic: loader={} is not supported by this launcher; importing the instance without a loader (files are still copied)",
                loader
            );
        }
    }
    dependencies
}

/// Maps a detected loader name to a dependency the launcher can install.
/// Returns `None` for loaders the launcher does not support (LabyMod,
/// Legacy Fabric, Cleanroom, LiteLoader, ...); such instances are still
/// imported with their files intact, just without a registered loader.
fn loader_dependency(loader: &str) -> Option<PackDependency> {
    match loader {
        "forge" => Some(PackDependency::Forge),
        "neoforge" => Some(PackDependency::NeoForge),
        "fabric" => Some(PackDependency::FabricLoader),
        "quilt" => Some(PackDependency::QuiltLoader),
        "optifine" => Some(PackDependency::OptiFine),
        _ => None,
    }
}

/// Resolves a missing loader version by asking the metadata API for the
/// latest version compatible with the detected game version.
async fn resolve_loader_version(
    info: &instance_json::InstanceInfo,
) -> Option<String> {
    if info
        .loader_version
        .as_deref()
        .is_some_and(|version| !version.is_empty() && version != "latest")
    {
        return info.loader_version.clone();
    }
    let loader = info.loader.as_deref()?;
    let mod_loader = match loader {
        "forge" => Some(ModLoader::Forge),
        "neoforge" => Some(ModLoader::NeoForge),
        "fabric" => Some(ModLoader::Fabric),
        "quilt" => Some(ModLoader::Quilt),
        "optifine" => Some(ModLoader::OptiFine),
        _ => None,
    }?;
    tracing::debug!(
        "import_generic: loader={} has no version, resolving latest for game_version={}",
        loader,
        info.vanilla_name
    );
    match get_loader_version_from_profile(&info.vanilla_name, mod_loader, None)
        .await
    {
        Ok(Some(lv)) => {
            tracing::debug!(
                "import_generic: resolved latest loader version: {}",
                lv.id
            );
            Some(lv.id)
        }
        Ok(None) => {
            tracing::warn!(
                "import_generic: no loader version found for {} {}",
                mod_loader.as_str(),
                info.vanilla_name
            );
            None
        }
        Err(e) => {
            tracing::warn!(
                "import_generic: failed to resolve loader version: {e}",
            );
            None
        }
    }
}

/// Stage 4 — copy (or symlink) the source files into the instance profile.
async fn copy_instance_files(
    instance_id: &str,
    dotminecraft: &Path,
    reporter: InstallProgressReporter,
    details: InstallPhaseDetails,
    symlink: bool,
) -> crate::Result<()> {
    let state = State::get().await?;
    tracing::debug!(
        "import_generic: finishing import for instance_id={}",
        instance_id
    );
    finish_import(
        instance_id,
        dotminecraft.to_path_buf(),
        &state.io_semaphore,
        reporter,
        details,
        symlink,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loader_dependency_maps_supported_loaders() {
        assert_eq!(loader_dependency("forge"), Some(PackDependency::Forge));
        assert_eq!(
            loader_dependency("neoforge"),
            Some(PackDependency::NeoForge)
        );
        assert_eq!(
            loader_dependency("fabric"),
            Some(PackDependency::FabricLoader)
        );
        assert_eq!(
            loader_dependency("quilt"),
            Some(PackDependency::QuiltLoader)
        );
        assert_eq!(
            loader_dependency("optifine"),
            Some(PackDependency::OptiFine)
        );
    }

    #[test]
    fn loader_dependency_rejects_unsupported_loaders() {
        for loader in [
            "labymod",
            "legacy_fabric",
            "cleanroom",
            "lite_loader",
            "vanilla",
        ] {
            assert_eq!(loader_dependency(loader), None, "{loader}");
        }
    }

    #[tokio::test]
    async fn detect_overrides_replace_missing_detection() {
        let directory = tempfile::tempdir().unwrap();
        let overrides = ImportOverrides {
            game_version: Some("1.20.1".to_string()),
            loader: Some(ModLoader::Fabric),
            loader_version: Some("0.15.11".to_string()),
        };

        let info = detect_instance_info(directory.path(), &overrides)
            .await
            .unwrap();

        assert_eq!(info.vanilla_name, "1.20.1");
        assert_eq!(info.loader.as_deref(), Some("fabric"));
        assert_eq!(info.loader_version.as_deref(), Some("0.15.11"));
    }

    #[tokio::test]
    async fn detect_overrides_ignore_blank_and_latest_loader_version() {
        let directory = tempfile::tempdir().unwrap();
        for loader_version in ["", "latest"] {
            let overrides = ImportOverrides {
                game_version: Some("1.20.1".to_string()),
                loader: Some(ModLoader::Fabric),
                loader_version: Some(loader_version.to_string()),
            };

            let info = detect_instance_info(directory.path(), &overrides)
                .await
                .unwrap();

            assert_eq!(info.loader_version, None);
        }
    }

    #[tokio::test]
    async fn build_dependencies_uses_override_values() {
        let info = instance_json::InstanceInfo {
            vanilla_name: "1.20.1".to_string(),
            loader: Some("fabric".to_string()),
            loader_version: Some("0.15.11".to_string()),
        };

        let dependencies = build_dependencies(&info).await;

        assert_eq!(
            dependencies.get(&PackDependency::Minecraft),
            Some(&"1.20.1".to_string())
        );
        assert_eq!(
            dependencies.get(&PackDependency::FabricLoader),
            Some(&"0.15.11".to_string())
        );
    }
}
