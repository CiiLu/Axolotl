use crate::api::content_search::{
    chinese_file_title_for_modrinth_slug, localized_content_file_name,
};
use crate::state::instances::{
    ContentRequirement, ContentSourceKind, Instance, InstanceFile,
    adapters::sqlite::{content_rows, instance_rows},
};
use crate::state::{
    CacheBehaviour, CachedEntry, ContentProviderRef, Dependency,
    DependencyType, KnownModrinthFile, ModLoader, ModrinthProjectId,
    ModrinthVersionId, ProjectType, State, Version, cache_file_hash,
    cache_file_hash_metadata,
};
use crate::util::fetch::{
    self, ContentValidation, DownloadMeta, DownloadReason, DownloadRequest,
    Integrity, ResourceClass, download_to_path,
};
use crate::util::io;
use crate::util::io::io_error_with_lock_info;
use async_trait::async_trait;
use bytes::Bytes;
use modrinth_content_management::{
    ContentMetadataProvider, ContentType, Error as ResolveError,
    ResolutionPreferences, ResolveContentPlan, ResolveContentRequest,
    ResolvedContent,
};
use std::path::{Path, PathBuf};
use tracing::warn;

pub(crate) struct ContentScope {
    pub instance: Instance,
    pub content_set_id: String,
}

pub(crate) struct InstalledContentFile {
    pub relative_path: String,
    pub provider_refs: Vec<ContentProviderRef>,
    pub enabled: bool,
}

pub(crate) struct DownloadedProjectVersion {
    pub file_name: String,
    pub path: PathBuf,
    pub sha1: String,
    pub size: u64,
    pub project_type: ProjectType,
    pub project_id: String,
    pub version_id: String,
}

pub(crate) struct InstanceInstallProjectRequest {
    pub project_id: String,
    pub version_id: Option<String>,
    pub content_type: ContentType,
    pub selected: ResolutionPreferences,
}

struct CachedEntryContentProvider<'a> {
    state: &'a State,
    cache_behaviour: Option<CacheBehaviour>,
}

#[async_trait]
impl ContentMetadataProvider for CachedEntryContentProvider<'_> {
    async fn get_version(
        &mut self,
        version_id: &str,
    ) -> Result<Option<modrinth_content_management::Version>, ResolveError>
    {
        let version = CachedEntry::get_version(
            &ModrinthVersionId::new(version_id.to_string())
                .map_err(resolve_provider_error)?,
            self.cache_behaviour,
            &self.state.pool,
            &self.state.api_semaphore,
        )
        .await
        .map_err(resolve_provider_error)?;

        Ok(version.map(version_to_resolver))
    }

    async fn get_project_versions(
        &mut self,
        project_id: &str,
    ) -> Result<Vec<modrinth_content_management::Version>, ResolveError> {
        let versions = CachedEntry::get_project_versions(
            &ModrinthProjectId::new(project_id.to_string())
                .map_err(resolve_provider_error)?,
            self.cache_behaviour,
            &self.state.pool,
            &self.state.api_semaphore,
        )
        .await
        .map_err(resolve_provider_error)?;

        Ok(versions
            .unwrap_or_default()
            .into_iter()
            .map(version_to_resolver)
            .collect())
    }
}

fn resolve_provider_error(error: crate::Error) -> ResolveError {
    ResolveError::Provider(error.to_string())
}

fn resolver_error(error: ResolveError) -> crate::Error {
    crate::ErrorKind::InputError(error.to_string()).into()
}

fn version_to_resolver(
    version: Version,
) -> modrinth_content_management::Version {
    modrinth_content_management::Version {
        id: version.id,
        project_id: version.project_id,
        date_published: version.date_published,
        dependencies: version
            .dependencies
            .into_iter()
            .map(dependency_to_resolver)
            .collect(),
        game_versions: version.game_versions,
        loaders: version.loaders,
    }
}

fn dependency_to_resolver(
    dependency: Dependency,
) -> modrinth_content_management::Dependency {
    modrinth_content_management::Dependency {
        version_id: dependency.version_id,
        project_id: dependency.project_id,
        file_name: dependency.file_name,
        dependency_type: match dependency.dependency_type {
            DependencyType::Required => {
                modrinth_content_management::DependencyType::Required
            }
            DependencyType::Optional => {
                modrinth_content_management::DependencyType::Optional
            }
            DependencyType::Incompatible => {
                modrinth_content_management::DependencyType::Incompatible
            }
            DependencyType::Embedded => {
                modrinth_content_management::DependencyType::Embedded
            }
        },
    }
}

fn target_preferences(
    game_version: String,
    loader: ModLoader,
    content_type: ContentType,
) -> ResolutionPreferences {
    let loader = match content_type {
        ContentType::DataPack => "datapack".to_string(),
        ContentType::ResourcePack => "minecraft".to_string(),
        ContentType::Shader => "iris".to_string(),
        _ => loader.as_str().to_string(),
    };

    ResolutionPreferences {
        game_versions: vec![game_version],
        loaders: vec![loader],
    }
}

pub(crate) async fn resolve_install_plan(
    instance_id: &str,
    request: InstanceInstallProjectRequest,
    state: &State,
) -> crate::Result<ResolveContentPlan> {
    let content_set =
        content_rows::get_applied_content_set(instance_id, &state.pool)
            .await?
            .ok_or_else(|| {
                crate::ErrorKind::InputError(format!(
                    "Instance {instance_id} has no applied content set"
                ))
            })?;
    let existing_project_ids =
        crate::state::get_installed_project_ids_for_instance(
            instance_id,
            None,
            state,
        )
        .await?;
    let provider = CachedEntryContentProvider {
        state,
        cache_behaviour: Some(CacheBehaviour::MustRevalidate),
    };
    let content_type = request.content_type;
    let request = ResolveContentRequest {
        project_id: request.project_id,
        version_id: request.version_id,
        content_type,
        selected: request.selected,
        target: target_preferences(
            content_set.game_version,
            content_set.loader,
            content_type,
        ),
        existing_project_ids,
    };

    modrinth_content_management::resolve_content(provider, request)
        .await
        .map_err(resolver_error)
}

