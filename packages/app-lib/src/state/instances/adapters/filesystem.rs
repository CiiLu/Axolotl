use crate::state::ProjectType;
use crate::util::io::{self, IOError};
use std::path::Path;

#[derive(Clone, Debug)]
pub(crate) struct ScannedContentFile {
    pub relative_path: String,
    pub file_name: String,
    pub enabled: bool,
    pub size: u64,
    pub hash_cache_key: String,
}

pub(crate) fn scan_content_files(
    instances_dir: &Path,
    instance_path: &str,
) -> crate::Result<Vec<ScannedContentFile>> {
    let instance_dir = io::canonicalize(instances_dir.join(instance_path))?;
    let mut files = Vec::new();

    for project_type in ProjectType::iterator() {
        let folder = project_type.get_folder();
        let folder_path = instance_dir.join(folder);

        if !folder_path.exists() {
            continue;
        }

        scan_content_folder(
            &folder_path,
            folder,
            project_type,
            instance_path,
            &mut files,
        )?;
    }

    Ok(files)
}

fn scan_content_folder(
    folder_path: &Path,
    relative_dir: &str,
    project_type: ProjectType,
    instance_path: &str,
    files: &mut Vec<ScannedContentFile>,
) -> crate::Result<()> {
    for entry in std::fs::read_dir(folder_path)
        .map_err(|err| IOError::with_path(err, folder_path))?
    {
        let path = entry.map_err(IOError::from)?.path();
        if path.is_dir() {
            // Only schematics may live in nested folders; other content
            // folders are scanned at the top level only.
            if project_type == ProjectType::Schematic {
                let Some(dir_name) =
                    path.file_name().and_then(|value| value.to_str())
                else {
                    continue;
                };
                scan_content_folder(
                    &path,
                    &format!("{relative_dir}/{dir_name}"),
                    project_type,
                    instance_path,
                    files,
                )?;
            }
            continue;
        }

        if !path.is_file() {
            continue;
        }

        let Some(file_name) = path.file_name().and_then(|value| value.to_str())
        else {
            continue;
        };

        if !is_scannable_project_file(project_type, file_name) {
            continue;
        }

        let size = path.metadata().map_err(IOError::from)?.len();
        let relative_path = format!("{relative_dir}/{file_name}");
        let hash_cache_key =
            format!("{size}-{instance_path}/{relative_path}");

        files.push(ScannedContentFile {
            relative_path,
            file_name: file_name.to_string(),
            enabled: !file_name.ends_with(".disabled"),
            size,
            hash_cache_key,
        });
    }

    Ok(())
}

pub(crate) fn project_type_from_relative_path(
    relative_path: &str,
) -> Option<ProjectType> {
    let mut current = Path::new(relative_path).parent();
    while let Some(parent) = current {
        let folder_name = parent
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if let Some(project_type) = ProjectType::from_folder_name(folder_name) {
            return Some(project_type);
        }
        current = parent.parent();
    }
    None
}

fn is_scannable_project_file(
    project_type: ProjectType,
    file_name: &str,
) -> bool {
    let Some(extension) = Path::new(file_name.trim_end_matches(".disabled"))
        .extension()
        .and_then(|ext| ext.to_str())
    else {
        return false;
    };

    match project_type {
        ProjectType::Mod => extension.eq_ignore_ascii_case("jar"),
        ProjectType::DataPack
        | ProjectType::ResourcePack
        | ProjectType::ShaderPack => extension.eq_ignore_ascii_case("zip"),
        ProjectType::Schematic => {
            extension.eq_ignore_ascii_case("litematic")
                || extension.eq_ignore_ascii_case("schematic")
                || extension.eq_ignore_ascii_case("schem")
        }
        // WorldSave folders (saves/) are handled separately via worlds.rs,
        // not scanned as regular project files.
        ProjectType::WorldSave => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn scan_content_files_finds_nested_schematics() {
        let root = tempdir().unwrap();
        let instance_dir = root.path().join("inst");
        let schematics = instance_dir.join("schematics");
        fs::create_dir_all(schematics.join("redstone/contraptions")).unwrap();
        fs::create_dir_all(instance_dir.join("mods")).unwrap();
        fs::write(schematics.join("house.litematic"), "a").unwrap();
        fs::write(schematics.join("redstone/clock.litematic"), "b").unwrap();
        fs::write(schematics.join("redstone/contraptions/gear.schem"), "c")
            .unwrap();
        fs::write(schematics.join("redstone/notes.txt"), "d").unwrap();
        fs::write(instance_dir.join("mods/example.jar"), "e").unwrap();

        let files = scan_content_files(root.path(), "inst").unwrap();

        let paths: Vec<&str> = files
            .iter()
            .map(|file| file.relative_path.as_str())
            .collect();
        assert!(paths.contains(&"schematics/house.litematic"));
        assert!(paths.contains(&"schematics/redstone/clock.litematic"));
        assert!(paths.contains(&"schematics/redstone/contraptions/gear.schem"));
        assert!(!paths.contains(&"schematics/redstone/notes.txt"));
        assert!(paths.contains(&"mods/example.jar"));
        assert_eq!(files.len(), 4);
    }

    #[test]
    fn project_type_from_relative_path_matches_nested_folders() {
        assert_eq!(
            project_type_from_relative_path("schematics/house.litematic"),
            Some(ProjectType::Schematic)
        );
        assert_eq!(
            project_type_from_relative_path(
                "schematics/redstone/clock.litematic"
            ),
            Some(ProjectType::Schematic)
        );
        assert_eq!(
            project_type_from_relative_path("schematics/a/b/c/tower.litematic"),
            Some(ProjectType::Schematic)
        );
        assert_eq!(
            project_type_from_relative_path("mods/example.jar"),
            Some(ProjectType::Mod)
        );
        assert_eq!(
            project_type_from_relative_path("config/example.json"),
            None
        );
    }
}