pub(crate) async fn install_resolved_content_plan(
    instance_id: &str,
    plan: &ResolveContentPlan,
    state: &State,
) -> crate::Result<()> {
    add_resolved_content(
        instance_id,
        &plan.primary,
        DownloadReason::Standalone,
        state,
    )
    .await?;
    for dependency in &plan.dependencies {
        add_resolved_content(
            instance_id,
            dependency,
            DownloadReason::Dependency,
            state,
        )
        .await?;
    }

    Ok(())
}

pub(crate) async fn switch_project_version_with_dependencies(
    instance_id: &str,
    project_path: &str,
    version_id: &str,
    state: &State,
) -> crate::Result<String> {
    let version = CachedEntry::get_version(
        &ModrinthVersionId::new(version_id.to_string())?,
        Some(CacheBehaviour::MustRevalidate),
        &state.pool,
        &state.api_semaphore,
    )
    .await?
    .ok_or_else(|| {
        crate::ErrorKind::InputError(format!(
            "Unable to install version id {version_id}. Not found."
        ))
    })?;
    let content_type = ProjectType::get_from_loaders(version.loaders.clone())
        .map(ContentType::from)
        .unwrap_or(ContentType::Mod);
    let plan = resolve_install_plan(
        instance_id,
        InstanceInstallProjectRequest {
            project_id: version.project_id,
            version_id: Some(version_id.to_string()),
            content_type,
            selected: ResolutionPreferences::default(),
        },
        state,
    )
    .await?;

    let was_disabled = project_path.ends_with(".disabled");
    let mut new_path = add_project_from_version(
        instance_id,
        &plan.primary.version_id,
        DownloadReason::Update,
        None,
        ContentSourceKind::Local,
        state,
    )
    .await?;

    if was_disabled {
        new_path =
            toggle_disable_project(instance_id, &new_path, Some(false), state)
                .await?;
    }

    for dependency in &plan.dependencies {
        add_resolved_content(
            instance_id,
            dependency,
            DownloadReason::Dependency,
            state,
        )
        .await?;
    }

    if new_path != project_path {
        remove_project(instance_id, project_path, state).await?;
    }

    Ok(new_path)
}

async fn add_resolved_content(
    instance_id: &str,
    content: &ResolvedContent,
    reason: DownloadReason,
    state: &State,
) -> crate::Result<String> {
    add_project_from_version(
        instance_id,
        &content.version_id,
        reason,
        content.dependent_on_version_id.clone(),
        ContentSourceKind::Local,
        state,
    )
    .await
}

pub(crate) async fn resolve_content_scope(
    instance_id: &str,
    content_set_id: Option<&str>,
    state: &State,
) -> crate::Result<ContentScope> {
    let instance = instance_rows::get_instance_by_id(instance_id, &state.pool)
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::InputError("Unknown instance".to_string())
        })?;
    let content_set_id = match content_set_id {
        Some(id) => id.to_string(),
        None => instance.applied_content_set_id.clone().ok_or_else(|| {
            crate::ErrorKind::InputError(format!(
                "Instance {} has no applied content set",
                instance.id
            ))
        })?,
    };

    Ok(ContentScope {
        instance,
        content_set_id,
    })
}

pub(crate) async fn add_project_from_version(
    instance_id: &str,
    version_id: &str,
    reason: DownloadReason,
    dependent_on_version_id: Option<String>,
    source_kind: ContentSourceKind,
    state: &State,
) -> crate::Result<String> {
    let downloaded = download_project_version(
        instance_id,
        version_id,
        reason,
        dependent_on_version_id,
        state,
    )
    .await?;

    add_downloaded_project_version(instance_id, downloaded, source_kind, state)
        .await
}

pub(crate) async fn download_project_version(
    instance_id: &str,
    version_id: &str,
    reason: DownloadReason,
    dependent_on_version_id: Option<String>,
    state: &State,
) -> crate::Result<DownloadedProjectVersion> {
    let prepared = prepare_version_download(
        instance_id,
        version_id,
        reason,
        dependent_on_version_id,
        state,
    )
    .await?;

    let download = download_to_path(
        DownloadRequest::new(&prepared.url, ResourceClass::Modrinth)
            .with_integrity(prepared.integrity)
            .with_download_meta(prepared.download_meta),
        &prepared.path,
        &state.download_semaphore,
        &state.pool,
        None,
    )
    .await?;

    let sha1 = if let Some(hash) = &prepared.sha1 {
        hash.clone()
    } else {
        fetch::sha1_file_async(&prepared.path).await?.1
    };
    let project_type = ProjectType::get_from_loaders(prepared.loaders.clone())
        .ok_or_else(|| {
            crate::ErrorKind::InputError(format!(
                "Unable to infer project type for version {version_id}"
            ))
        })?;

    Ok(DownloadedProjectVersion {
        file_name: prepared.file_name,
        path: prepared.path,
        sha1,
        size: download.size,
        project_type,
        project_id: prepared.project_id,
        version_id: prepared.version_id,
    })
}

/// Everything needed to download a version file, resolved before the actual
/// network transfer.
struct PreparedVersionDownload {
    url: String,
    path: PathBuf,
    download_meta: DownloadMeta,
    integrity: Integrity,
    file_name: String,
    sha1: Option<String>,
    loaders: Vec<String>,
    project_id: String,
    version_id: String,
}

/// Resolves the content scope and version metadata for a download and
/// validates the target path, without touching the network.
async fn prepare_version_download(
    instance_id: &str,
    version_id: &str,
    reason: DownloadReason,
    dependent_on_version_id: Option<String>,
    state: &State,
) -> crate::Result<PreparedVersionDownload> {
    let scope = resolve_content_scope(instance_id, None, state).await?;
    let content_set =
        content_rows::get_content_set(&scope.content_set_id, &state.pool)
            .await?
            .ok_or_else(|| {
                crate::ErrorKind::InputError(format!(
                    "Unknown content set {}",
                    scope.content_set_id
                ))
            })?;
    let version = CachedEntry::get_version(
        &ModrinthVersionId::new(version_id.to_string())?,
        None,
        &state.pool,
        &state.api_semaphore,
    )
    .await?
    .ok_or_else(|| {
        crate::ErrorKind::InputError(format!(
            "Unable to install version id {version_id}. Not found."
        ))
    })?;
    let file = version
        .files
        .iter()
        .find(|file| file.primary)
        .or_else(|| version.files.first())
        .ok_or_else(|| {
            crate::ErrorKind::InputError(
                "No files for input version present!".to_string(),
            )
        })?;
    let download_meta = DownloadMeta {
        reason,
        game_version: content_set.game_version,
        loader: content_set.loader.as_str().to_string(),
        dependent_on: dependent_on_version_id,
    };
    let file_name_path = Path::new(&file.filename);
    if file_name_path.as_os_str().is_empty()
        || file_name_path.is_absolute()
        || file_name_path.components().count() != 1
        || !matches!(
            file_name_path.components().next(),
            Some(std::path::Component::Normal(_))
        )
    {
        return Err(crate::ErrorKind::InputError(
            "Modrinth returned an invalid project file name".to_string(),
        )
        .into());
    }
    let path = state
        .directories
        .caches_dir()
        .join("content")
        .join("modrinth")
        .join(&version.id)
        .join(file_name_path);
    let content = if matches!(
        file_name_path.extension().and_then(|value| value.to_str()),
        Some("jar" | "zip" | "mrpack")
    ) {
        ContentValidation::Jar
    } else {
        ContentValidation::None
    };
    let integrity = Integrity {
        size: Some(file.size as u64),
        sha1: file.hashes.get("sha1").cloned(),
        sha512: file.hashes.get("sha512").cloned(),
        content,
        ..Integrity::default()
    };

    Ok(PreparedVersionDownload {
        url: file.url.clone(),
        path,
        download_meta,
        integrity,
        file_name: file.filename.clone(),
        sha1: file.hashes.get("sha1").cloned(),
        loaders: version.loaders.clone(),
        project_id: version.project_id.clone(),
        version_id: version.id.clone(),
    })
}

/// Chooses the instance-relative path for a content file that is about to be
/// installed. Paths already recorded for this instance always win, so repeat
/// installs never duplicate a project across locale switches; otherwise the
/// `[中文名]` candidate is used when the app language is Simplified Chinese.
pub(crate) async fn resolve_content_install_relative_path(
    instance_id: &str,
    original_relative_path: String,
    localized_candidate: Option<String>,
    pool: &sqlx::SqlitePool,
) -> crate::Result<String> {
    if content_rows::get_instance_file_by_relative_path(
        instance_id,
        &original_relative_path,
        pool,
    )
    .await?
    .is_some()
    {
        return Ok(original_relative_path);
    }
    let Some(localized_candidate) = localized_candidate else {
        return Ok(original_relative_path);
    };
    if content_rows::get_instance_file_by_relative_path(
        instance_id,
        &localized_candidate,
        pool,
    )
    .await?
    .is_some()
    {
        return Ok(localized_candidate);
    }
    if crate::state::Settings::get(pool).await?.locale == "zh-CN" {
        return Ok(localized_candidate);
    }
    Ok(original_relative_path)
}

/// Builds the `[中文名]` file-name candidate for a Modrinth project file by
/// resolving the project's slug from cache. Any failure disables the naming.
async fn modrinth_chinese_file_name_candidate(
    project_id: &str,
    file_name: &str,
    state: &State,
) -> Option<String> {
    let project = CachedEntry::get_project(
        &ModrinthProjectId::new(project_id.to_string()).ok()?,
        None,
        &state.pool,
        &state.api_semaphore,
    )
    .await
    .ok()??;
    let title = chinese_file_title_for_modrinth_slug(&project.slug?)?;
    localized_content_file_name(file_name, &title)
}

pub(crate) async fn add_downloaded_project_version(
    instance_id: &str,
    downloaded: DownloadedProjectVersion,
    source_kind: ContentSourceKind,
    state: &State,
) -> crate::Result<String> {
    let _instance_lock = state.lock_instance_content(instance_id).await;

    let DownloadedProjectVersion {
        file_name,
        path,
        sha1,
        size,
        project_type,
        project_id,
        version_id,
    } = downloaded;
    let scope = resolve_content_scope(instance_id, None, state).await?;
    let localized_candidate =
        modrinth_chinese_file_name_candidate(&project_id, &file_name, state)
            .await
            .map(|file_name| {
                format!("{}/{}", project_type.get_folder(), file_name)
            });
    let relative_path = resolve_content_install_relative_path(
        instance_id,
        format!("{}/{}", project_type.get_folder(), file_name),
        localized_candidate,
        &state.pool,
    )
    .await?;
    let full_path =
        instance_full_path(state, &scope.instance).join(&relative_path);
    let previous_path = materialize_project_download(&path, &full_path).await?;
    let provider_ref = ContentProviderRef::Modrinth {
        project_id: ModrinthProjectId::new(project_id.clone())?,
        version_id: Some(ModrinthVersionId::new(version_id.clone())?),
    };
    let record_result = record_project_file_atomic(
        instance_id,
        &relative_path,
        &sha1,
        size,
        project_type,
        source_kind,
        Some(&provider_ref),
        true,
        Some(KnownModrinthFile {
            project_id: &project_id,
            version_id: &version_id,
        }),
        state,
    )
    .await;
    match record_result {
        Ok(()) => {
            finalize_project_materialization(previous_path.as_deref()).await?
        }
        Err(error) => {
            restore_project_materialization(
                &full_path,
                previous_path.as_deref(),
            )
            .await?;
            return Err(error);
        }
    }
    Ok(relative_path)
}

pub(crate) async fn materialize_project_download(
    source: &Path,
    destination: &Path,
) -> crate::Result<Option<PathBuf>> {
    if let Some(parent) = destination.parent() {
        io::create_dir_all(parent).await?;
    }
    let mut temporary = destination.as_os_str().to_os_string();
    temporary.push(".installing");
    let temporary = PathBuf::from(temporary);
    if temporary.exists() {
        io::remove_file(&temporary).await?;
    }
    if tokio::fs::hard_link(source, &temporary).await.is_err() {
        io::copy(source, &temporary).await?;
    }
    let mut backup = destination.as_os_str().to_os_string();
    backup.push(".installing.previous");
    let backup = PathBuf::from(backup);
    if backup.exists() {
        io::remove_file(&backup).await?;
    }
    if destination.exists() {
        tokio::fs::rename(destination, &backup).await?;
    }
    if let Err(error) = tokio::fs::rename(&temporary, destination).await {
        if backup.exists() {
            let _ = tokio::fs::rename(&backup, destination).await;
        }
        warn!(
            "Failed to rename temporary file to destination — checking file lock"
        );
        return Err(crate::Error::from(io_error_with_lock_info(
            error,
            destination,
        )));
    }
    Ok(destination
        .exists()
        .then_some(backup)
        .filter(|path| path.exists()))
}

pub(crate) async fn finalize_project_materialization(
    previous_path: Option<&Path>,
) -> crate::Result<()> {
    if let Some(previous_path) = previous_path
        && previous_path.exists()
    {
        io::remove_file(previous_path).await?;
    }
    Ok(())
}

pub(crate) async fn restore_project_materialization(
    destination: &Path,
    previous_path: Option<&Path>,
) -> crate::Result<()> {
    if destination.exists() {
        io::remove_file(destination).await?;
    }
    if let Some(previous_path) = previous_path
        && previous_path.exists()
    {
        tokio::fs::rename(previous_path, destination).await?;
    }
    Ok(())
}

pub(crate) async fn add_project_from_path(
    instance_id: &str,
    path: &Path,
    project_type: Option<ProjectType>,
    state: &State,
) -> crate::Result<String> {
    let file = io::read(path).await?;
    let file_name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    add_project_bytes(
        instance_id,
        &file_name,
        Bytes::from(file),
        None,
        project_type,
        ContentSourceKind::Local,
        state,
    )
    .await
}

pub(crate) async fn add_project_bytes(
    instance_id: &str,
    file_name: &str,
    bytes: Bytes,
    hash: Option<&str>,
    project_type: Option<ProjectType>,
    source_kind: ContentSourceKind,
    state: &State,
) -> crate::Result<String> {
    let _instance_lock = state.lock_instance_content(instance_id).await;

    let scope = resolve_content_scope(instance_id, None, state).await?;
    let project_type = match project_type {
        Some(project_type) => project_type,
        None => infer_project_type(&bytes)?,
    };
    // Minecraft only loads resource/shader pack ZIPs whose markers sit at
    // the archive root. Archives produced by zipping a pack folder wrap it
    // in one folder; extract the pack folder(s) directly so the result is
    // usable as-is — no re-packing, no recompression.
    if let Some(plan) = wrapped_pack_plan(&bytes, project_type) {
        let install_path = install_wrapped_pack(
            &bytes,
            &plan,
            project_type,
            &scope.instance,
            state,
        )
        .await?;
        // Let the instance content scanner index the extracted files.
        crate::event::emit::emit_instance(
            &scope.instance.id,
            crate::event::InstancePayloadType::Synced,
        )
        .await?;
        return Ok(install_path);
    }
    let relative_path = format!("{}/{}", project_type.get_folder(), file_name);
    let full_path =
        instance_full_path(state, &scope.instance).join(&relative_path);
    let sha1 = match hash {
        Some(hash) => hash.to_string(),
        None => fetch::sha1_async(bytes.clone()).await?,
    };

    cache_file_hash(
        bytes.clone(),
        &scope.instance.path,
        &relative_path,
        Some(&sha1),
        Some(project_type),
        None,
        &state.pool,
    )
    .await?;
    fetch::write(&full_path, &bytes, &state.io_semaphore).await?;

    let local_mod_data = if project_type == ProjectType::Mod {
        crate::mod_metadata::extract_mod_metadata(&bytes)
            .and_then(|meta| serde_json::to_string(&meta).ok())
    } else {
        None
    };

    let file = content_rows::upsert_instance_file_from_parts(
        content_rows::UpsertInstanceFile {
            instance_id: &scope.instance.id,
            relative_path: &relative_path,
            file_name,
            enabled: !relative_path.ends_with(".disabled"),
            sha1: &sha1,
            size: bytes.len() as u64,
            missing: false,
            local_mod_data: local_mod_data.as_deref(),
            icon_path: None,
        },
        &state.pool,
    )
    .await?;
    upsert_entry_for_file(
        &scope,
        &file,
        project_type,
        source_kind,
        None,
        false,
        state,
    )
    .await?;

    Ok(relative_path)
}

/// Install plan for a resource/shader pack ZIP wrapped in a single root
/// folder: either the wrapper folder itself is the pack, or its direct
/// children are one or more packs (a collection).
#[derive(Debug, PartialEq)]
enum WrappedPackPlan {
    /// The wrapper folder is the pack; install it as a whole folder.
    Whole { wrapper: String },
    /// Direct children of the wrapper are packs; install each one.
    Children { children: Vec<String> },
}

/// Detect whether `bytes` is a resource/shader pack ZIP wrapped in a single
/// top-level folder. Returns the install plan, or `None` for flat archives,
/// other content types and mixed layouts (which are installed as-is).
fn wrapped_pack_plan(
    bytes: &[u8],
    project_type: ProjectType,
) -> Option<WrappedPackPlan> {
    use std::io::Cursor;

    if !matches!(
        project_type,
        ProjectType::ResourcePack | ProjectType::ShaderPack
    ) {
        return None;
    }

    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).ok()?;
    let mut names: Vec<String> = Vec::with_capacity(archive.len());
    for i in 0..archive.len() {
        let entry = archive.by_index_raw(i).ok()?;
        names.push(
            crate::api::pack::detect::decode_zip_entry_name(entry.name_raw())
                .replace('\\', "/"),
        );
    }

    let has_marker_at = |prefix: &str| -> bool {
        let pack_mcmeta = format!("{prefix}pack.mcmeta");
        let shaders = format!("{prefix}shaders/");
        match project_type {
            ProjectType::ResourcePack => {
                names.iter().any(|name| name == &pack_mcmeta)
            }
            ProjectType::ShaderPack => names.iter().any(|name| {
                let Some(rest) = name.strip_prefix(&shaders) else {
                    return false;
                };
                if rest.is_empty() {
                    return false;
                }
                let lower = rest.to_ascii_lowercase();
                lower.ends_with(".fsh")
                    || lower.ends_with(".vsh")
                    || lower.ends_with(".glsl")
            }),
            _ => false,
        }
    };

    // Every non-directory entry must share a single top-level folder.
    let mut wrapper: Option<&str> = None;
    for name in &names {
        if name.is_empty() || name.ends_with('/') {
            continue;
        }
        let Some((first, _)) = name.split_once('/') else {
            return None; // flat archive
        };
        match wrapper {
            None => wrapper = Some(first),
            Some(existing) if existing != first => return None,
            _ => {}
        }
    }
    let wrapper = wrapper?;
    let wrapper_prefix = format!("{wrapper}/");

    // The wrapper itself is the pack when its marker sits at its root.
    if has_marker_at(&wrapper_prefix) {
        return Some(WrappedPackPlan::Whole {
            wrapper: wrapper.to_string(),
        });
    }

    // Otherwise its direct children are packs (single pack or a collection).
    let mut children: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for name in &names {
        let Some(rest) = name.strip_prefix(&wrapper_prefix) else {
            continue;
        };
        if rest.is_empty() {
            continue;
        }
        let Some((child, _)) = rest.split_once('/') else {
            // Direct file under the wrapper: a zipped pack.
            if rest.to_lowercase().ends_with(".zip")
                && seen.insert(rest.to_string())
            {
                children.push(rest.to_string());
            }
            continue;
        };
        if seen.insert(child.to_string()) {
            let child_base = format!("{wrapper_prefix}{child}/");
            if has_marker_at(&child_base) {
                children.push(child.to_string());
            }
        }
    }
    if children.is_empty() {
        return None;
    }
    Some(WrappedPackPlan::Children { children })
}

/// Materialize a wrapped resource/shader pack into the instance's
/// `resourcepacks/` / `shaderpacks/` folder by extracting it (no
/// recompression). Returns the relative path of the installed pack.
async fn install_wrapped_pack(
    bytes: &Bytes,
    plan: &WrappedPackPlan,
    project_type: ProjectType,
    instance: &Instance,
    state: &State,
) -> crate::Result<String> {
    let folder = project_type.get_folder();
    let dest = instance_full_path(state, instance).join(folder);
    tokio::fs::create_dir_all(&dest).await?;

    let temp_dir = tempfile::tempdir().map_err(|e| {
        crate::ErrorKind::InputError(format!(
            "Failed to create temporary directory: {e}"
        ))
    })?;
    let zip_path = temp_dir.path().join("pack.zip");
    tokio::fs::write(&zip_path, &bytes[..]).await?;

    match plan {
        WrappedPackPlan::Whole { wrapper } => {
            // Extract straight into the pack folder: dest/<wrapper>/.
            extract_zip_to_dir_async(&zip_path, &dest).await?;
            Ok(format!("{folder}/{wrapper}"))
        }
        WrappedPackPlan::Children { children } => {
            let extracted = temp_dir.path().join("extracted");
            extract_zip_to_dir_async(&zip_path, &extracted).await?;
            let mut installed = String::new();
            for child in children {
                let src = extracted.join(child);
                let dst = dest.join(child);
                if dst.exists() {
                    continue;
                }
                move_or_copy(&src, &dst).await?;
                if installed.is_empty() {
                    installed = format!("{folder}/{child}");
                }
            }
            Ok(installed)
        }
    }
}

/// Zip-slip-safe extraction of a ZIP into a directory, run off the async
/// runtime.
async fn extract_zip_to_dir_async(
    zip_path: &Path,
    out: &Path,
) -> crate::Result<()> {
    let zip_path = zip_path.to_path_buf();
    let out = out.to_path_buf();
    tokio::task::spawn_blocking(move || {
        crate::api::drop_classifier::extract_zip_to_dir(&zip_path, &out)
    })
    .await
    .map_err(|e| {
        crate::ErrorKind::InputError(format!("Extraction task panicked: {e}"))
    })?
    .map_err(|e| crate::Error::from(crate::ErrorKind::InputError(e)))
}

/// Move a file or folder into place, falling back to a copy when the source
/// and destination live on different volumes.
async fn move_or_copy(src: &Path, dst: &Path) -> crate::Result<()> {
    match tokio::fs::rename(src, dst).await {
        Ok(()) => Ok(()),
        Err(_) => {
            if src.is_dir() {
                io::copy_dir(src, dst).await?;
            } else {
                tokio::fs::copy(src, dst).await?;
            }
            Ok(())
        }
    }
}

pub(crate) async fn record_project_file_atomic(
    instance_id: &str,
    relative_path: &str,
    sha1: &str,
    size: u64,
    project_type: ProjectType,
    source_kind: ContentSourceKind,
    provider_ref: Option<&ContentProviderRef>,
    origin: bool,
    known_modrinth_file: Option<KnownModrinthFile<'_>>,
    state: &State,
) -> crate::Result<()> {
    let _instance_lock = state.lock_instance_content(instance_id).await;

    let scope = resolve_content_scope(instance_id, None, state).await?;
    let file_name = Path::new(relative_path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let mut tx = begin_content_write(&state.pool).await?;
    content_rows::ensure_content_write_parents(
        &scope.instance.id,
        &scope.content_set_id,
        &mut tx,
    )
    .await?;
    let file = content_rows::upsert_instance_file_from_parts_in_transaction(
        content_rows::UpsertInstanceFile {
            instance_id: &scope.instance.id,
            relative_path,
            file_name: &file_name,
            enabled: !relative_path.ends_with(".disabled"),
            sha1,
            size,
            missing: false,
            local_mod_data: None,
            icon_path: None,
        },
        &mut tx,
    )
    .await?;
    let entry = content_rows::upsert_content_entry_from_parts_in_transaction(
        content_rows::UpsertContentEntry {
            instance_id: &scope.instance.id,
            content_set_id: &scope.content_set_id,
            file_id: Some(&file.id),
            project_type,
            source_kind,
            server_requirement: ContentRequirement::Required,
            client_requirement: ContentRequirement::Required,
            enabled: file.enabled,
        },
        &mut tx,
    )
    .await?;
    if let Some(provider_ref) = provider_ref {
        content_rows::upsert_content_provider_ref_in_transaction(
            &entry.id,
            provider_ref,
            origin,
            &mut tx,
        )
        .await?;
    }
    cache_file_hash_metadata(
        &scope.instance.path,
        relative_path,
        size,
        sha1.to_string(),
        Some(project_type),
        known_modrinth_file,
        &mut *tx,
    )
    .await?;
    tx.commit().await?;
    Ok(())
}

async fn begin_content_write(
    pool: &sqlx::SqlitePool,
) -> crate::Result<sqlx::Transaction<'static, sqlx::Sqlite>> {
    Ok(pool.begin_with("BEGIN IMMEDIATE").await?)
}

pub(crate) async fn toggle_disable_project(
    instance_id: &str,
    project_path: &str,
    desired_enabled: Option<bool>,
    state: &State,
) -> crate::Result<String> {
    let scope = resolve_content_scope(instance_id, None, state).await?;
    let base = instance_full_path(state, &scope.instance);
    let (current_path, enabled, new_path) =
        resolve_toggle_paths(&base, project_path, desired_enabled)?;

    if current_path != new_path {
        io::rename_or_move(&base.join(&current_path), &base.join(&new_path))
            .await?;
    }

    let file = rename_indexed_file(
        &scope,
        project_path,
        &current_path,
        &new_path,
        enabled,
        state,
    )
    .await?;
    let updated_entry = content_rows::set_content_entry_enabled_for_file(
        &scope.content_set_id,
        &file.id,
        enabled,
        &state.pool,
    )
    .await?;
    if !updated_entry {
        let project_type = ProjectType::get_from_parent_folder(&new_path)
            .ok_or_else(|| {
                crate::ErrorKind::InputError(format!(
                    "Unable to infer project type from {new_path}"
                ))
            })?;
        upsert_entry_for_file(
            &scope,
            &file,
            project_type,
            ContentSourceKind::Local,
            None,
            false,
            state,
        )
        .await?;
    }

    Ok(new_path)
}

/// Resolves which of `project_path` / `{trimmed}.disabled` currently exists
/// and which path the toggle should end up at.
fn resolve_toggle_paths(
    base: &Path,
    project_path: &str,
    desired_enabled: Option<bool>,
) -> crate::Result<(String, bool, String)> {
    let trimmed = project_path.trim_end_matches(".disabled");
    let current_path = if base.join(project_path).exists() {
        project_path.to_string()
    } else if base.join(format!("{trimmed}.disabled")).exists() {
        format!("{trimmed}.disabled")
    } else if base.join(trimmed).exists() {
        trimmed.to_string()
    } else {
        return Err(crate::ErrorKind::FSError(format!(
            "Could not find project file for '{project_path}' in instance"
        ))
        .into());
    };
    let current_enabled = !current_path.ends_with(".disabled");
    let enabled = desired_enabled.unwrap_or(!current_enabled);
    let new_path = if enabled {
        trimmed.to_string()
    } else {
        format!("{trimmed}.disabled")
    };
    Ok((current_path, enabled, new_path))
}

/// Renames the instance-file DB row to match the new on-disk path, falling
/// back to indexing the file when no row matches either name.
async fn rename_indexed_file(
    scope: &ContentScope,
    project_path: &str,
    current_path: &str,
    new_path: &str,
    enabled: bool,
    state: &State,
) -> crate::Result<InstanceFile> {
    let file_name = Path::new(&new_path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let file = match content_rows::rename_instance_file(
        &scope.instance.id,
        current_path,
        new_path,
        &file_name,
        enabled,
        &state.pool,
    )
    .await?
    {
        Some(file) => file,
        None if current_path != project_path => {
            match content_rows::rename_instance_file(
                &scope.instance.id,
                project_path,
                new_path,
                &file_name,
                enabled,
                &state.pool,
            )
            .await?
            {
                Some(file) => file,
                None => index_existing_file(scope, new_path, state).await?,
            }
        }
        None => index_existing_file(scope, new_path, state).await?,
    };
    Ok(file)
}

pub(crate) async fn remove_project(
    instance_id: &str,
    project_path: &str,
    state: &State,
) -> crate::Result<()> {
    let _instance_lock = state.lock_instance_content(instance_id).await;

    let scope = resolve_content_scope(instance_id, None, state).await?;
    let base = instance_full_path(state, &scope.instance);
    let file = content_rows::get_instance_file_by_relative_path(
        &scope.instance.id,
        project_path,
        &state.pool,
    )
    .await?;

    io::remove_file(base.join(project_path)).await?;

    if let Some(file) = file {
        content_rows::remove_content_entries_for_file(
            &scope.content_set_id,
            &file.id,
            &state.pool,
        )
        .await?;
        content_rows::remove_instance_file_by_relative_path(
            &scope.instance.id,
            project_path,
            &state.pool,
        )
        .await?;
    }

    Ok(())
}

pub(crate) async fn list_project_files(
    instance_id: &str,
    state: &State,
) -> crate::Result<Vec<InstalledContentFile>> {
    let scope = resolve_content_scope(instance_id, None, state).await?;
    let entries =
        content_rows::get_content_entries(&scope.content_set_id, &state.pool)
            .await?;
    let files =
        content_rows::get_instance_files(&scope.instance.id, &state.pool)
            .await?
            .into_iter()
            .map(|file| (file.id.clone(), file))
            .collect::<std::collections::HashMap<_, _>>();

    let mut output = Vec::new();
    for entry in entries {
        let Some(file_id) = entry.file_id.as_deref() else {
            continue;
        };
        let Some(file) = files.get(file_id) else {
            continue;
        };
        output.push(InstalledContentFile {
            relative_path: file.relative_path.clone(),
            provider_refs: content_rows::get_content_provider_refs(
                &entry.id,
                &state.pool,
            )
            .await?,
            enabled: entry.enabled && file.enabled,
        });
    }

    Ok(output)
}

pub(crate) fn instance_full_path(
    state: &State,
    instance: &Instance,
) -> PathBuf {
    state.directories.instances_dir().join(&instance.path)
}

async fn index_existing_file(
    scope: &ContentScope,
    relative_path: &str,
    state: &State,
) -> crate::Result<InstanceFile> {
    let full_path =
        instance_full_path(state, &scope.instance).join(relative_path);
    // Reuse the size-keyed hash cache (same key format the content scanner
    // uses) before hashing from disk: batch enable/disable of many untracked
    // files otherwise re-reads every file, which is both slow and CPU-heavy.
    let size = tokio::fs::metadata(&full_path)
        .await
        .map_err(|e| io::IOError::with_path(e, &full_path))?
        .len();
    let cache_key = format!("{size}-{}/{}", scope.instance.path, relative_path);
    let (size, sha1) = match CachedEntry::get_file_hash_many(
        &[&cache_key],
        None,
        &state.pool,
        &state.api_semaphore,
    )
    .await?
    .into_iter()
    .next()
    {
        Some(cached) => (cached.size, cached.hash),
        None => fetch::sha1_file_async(&full_path).await?,
    };
    let file_name = Path::new(relative_path)
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let project_type = ProjectType::get_from_parent_folder(relative_path)
        .ok_or_else(|| {
            crate::ErrorKind::InputError(format!(
                "Unable to infer project type from {relative_path}"
            ))
        })?;

    let local_mod_data = if project_type == ProjectType::Mod {
        // Read the file bytes, extract mod metadata
        match tokio::fs::read(&full_path).await {
            Ok(data) => {
                let bytes = bytes::Bytes::from(data);
                crate::mod_metadata::extract_mod_metadata(&bytes)
                    .and_then(|meta| serde_json::to_string(&meta).ok())
            }
            Err(_) => None,
        }
    } else {
        None
    };

    // The upserts below insert foreign-keyed rows. Take the instance lock and
    // re-resolve the scope so a concurrent instance deletion fails with a
    // clean error instead of a raw foreign-key violation; hashing above stays
    // unlocked so parallel batch operations are not serialized by it.
    let _instance_lock = state.lock_instance_content(&scope.instance.id).await;
    let scope = resolve_content_scope(&scope.instance.id, None, state).await?;

    let file = content_rows::upsert_instance_file_from_parts(
        content_rows::UpsertInstanceFile {
            instance_id: &scope.instance.id,
            relative_path,
            file_name: &file_name,
            enabled: !relative_path.ends_with(".disabled"),
            sha1: &sha1,
            size,
            missing: false,
            local_mod_data: local_mod_data.as_deref(),
            icon_path: None,
        },
        &state.pool,
    )
    .await?;
    upsert_entry_for_file(
        &scope,
        &file,
        project_type,
        ContentSourceKind::Local,
        None,
        false,
        state,
    )
    .await?;

    Ok(file)
}

async fn upsert_entry_for_file(
    scope: &ContentScope,
    file: &InstanceFile,
    project_type: ProjectType,
    source_kind: ContentSourceKind,
    provider_ref: Option<&ContentProviderRef>,
    origin: bool,
    state: &State,
) -> crate::Result<()> {
    // Serialize the foreign-keyed entry insert with instance deletion and
    // re-validate the parent rows under the lock.
    let _instance_lock = state.lock_instance_content(&scope.instance.id).await;
    let scope = resolve_content_scope(&scope.instance.id, None, state).await?;
    let pool = &state.pool;

    let entry = content_rows::upsert_content_entry_from_parts(
        content_rows::UpsertContentEntry {
            instance_id: &scope.instance.id,
            content_set_id: &scope.content_set_id,
            file_id: Some(&file.id),
            project_type,
            source_kind,
            server_requirement: ContentRequirement::Required,
            client_requirement: ContentRequirement::Required,
            enabled: file.enabled,
        },
        pool,
    )
    .await?;

    if let Some(provider_ref) = provider_ref {
        content_rows::upsert_content_provider_ref(
            &entry.id,
            provider_ref,
            origin,
            pool,
        )
        .await?;
    }

    Ok(())
}

fn infer_project_type(bytes: &Bytes) -> crate::Result<ProjectType> {
    let cursor = std::io::Cursor::new(&**bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|_| {
        crate::ErrorKind::InputError(
            "Unable to infer project type for input file".to_string(),
        )
    })?;

    if has_any_entry(
        &mut archive,
        &[
            "fabric.mod.json",
            "quilt.mod.json",
            "META-INF/neoforge.mods.toml",
            "META-INF/mods.toml",
            "mcmod.info",
        ],
    ) {
        return Ok(ProjectType::Mod);
    }
    if archive.by_name("pack.mcmeta").is_ok() {
        return classify_pack_archive(&mut archive);
    }
    if archive
        .file_names()
        .any(|name| name.starts_with("shaders/"))
    {
        return Ok(ProjectType::ShaderPack);
    }
    if archive.file_names().any(|name| name == "Metadata") {
        // .litematic files (Litematica schematic format) contain a root-level
        // "Metadata" NBT entry, which is unique among Minecraft content types.
        return Ok(ProjectType::Schematic);
    }
    Err(crate::ErrorKind::InputError(
        "Unable to infer project type for input file".to_string(),
    )
    .into())
}

/// Whether the archive contains any of the given marker files.
fn has_any_entry(
    archive: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>,
    names: &[&str],
) -> bool {
    names.iter().any(|name| archive.by_name(name).is_ok())
}

/// Distinguishes a data pack from a resource pack by the presence of a
/// top-level `data/` directory.
fn classify_pack_archive(
    archive: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>,
) -> crate::Result<ProjectType> {
    if archive.file_names().any(|name| name.starts_with("data/")) {
        Ok(ProjectType::DataPack)
    } else {
        Ok(ProjectType::ResourcePack)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::time::Duration;

    fn zip_bytes(entries: &[(&str, &[u8])]) -> Vec<u8> {
        use std::io::Write as _;
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            for (name, bytes) in entries {
                zip.start_file(*name, zip::write::FileOptions::<()>::default())
                    .expect("start entry");
                zip.write_all(bytes).expect("write entry");
            }
            zip.finish().expect("finish zip");
        }
        buf.into_inner()
    }

    #[test]
    fn wrapped_resource_pack_zip_plans_whole() {
        let bytes = zip_bytes(&[
            ("My Pack/pack.mcmeta", br#"{"pack":{"pack_format":15}}"#),
            ("My Pack/assets/minecraft/lang/en_us.json", b"{}"),
            ("My Pack/", b""),
        ]);
        assert!(
            wrapped_pack_plan(&bytes, ProjectType::ResourcePack)
                == Some(WrappedPackPlan::Whole {
                    wrapper: "My Pack".to_string()
                })
        );
    }

    #[test]
    fn wrapped_shader_pack_zip_plans_whole() {
        let bytes = zip_bytes(&[
            ("My Shader/shaders/a.fsh", b"v"),
            ("My Shader/shaders/b.vsh", b"v"),
        ]);
        assert!(
            wrapped_pack_plan(&bytes, ProjectType::ShaderPack)
                == Some(WrappedPackPlan::Whole {
                    wrapper: "My Shader".to_string()
                })
        );
    }

    #[test]
    fn flat_or_other_archives_have_no_plan() {
        let flat_rp = zip_bytes(&[
            ("pack.mcmeta", br#"{"pack":{"pack_format":15}}"#),
            ("assets/x.txt", b"x"),
        ]);
        assert_eq!(
            wrapped_pack_plan(&flat_rp, ProjectType::ResourcePack),
            None
        );
        let mod_zip = zip_bytes(&[("fabric.mod.json", b"{}")]);
        assert_eq!(wrapped_pack_plan(&mod_zip, ProjectType::Mod), None);
        let mixed = zip_bytes(&[
            ("My Pack/pack.mcmeta", br#"{"pack":{"pack_format":15}}"#),
            ("loose.txt", b"x"),
        ]);
        assert_eq!(wrapped_pack_plan(&mixed, ProjectType::ResourcePack), None);
        let no_marker = zip_bytes(&[("My Pack/assets/x.txt", b"x")]);
        assert_eq!(
            wrapped_pack_plan(&no_marker, ProjectType::ResourcePack),
            None
        );
    }

    #[test]
    fn wrapper_with_pack_children_plans_each_child() {
        let bytes = zip_bytes(&[
            ("packs/A/pack.mcmeta", br#"{"pack":{"pack_format":15}}"#),
            ("packs/A/assets/a.txt", b"x"),
            ("packs/B/pack.mcmeta", br#"{"pack":{"pack_format":15}}"#),
            ("packs/B/assets/b.txt", b"x"),
        ]);
        assert!(
            wrapped_pack_plan(&bytes, ProjectType::ResourcePack)
                == Some(WrappedPackPlan::Children {
                    children: vec!["A".to_string(), "B".to_string()]
                })
        );
    }

    #[test]
    fn shaderpack_collection_with_direct_shaders_plans_whole() {
        // A zipped shaderpacks folder that itself contains a pack folder
        // named "shaders" plus other packs: the wrapper is treated as one
        // pack (its marker sits at the wrapper root).
        let bytes = zip_bytes(&[
            ("shaderpacks/shaders/a.fsh", b"v"),
            ("shaderpacks/Other Pack/shaders/b.vsh", b"v"),
            ("shaderpacks/Another.zip", b"zip"),
        ]);
        assert!(
            wrapped_pack_plan(&bytes, ProjectType::ShaderPack)
                == Some(WrappedPackPlan::Whole {
                    wrapper: "shaderpacks".to_string()
                })
        );
    }

    #[tokio::test]
    async fn content_writes_wait_for_the_existing_sqlite_writer() {
        let directory = tempfile::tempdir().unwrap();
        let options = SqliteConnectOptions::new()
            .filename(directory.path().join("content-write.db"))
            .create_if_missing(true)
            .busy_timeout(Duration::from_secs(2));
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(options)
            .await
            .unwrap();

        let first = begin_content_write(&pool).await.unwrap();
        let second_pool = pool.clone();
        let second =
            tokio::spawn(
                async move { begin_content_write(&second_pool).await },
            );
        tokio::time::sleep(Duration::from_millis(50)).await;

        assert!(!second.is_finished());
        first.commit().await.unwrap();

        let second = tokio::time::timeout(Duration::from_secs(2), second)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        second.rollback().await.unwrap();
    }
}
