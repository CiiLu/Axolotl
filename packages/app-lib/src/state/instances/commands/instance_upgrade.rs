use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};

use chrono::{DateTime, Utc};

use crate::State;
use crate::api::curseforge::{
    CurseForgeFilesRequest, DEPENDENCY_RELATION_REQUIRED, get_file, get_files,
    get_files_many,
};
use crate::state::instances::{
    ContentItemCapabilities, ContentOwnershipKind, InstanceContentSnapshot,
    InstanceContentSnapshotItem, InstanceLink, LoaderComponentKind,
    PackMemberMaterializationState, PackMemberOverrideKind,
};
use crate::state::{
    CacheBehaviour, CachedEntry, ContentItem, ContentProvider,
    ContentProviderRef, ContentSourceKind, DependencyType,
    InstanceUpgradeAction, InstanceUpgradeDependencyChange,
    InstanceUpgradeDependencyChangeKind, InstanceUpgradeDependencyRequirement,
    InstanceUpgradeEnvironment, InstanceUpgradeFixedConstraint,
    InstanceUpgradeIssue, InstanceUpgradeIssueCode, InstanceUpgradeItem,
    InstanceUpgradeItemStatus, InstanceUpgradePlan, InstanceUpgradeResolution,
    InstanceUpgradeSelection, InstanceUpgradeSolution,
    InstanceUpgradeSolutionKind, ModrinthProjectId, ModrinthVersionId,
    ProjectType, ShaderRuntime, Version,
};

const MAX_CANDIDATES_PER_PROJECT: usize = 6;
const MAX_SEARCH_STATES: usize = 10_000;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct NodeKey {
    provider: ContentProvider,
    project_id: String,
}

impl NodeKey {
    fn new(provider: ContentProvider, project_id: impl Into<String>) -> Self {
        Self {
            provider,
            project_id: project_id.into(),
        }
    }

    fn label(&self) -> String {
        format!("{}:{}", self.provider.as_str(), self.project_id)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CandidateChannel {
    Release,
    Beta,
    Alpha,
}

impl CandidateChannel {
    fn rank(self) -> u8 {
        match self {
            Self::Release => 3,
            Self::Beta => 2,
            Self::Alpha => 1,
        }
    }

    fn is_prerelease(self) -> bool {
        self != Self::Release
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CandidateDependencyKind {
    Required,
    Optional,
    Incompatible,
    Embedded,
}

#[derive(Clone, Debug)]
struct CandidateDependency {
    key: NodeKey,
    version_id: Option<String>,
    kind: CandidateDependencyKind,
}

#[derive(Clone, Debug)]
struct UpgradeCandidate {
    key: NodeKey,
    version_id: String,
    published: DateTime<Utc>,
    channel: CandidateChannel,
    compatible: bool,
    installed_current: bool,
    dependencies: Vec<CandidateDependency>,
}

#[derive(Clone, Debug, Default)]
struct CandidatePool {
    candidates: Vec<UpgradeCandidate>,
    exploration_limited: bool,
}

type UpgradeCatalog = HashMap<NodeKey, CandidatePool>;

#[derive(Clone, Debug)]
struct InstalledAlias {
    key: NodeKey,
    current_release_id: String,
}

#[derive(Clone, Debug)]
struct InstalledNode {
    content_id: String,
    key: NodeKey,
    current_release_id: String,
    project_type: ProjectType,
    enabled: bool,
    auto_dependency: bool,
    user_owned: bool,
    migratable: bool,
    aliases: Vec<InstalledAlias>,
}

#[derive(Clone, Debug)]
struct RootRequest {
    content_id: String,
    key: NodeKey,
    current_release_id: String,
    enabled: bool,
    action: InstanceUpgradeAction,
    allow_prerelease: bool,
}

#[derive(Clone, Debug)]
struct Requirement {
    key: NodeKey,
    version_id: Option<String>,
    explicit_prerelease: bool,
    preserve_unsafe: bool,
    root_content_id: String,
    root_key: NodeKey,
    origins: Vec<InstanceUpgradeDependencyRequirement>,
}

#[derive(Clone, Debug)]
struct SolverResult {
    assignments: HashMap<NodeKey, UpgradeCandidate>,
    preserved_unsafe: HashSet<NodeKey>,
}

#[derive(Default)]
struct SearchState {
    visited: usize,
    limit_reached: bool,
    first_issue: Option<InstanceUpgradeIssue>,
}

pub(crate) async fn create_instance_upgrade_plan(
    instance_id: &str,
    target_environment: InstanceUpgradeEnvironment,
    state: &State,
) -> crate::Result<InstanceUpgradePlan> {
    let metadata = super::get_instance_metadata(instance_id, &state.pool)
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::InputError("Unknown instance".to_string())
        })?;
    if !matches!(metadata.link, InstanceLink::Unmanaged) {
        return Err(crate::ErrorKind::InputError(
            "Instance upgrade planner only supports unmanaged instances"
                .to_string(),
        )
        .into());
    }
    if metadata.applied_content_set.source_kind != ContentSourceKind::Local {
        return Err(crate::ErrorKind::InputError(
            "Instance upgrade planner only supports local content sets"
                .to_string(),
        )
        .into());
    }

    let snapshot = read_only_upgrade_snapshot(instance_id, state).await?;
    if snapshot.revision != metadata.applied_content_set.revision {
        return Err(crate::ErrorKind::InputError(
            "Instance content changed while the upgrade plan was being created; retry planning"
                .to_string(),
        )
        .into());
    }
    let source_environment = InstanceUpgradeEnvironment {
        game_version: metadata.applied_content_set.game_version.clone(),
        mod_loader: metadata.applied_content_set.loader,
        mod_loader_version: metadata.applied_content_set.loader_version.clone(),
        shader_runtime: source_shader_runtime(
            &metadata.loader_components,
            &snapshot,
        ),
    };
    let (mut items, installed) = snapshot_upgrade_items(&snapshot);
    let root_types = installed
        .iter()
        .filter(|node| !node.auto_dependency && node.migratable)
        .map(|node| (node.key.clone(), node.project_type))
        .collect::<HashMap<_, _>>();
    let catalog = load_upgrade_catalog(
        &root_types,
        &installed,
        &HashMap::new(),
        &target_environment,
        state,
    )
    .await?;
    classify_items(&mut items, &installed, &catalog, &target_environment);

    let roots = roots_from_items(&items, &installed);
    let outcome = solve_upgrade(
        &roots,
        &installed,
        &catalog,
        &HashMap::new(),
        &confirmed_prereleases(&items),
    );
    apply_solver_issues_to_items(&mut items, &outcome.issues);
    let mut blocking_issues = outcome.issues;
    for item in &items {
        if let Some(issue) = blocking_issue_for_item(item, false) {
            blocking_issues.push(issue);
        }
    }
    deduplicate_issues(&mut blocking_issues);
    let warnings = item_warnings(&items);
    let newest_solution = outcome
        .solutions
        .iter()
        .max_by(|left, right| compare_newest(left, right, &roots))
        .map(|solution| {
            materialize_solution(
                InstanceUpgradeSolutionKind::Newest,
                solution,
                &roots,
                &installed,
            )
        });
    let minimal_change_solution = outcome
        .solutions
        .iter()
        .min_by(|left, right| compare_minimal(left, right, &roots, &installed))
        .map(|solution| {
            materialize_solution(
                InstanceUpgradeSolutionKind::MinimalChange,
                solution,
                &roots,
                &installed,
            )
        });
    let selected_solution = newest_solution.clone();
    let dependency_changes = selected_solution
        .as_ref()
        .map(|solution| solution.dependency_changes.clone())
        .unwrap_or_default();

    Ok(InstanceUpgradePlan {
        id: format!("instance-upgrade-plan:{}", uuid::Uuid::new_v4()),
        instance_id: instance_id.to_string(),
        source_revision: snapshot.revision,
        source_environment,
        target_environment,
        items,
        dependency_changes,
        warnings,
        blocking_issues,
        newest_solution,
        minimal_change_solution,
        selected_solution,
        custom_constraints: Vec::new(),
    })
}

async fn read_only_upgrade_snapshot(
    instance_id: &str,
    state: &State,
) -> crate::Result<InstanceContentSnapshot> {
    let mut snapshot =
        super::get_content_snapshot(instance_id, false, state).await?;
    let instance = crate::state::instances::adapters::sqlite::instance_rows::get_instance_by_id(
        instance_id,
        &state.pool,
    )
    .await?
    .ok_or_else(|| crate::ErrorKind::InputError("Unknown instance".to_string()))?;
    let scanned =
        crate::state::instances::adapters::filesystem::scan_content_files(
            &state.directories.instances_dir(),
            &instance.path,
        )?;
    let instance_dir = state.directories.instances_dir().join(&instance.path);
    let mut scanned_by_path = HashMap::new();
    for file in scanned {
        let (_, sha1) = crate::util::fetch::sha1_file_async(
            instance_dir.join(&file.relative_path),
        )
        .await?;
        scanned_by_path.insert(file.relative_path.clone(), (file, sha1));
    }
    snapshot.items.retain(|item| {
        if !crate::state::instances::adapters::filesystem::is_scannable_project_path(
            item.project_type,
            &item.expected_relative_path,
        ) {
            return true;
        }
        scanned_by_path
            .get(&item.expected_relative_path.replace('\\', "/"))
            .is_some_and(|(_, sha1)| {
                item.content.as_ref().is_some_and(|content| {
                    content.id.eq_ignore_ascii_case(sha1)
                })
            })
    });
    let represented = snapshot
        .items
        .iter()
        .map(|item| item.expected_relative_path.replace('\\', "/"))
        .collect::<HashSet<_>>();
    for (_, (file, sha1)) in scanned_by_path {
        if represented.contains(&file.relative_path) {
            continue;
        }
        let Some(project_type) = crate::state::instances::adapters::filesystem::project_type_from_relative_path(
            &file.relative_path,
        ) else {
            continue;
        };
        snapshot.items.push(InstanceContentSnapshotItem {
            file_id: None,
            entry_id: None,
            member_id: None,
            ownership_kind: ContentOwnershipKind::LocalDiscovered,
            materialization_state: PackMemberMaterializationState::Present,
            override_kind: PackMemberOverrideKind::None,
            expected_relative_path: file.relative_path.clone(),
            required: false,
            project_type,
            provider: None,
            provider_project_id: None,
            provider_release_id: None,
            content: Some(ContentItem {
                file_name: file.file_name,
                file_path: file.relative_path,
                id: sha1,
                size: file.size,
                enabled: file.enabled,
                project_type,
                project: None,
                version: None,
                owner: None,
                update: None,
                date_added: None,
                provider_refs: Vec::new(),
                origin_provider: None,
                rollback: None,
                environment: None,
                source_kind: None,
                external: true,
                loader: None,
            }),
            capabilities: ContentItemCapabilities::default(),
            dependency: None,
        });
    }
    Ok(snapshot)
}

fn source_shader_runtime(
    components: &[crate::state::LoaderComponent],
    snapshot: &InstanceContentSnapshot,
) -> ShaderRuntime {
    if snapshot.items.iter().any(item_has_iris_identity) {
        return ShaderRuntime::Iris;
    }
    if components
        .iter()
        .any(|component| component.kind == LoaderComponentKind::OptiFine)
    {
        return ShaderRuntime::OptiFine;
    }
    if snapshot.items.iter().any(|item| {
        item.project_type == ProjectType::Mod
            && item
                .content
                .as_ref()
                .is_none_or(|content| content.provider_refs.is_empty())
    }) {
        ShaderRuntime::Unknown
    } else {
        ShaderRuntime::None
    }
}

fn item_has_iris_identity(item: &InstanceContentSnapshotItem) -> bool {
    (item.provider == Some(ContentProvider::Modrinth)
        && item.provider_project_id.as_deref() == Some("YL57xq9U"))
        || item.content.as_ref().is_some_and(|content| {
            content.provider_refs.iter().any(|reference| {
                matches!(reference, ContentProviderRef::Modrinth { project_id, .. } if project_id.as_str() == "YL57xq9U")
            })
        })
}

fn snapshot_upgrade_items(
    snapshot: &InstanceContentSnapshot,
) -> (Vec<InstanceUpgradeItem>, Vec<InstalledNode>) {
    let mut items = Vec::new();
    let mut installed = Vec::new();
    for item in &snapshot.items {
        let content_id = stable_content_id(item);
        let current_enabled =
            item.content.as_ref().is_none_or(|content| content.enabled);
        let auto_dependency = item
            .dependency
            .as_ref()
            .is_some_and(|dependency| dependency.auto_dependency);
        let unsupported = is_world_datapack(&item.expected_relative_path)
            || matches!(
                item.project_type,
                ProjectType::Schematic | ProjectType::WorldSave
            );
        let recognized = item.provider.zip(item.provider_project_id.clone());
        let status = if unsupported {
            InstanceUpgradeItemStatus::UnsupportedContentType
        } else if recognized.is_none() {
            InstanceUpgradeItemStatus::Unidentified
        } else {
            InstanceUpgradeItemStatus::NoCompatibleRelease
        };
        let action = if unsupported || recognized.is_none() {
            InstanceUpgradeAction::Keep
        } else {
            InstanceUpgradeAction::Upgrade
        };
        items.push(InstanceUpgradeItem {
            content_id: content_id.clone(),
            relative_path: item.expected_relative_path.clone(),
            project_type: item.project_type,
            provider: item.provider,
            project_id: item.provider_project_id.clone(),
            current_release_id: item.provider_release_id.clone(),
            current_enabled,
            auto_dependency,
            status,
            resolution: InstanceUpgradeResolution {
                content_id: content_id.clone(),
                action,
                allow_prerelease: false,
                confirmed_prerelease_dependencies: Vec::new(),
            },
            candidate_release_ids: Vec::new(),
        });
        if let (Some((provider, project_id)), Some(current_release_id)) =
            (recognized, item.provider_release_id.clone())
        {
            installed.push(InstalledNode {
                content_id,
                key: NodeKey::new(provider, project_id.clone()),
                current_release_id: current_release_id.clone(),
                project_type: item.project_type,
                enabled: current_enabled,
                auto_dependency,
                user_owned: item.ownership_kind
                    == ContentOwnershipKind::UserAdded
                    && !auto_dependency,
                migratable: !unsupported,
                aliases: installed_aliases(
                    item,
                    provider,
                    &project_id,
                    &current_release_id,
                ),
            });
        }
    }
    (items, installed)
}

fn installed_aliases(
    item: &InstanceContentSnapshotItem,
    primary_provider: ContentProvider,
    primary_project_id: &str,
    primary_release_id: &str,
) -> Vec<InstalledAlias> {
    let mut aliases = vec![InstalledAlias {
        key: NodeKey::new(primary_provider, primary_project_id),
        current_release_id: primary_release_id.to_string(),
    }];
    if let Some(content) = item.content.as_ref() {
        for reference in &content.provider_refs {
            let Some(release_id) = reference.database_release_id() else {
                continue;
            };
            let alias = InstalledAlias {
                key: NodeKey::new(
                    reference.provider(),
                    reference.database_project_id(),
                ),
                current_release_id: release_id,
            };
            if !aliases.iter().any(|existing| existing.key == alias.key) {
                aliases.push(alias);
            }
        }
    }
    aliases
}

fn stable_content_id(item: &InstanceContentSnapshotItem) -> String {
    item.entry_id
        .as_deref()
        .or(item.member_id.as_deref())
        .or(item.file_id.as_deref())
        .unwrap_or(&item.expected_relative_path)
        .to_string()
}

fn is_world_datapack(path: &str) -> bool {
    let components = path
        .replace('\\', "/")
        .split('/')
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>();
    components.len() >= 4
        && components[0] == "saves"
        && components[2] == "datapacks"
}

struct SolveOutcome {
    solutions: Vec<SolverResult>,
    issues: Vec<InstanceUpgradeIssue>,
}

pub(crate) async fn recompute_instance_upgrade_plan(
    plan: &mut InstanceUpgradePlan,
    fixed_constraints: &[InstanceUpgradeFixedConstraint],
    selected_kind: InstanceUpgradeSolutionKind,
    state: &State,
) -> crate::Result<()> {
    let snapshot = read_only_upgrade_snapshot(&plan.instance_id, state).await?;
    let (_, installed) = snapshot_upgrade_items(&snapshot);
    let root_types = installed
        .iter()
        .filter(|node| !node.auto_dependency && node.migratable)
        .map(|node| (node.key.clone(), node.project_type))
        .collect::<HashMap<_, _>>();
    let fixed = fixed_constraints
        .iter()
        .map(|constraint| {
            (
                NodeKey::new(constraint.provider, &constraint.project_id),
                constraint.version_id.clone(),
            )
        })
        .collect::<HashMap<_, _>>();
    let catalog = load_upgrade_catalog(
        &root_types,
        &installed,
        &fixed,
        &plan.target_environment,
        state,
    )
    .await?;
    classify_items(
        &mut plan.items,
        &installed,
        &catalog,
        &plan.target_environment,
    );
    let roots = roots_from_items(&plan.items, &installed);
    let outcome = solve_upgrade(
        &roots,
        &installed,
        &catalog,
        &fixed,
        &confirmed_prereleases(&plan.items),
    );
    apply_solver_issues_to_items(&mut plan.items, &outcome.issues);
    let mut blocking_issues = outcome.issues;
    for item in &plan.items {
        let fixed_prerelease = item
            .provider
            .zip(item.project_id.as_ref())
            .is_some_and(|(provider, project_id)| {
                fixed.contains_key(&NodeKey::new(provider, project_id))
            });
        if let Some(issue) = blocking_issue_for_item(item, fixed_prerelease) {
            blocking_issues.push(issue);
        }
    }
    deduplicate_issues(&mut blocking_issues);
    plan.warnings = item_warnings_with_fixed(&plan.items, &fixed);
    plan.blocking_issues = blocking_issues;
    plan.newest_solution = outcome
        .solutions
        .iter()
        .max_by(|left, right| compare_newest(left, right, &roots))
        .map(|solution| {
            materialize_solution(
                InstanceUpgradeSolutionKind::Newest,
                solution,
                &roots,
                &installed,
            )
        });
    plan.minimal_change_solution = outcome
        .solutions
        .iter()
        .min_by(|left, right| compare_minimal(left, right, &roots, &installed))
        .map(|solution| {
            materialize_solution(
                InstanceUpgradeSolutionKind::MinimalChange,
                solution,
                &roots,
                &installed,
            )
        });
    plan.selected_solution = match selected_kind {
        InstanceUpgradeSolutionKind::Newest => plan.newest_solution.clone(),
        InstanceUpgradeSolutionKind::MinimalChange => {
            plan.minimal_change_solution.clone()
        }
        InstanceUpgradeSolutionKind::Custom => {
            outcome.solutions.first().map(|solution| {
                materialize_solution(
                    InstanceUpgradeSolutionKind::Custom,
                    solution,
                    &roots,
                    &installed,
                )
            })
        }
    };
    plan.dependency_changes = plan
        .selected_solution
        .as_ref()
        .map(|solution| solution.dependency_changes.clone())
        .unwrap_or_default();
    Ok(())
}

async fn load_upgrade_catalog(
    root_types: &HashMap<NodeKey, ProjectType>,
    installed: &[InstalledNode],
    fixed: &HashMap<NodeKey, String>,
    target: &InstanceUpgradeEnvironment,
    state: &State,
) -> crate::Result<UpgradeCatalog> {
    let current_versions = installed
        .iter()
        .flat_map(|node| {
            node.aliases.iter().map(|alias| {
                (alias.key.clone(), alias.current_release_id.clone())
            })
        })
        .collect::<HashMap<_, _>>();
    let mut catalog = HashMap::new();
    let mut queue = root_types
        .iter()
        .map(|(key, project_type)| (key.clone(), *project_type))
        .collect::<VecDeque<_>>();
    let mut seen = HashSet::new();
    let mut exact_versions = fixed
        .iter()
        .map(|(key, version_id)| {
            (key.clone(), HashSet::from([version_id.clone()]))
        })
        .collect::<HashMap<NodeKey, HashSet<String>>>();
    while let Some((key, project_type)) = queue.pop_front() {
        if !seen.insert(key.clone()) {
            continue;
        }
        let current = current_versions.get(&key).map(String::as_str);
        let exact = exact_versions.get(&key).cloned().unwrap_or_default();
        let candidates = match key.provider {
            ContentProvider::Modrinth => {
                load_modrinth_candidates(
                    &key,
                    project_type,
                    current,
                    &exact,
                    fixed.get(&key).map(String::as_str),
                    target,
                    state,
                )
                .await?
            }
            ContentProvider::CurseForge => {
                load_curseforge_candidates(
                    &key,
                    project_type,
                    current,
                    &exact,
                    fixed.get(&key).map(String::as_str),
                    target,
                )
                .await?
            }
            ContentProvider::Local => CandidatePool::default(),
        };
        for candidate in &candidates.candidates {
            for dependency in &candidate.dependencies {
                if dependency.kind == CandidateDependencyKind::Required {
                    if let Some(version_id) = dependency.version_id.as_ref()
                        && exact_versions
                            .entry(dependency.key.clone())
                            .or_default()
                            .insert(version_id.clone())
                        && seen.remove(&dependency.key)
                    {
                        queue.push_back((
                            dependency.key.clone(),
                            ProjectType::Mod,
                        ));
                    }
                    queue.push_back((dependency.key.clone(), ProjectType::Mod));
                }
            }
        }
        catalog.insert(key, candidates);
    }
    for node in installed {
        for alias in &node.aliases {
            let pool = catalog.entry(alias.key.clone()).or_default();
            if let Some(candidate) =
                pool.candidates.iter_mut().find(|candidate| {
                    candidate.version_id == alias.current_release_id
                })
            {
                candidate.installed_current = true;
            } else {
                pool.candidates.push(UpgradeCandidate {
                    key: alias.key.clone(),
                    version_id: alias.current_release_id.clone(),
                    published: DateTime::<Utc>::MIN_UTC,
                    channel: CandidateChannel::Release,
                    compatible: false,
                    installed_current: true,
                    dependencies: Vec::new(),
                });
            }
        }
    }
    Ok(catalog)
}

async fn load_modrinth_candidates(
    key: &NodeKey,
    project_type: ProjectType,
    current_release_id: Option<&str>,
    exact_versions: &HashSet<String>,
    custom_fixed_version: Option<&str>,
    target: &InstanceUpgradeEnvironment,
    state: &State,
) -> crate::Result<CandidatePool> {
    let project_id = ModrinthProjectId::new(key.project_id.clone())?;
    let mut versions = CachedEntry::get_project_versions(
        &project_id,
        Some(CacheBehaviour::MustRevalidate),
        &state.pool,
        &state.api_semaphore,
    )
    .await?
    .unwrap_or_default();
    versions.sort_by(compare_modrinth_version);
    let (mut selected, exploration_limited) =
        bounded_compatible_candidates(&versions, |version| {
            modrinth_version_matches(version, project_type, target)
        });
    if let Some(current_release_id) = current_release_id
        && !selected
            .iter()
            .any(|version| version.id == current_release_id)
    {
        let current = versions
            .iter()
            .find(|version| version.id == current_release_id)
            .cloned()
            .or(CachedEntry::get_version(
                &ModrinthVersionId::new(current_release_id.to_string())?,
                Some(CacheBehaviour::MustRevalidate),
                &state.pool,
                &state.api_semaphore,
            )
            .await?);
        if let Some(current) = current {
            selected.push(current);
        }
    }
    for exact_version in exact_versions {
        let already_selected =
            selected.iter().any(|version| version.id == *exact_version);
        if already_selected
            && custom_fixed_version != Some(exact_version.as_str())
        {
            continue;
        }
        let exact = selected
            .iter()
            .find(|version| version.id == *exact_version)
            .cloned()
            .or_else(|| {
                versions
                    .iter()
                    .find(|version| version.id == *exact_version)
                    .cloned()
            })
            .or(CachedEntry::get_version(
                &ModrinthVersionId::new(exact_version.clone())?,
                Some(CacheBehaviour::MustRevalidate),
                &state.pool,
                &state.api_semaphore,
            )
            .await?);
        if let Some(exact) = exact {
            if custom_fixed_version == Some(exact_version.as_str()) {
                validate_modrinth_custom_fixed(
                    key,
                    &exact,
                    project_type,
                    target,
                )?;
            } else if exact.project_id != key.project_id {
                continue;
            }
            if !already_selected {
                selected.push(exact);
            }
        } else if custom_fixed_version == Some(exact_version.as_str()) {
            return Err(crate::ErrorKind::InputError(format!(
                "Unknown custom fixed Modrinth version {exact_version}"
            ))
            .into());
        }
    }

    let mut candidates = Vec::new();
    for version in selected {
        let installed_current = current_release_id == Some(version.id.as_str());
        let mut dependencies = Vec::new();
        for dependency in &version.dependencies {
            let project_id = match dependency.project_id.clone() {
                Some(project_id) => project_id,
                None => match dependency.version_id.as_deref() {
                    Some(version_id) => CachedEntry::get_version(
                        &ModrinthVersionId::new(version_id.to_string())?,
                        Some(CacheBehaviour::MustRevalidate),
                        &state.pool,
                        &state.api_semaphore,
                    )
                    .await?
                    .map(|version| version.project_id)
                    .unwrap_or_else(|| format!("missing-version:{version_id}")),
                    None => continue,
                },
            };
            dependencies.push(CandidateDependency {
                key: NodeKey::new(ContentProvider::Modrinth, project_id),
                version_id: dependency.version_id.clone(),
                kind: match dependency.dependency_type {
                    DependencyType::Required => {
                        CandidateDependencyKind::Required
                    }
                    DependencyType::Optional => {
                        CandidateDependencyKind::Optional
                    }
                    DependencyType::Incompatible => {
                        CandidateDependencyKind::Incompatible
                    }
                    DependencyType::Embedded => {
                        CandidateDependencyKind::Embedded
                    }
                },
            });
        }
        let compatible =
            modrinth_version_matches(&version, project_type, target);
        candidates.push(UpgradeCandidate {
            key: key.clone(),
            version_id: version.id,
            published: version.date_published,
            channel: modrinth_channel(&version.version_type),
            compatible,
            installed_current,
            dependencies,
        });
    }
    sort_candidates(&mut candidates);
    Ok(CandidatePool {
        candidates,
        exploration_limited,
    })
}

fn validate_modrinth_custom_fixed(
    key: &NodeKey,
    version: &Version,
    project_type: ProjectType,
    target: &InstanceUpgradeEnvironment,
) -> crate::Result<()> {
    if version.project_id != key.project_id {
        return Err(crate::ErrorKind::InputError(format!(
            "Custom fixed Modrinth version {} belongs to project {}, not {}",
            version.id, version.project_id, key.project_id
        ))
        .into());
    }
    if !modrinth_version_matches(version, project_type, target) {
        return Err(crate::ErrorKind::InputError(format!(
            "Custom fixed Modrinth version {} is not compatible with the target environment",
            version.id
        ))
        .into());
    }
    Ok(())
}

async fn load_curseforge_candidates(
    key: &NodeKey,
    project_type: ProjectType,
    current_release_id: Option<&str>,
    exact_versions: &HashSet<String>,
    custom_fixed_version: Option<&str>,
    target: &InstanceUpgradeEnvironment,
) -> crate::Result<CandidatePool> {
    let project_id = key.project_id.parse::<u32>().map_err(|_| {
        crate::ErrorKind::InputError(format!(
            "Invalid CurseForge project ID {}",
            key.project_id
        ))
    })?;
    let mut files = get_files(
        project_id,
        CurseForgeFilesRequest {
            game_version: Some(target.game_version.clone()),
            mod_loader_type: (project_type == ProjectType::Mod)
                .then(|| curseforge_loader_type(target.mod_loader))
                .flatten(),
            game_version_type_id: None,
            index: 0,
            page_size: 50,
        },
    )
    .await?
    .files;
    files.sort_by(|left, right| {
        curseforge_channel(right.release_type)
            .rank()
            .cmp(&curseforge_channel(left.release_type).rank())
            .then_with(|| right.file_date.cmp(&left.file_date))
            .then_with(|| right.id.cmp(&left.id))
    });
    let (mut selected, exploration_limited) =
        bounded_compatible_candidates(&files, |file| {
            file.is_available
                && curseforge_file_matches(file, project_type, target)
        });
    if let Some(current_release_id) = current_release_id
        && !selected
            .iter()
            .any(|file| file.id.to_string() == current_release_id)
    {
        let current = files
            .iter()
            .find(|file| file.id.to_string() == current_release_id)
            .cloned();
        let current = match current {
            Some(current) => Some(current),
            None => match current_release_id.parse::<u32>() {
                Ok(file_id) => Some(get_file(project_id, file_id).await?),
                Err(_) => None,
            },
        };
        if let Some(current) = current {
            selected.push(current);
        }
    }
    for exact_version in exact_versions {
        let already_selected = selected
            .iter()
            .any(|file| file.id.to_string() == *exact_version);
        if already_selected
            && custom_fixed_version != Some(exact_version.as_str())
        {
            continue;
        }
        let file_id = exact_version.parse::<u32>().map_err(|_| {
            crate::ErrorKind::InputError(format!(
                "Invalid CurseForge file ID {exact_version}"
            ))
        })?;
        let exact =
            match selected.iter().find(|file| file.id == file_id).cloned() {
                Some(file) => file,
                None => get_files_many(vec![file_id])
                    .await?
                    .into_iter()
                    .next()
                    .ok_or_else(|| {
                        crate::ErrorKind::InputError(format!(
                            "Unknown CurseForge file ID {exact_version}"
                        ))
                    })?,
            };
        if exact.mod_id != project_id {
            return Err(crate::ErrorKind::InputError(format!(
                "Fixed or exact CurseForge file {exact_version} belongs to project {}, not {project_id}",
                exact.mod_id
            ))
            .into());
        }
        if custom_fixed_version == Some(exact_version.as_str())
            && !curseforge_file_matches(&exact, project_type, target)
        {
            return Err(crate::ErrorKind::InputError(format!(
                "Custom fixed CurseForge file {exact_version} is not compatible with the target environment"
            ))
            .into());
        }
        if !already_selected {
            selected.push(exact);
        }
    }
    let mut candidates = selected
        .into_iter()
        .map(|file| {
            let installed_current = current_release_id
                .is_some_and(|current| current == file.id.to_string());
            UpgradeCandidate {
                key: key.clone(),
                version_id: file.id.to_string(),
                published: DateTime::parse_from_rfc3339(&file.file_date)
                    .map(|date| date.with_timezone(&Utc))
                    .unwrap_or(DateTime::<Utc>::MIN_UTC),
                channel: curseforge_channel(file.release_type),
                compatible: curseforge_file_matches(
                    &file,
                    project_type,
                    target,
                ),
                installed_current,
                dependencies: file
                    .dependencies
                    .into_iter()
                    .filter_map(|dependency| {
                        let kind = match dependency.relation_type {
                            DEPENDENCY_RELATION_REQUIRED | 6 => {
                                CandidateDependencyKind::Required
                            }
                            2 => CandidateDependencyKind::Optional,
                            5 => CandidateDependencyKind::Incompatible,
                            1 | 4 => CandidateDependencyKind::Embedded,
                            _ => return None,
                        };
                        Some(CandidateDependency {
                            key: NodeKey::new(
                                ContentProvider::CurseForge,
                                dependency.mod_id.to_string(),
                            ),
                            version_id: None,
                            kind,
                        })
                    })
                    .collect(),
            }
        })
        .collect::<Vec<_>>();
    sort_candidates(&mut candidates);
    Ok(CandidatePool {
        candidates,
        exploration_limited,
    })
}

fn bounded_compatible_candidates<T: Clone>(
    candidates: &[T],
    mut compatible: impl FnMut(&T) -> bool,
) -> (Vec<T>, bool) {
    let mut selected = Vec::new();
    let mut compatible_count = 0;
    for candidate in candidates {
        if !compatible(candidate) {
            continue;
        }
        compatible_count += 1;
        if selected.len() < MAX_CANDIDATES_PER_PROJECT {
            selected.push(candidate.clone());
        }
    }
    (selected, compatible_count > MAX_CANDIDATES_PER_PROJECT)
}

fn curseforge_loader_type(loader: crate::state::ModLoader) -> Option<u32> {
    match loader {
        crate::state::ModLoader::Forge => Some(1),
        crate::state::ModLoader::Fabric => Some(4),
        crate::state::ModLoader::Quilt => Some(5),
        crate::state::ModLoader::NeoForge => Some(6),
        _ => None,
    }
}

fn compare_modrinth_version(left: &Version, right: &Version) -> Ordering {
    modrinth_channel(&right.version_type)
        .rank()
        .cmp(&modrinth_channel(&left.version_type).rank())
        .then_with(|| right.date_published.cmp(&left.date_published))
        .then_with(|| right.id.cmp(&left.id))
}

fn modrinth_channel(version_type: &str) -> CandidateChannel {
    if version_type.eq_ignore_ascii_case("beta") {
        CandidateChannel::Beta
    } else if version_type.eq_ignore_ascii_case("alpha") {
        CandidateChannel::Alpha
    } else {
        CandidateChannel::Release
    }
}

fn curseforge_channel(release_type: u32) -> CandidateChannel {
    match release_type {
        1 => CandidateChannel::Release,
        2 => CandidateChannel::Beta,
        _ => CandidateChannel::Alpha,
    }
}

fn sort_candidates(candidates: &mut [UpgradeCandidate]) {
    candidates.sort_by(|left, right| {
        right
            .channel
            .rank()
            .cmp(&left.channel.rank())
            .then_with(|| right.published.cmp(&left.published))
            .then_with(|| right.version_id.cmp(&left.version_id))
    });
}

fn modrinth_version_matches(
    version: &Version,
    project_type: ProjectType,
    target: &InstanceUpgradeEnvironment,
) -> bool {
    if !version
        .game_versions
        .iter()
        .any(|game_version| game_version == &target.game_version)
    {
        return false;
    }
    match project_type {
        ProjectType::Mod => version.loaders.iter().any(|loader| {
            loader.eq_ignore_ascii_case(target.mod_loader.as_str())
        }),
        ProjectType::ShaderPack => shader_loader_matches(
            version.loaders.iter().map(String::as_str),
            target.shader_runtime,
        ),
        ProjectType::DataPack => version
            .loaders
            .iter()
            .any(|loader| loader.eq_ignore_ascii_case("datapack")),
        ProjectType::ResourcePack => version.loaders.iter().any(|loader| {
            loader.eq_ignore_ascii_case("minecraft")
                || loader.eq_ignore_ascii_case("vanilla")
        }),
        ProjectType::Schematic | ProjectType::WorldSave => false,
    }
}

fn curseforge_file_matches(
    file: &crate::api::curseforge::CurseForgeFile,
    project_type: ProjectType,
    target: &InstanceUpgradeEnvironment,
) -> bool {
    let game_version_matches = file
        .game_versions
        .iter()
        .any(|value| value == &target.game_version)
        || file.sortable_game_versions.iter().any(|value| {
            value.game_version.as_deref() == Some(&target.game_version)
                || value.game_version_name == target.game_version
        });
    if !game_version_matches {
        return false;
    }
    match project_type {
        ProjectType::Mod => file.game_versions.iter().any(|value| {
            value.eq_ignore_ascii_case(target.mod_loader.as_str())
        }),
        ProjectType::ShaderPack => shader_loader_matches(
            file.game_versions.iter().map(String::as_str),
            target.shader_runtime,
        ),
        ProjectType::DataPack | ProjectType::ResourcePack => true,
        ProjectType::Schematic | ProjectType::WorldSave => false,
    }
}

fn shader_loader_matches<'a>(
    loaders: impl Iterator<Item = &'a str>,
    runtime: ShaderRuntime,
) -> bool {
    let expected = match runtime {
        ShaderRuntime::Iris => "iris",
        ShaderRuntime::OptiFine => "optifine",
        ShaderRuntime::None | ShaderRuntime::Unknown => return false,
    };
    loaders.into_iter().any(|loader| {
        loader.eq_ignore_ascii_case(expected)
            || runtime == ShaderRuntime::OptiFine
                && loader.eq_ignore_ascii_case("optifine")
    })
}

fn classify_items(
    items: &mut [InstanceUpgradeItem],
    installed: &[InstalledNode],
    catalog: &UpgradeCatalog,
    target: &InstanceUpgradeEnvironment,
) {
    for item in items {
        if matches!(
            item.status,
            InstanceUpgradeItemStatus::Unidentified
                | InstanceUpgradeItemStatus::UnsupportedContentType
        ) {
            continue;
        }
        if item.project_type == ProjectType::ShaderPack {
            match target.shader_runtime {
                ShaderRuntime::None => {
                    item.status =
                        InstanceUpgradeItemStatus::ShaderRuntimeMissing;
                    continue;
                }
                ShaderRuntime::Unknown => {
                    item.status =
                        InstanceUpgradeItemStatus::ShaderRuntimeUnknown;
                    continue;
                }
                ShaderRuntime::Iris | ShaderRuntime::OptiFine => {}
            }
        }
        let Some(node) = installed
            .iter()
            .find(|node| node.content_id == item.content_id)
        else {
            continue;
        };
        let candidates = catalog
            .get(&node.key)
            .map(|pool| pool.candidates.as_slice())
            .unwrap_or(&[]);
        let compatible = candidates
            .iter()
            .filter(|candidate| candidate.compatible)
            .collect::<Vec<_>>();
        item.candidate_release_ids = compatible
            .iter()
            .map(|candidate| candidate.version_id.clone())
            .collect();
        let has_stable = compatible
            .iter()
            .any(|candidate| candidate.channel == CandidateChannel::Release);
        let current_compatible = compatible
            .iter()
            .any(|candidate| candidate.version_id == node.current_release_id);
        item.status = if has_stable {
            if current_compatible {
                InstanceUpgradeItemStatus::AlreadyCompatible
            } else {
                InstanceUpgradeItemStatus::UpgradeAvailable
            }
        } else if !compatible.is_empty() {
            InstanceUpgradeItemStatus::PrereleaseOnly
        } else if item.project_type == ProjectType::ShaderPack {
            InstanceUpgradeItemStatus::NoCompatibleShaderRuntime
        } else {
            InstanceUpgradeItemStatus::NoCompatibleRelease
        };
    }
}

fn roots_from_items(
    items: &[InstanceUpgradeItem],
    installed: &[InstalledNode],
) -> Vec<RootRequest> {
    items
        .iter()
        .filter_map(|item| {
            let node = installed.iter().find(|node| {
                node.content_id == item.content_id
                    && !node.auto_dependency
                    && node.migratable
            })?;
            Some(RootRequest {
                content_id: item.content_id.clone(),
                key: node.key.clone(),
                current_release_id: node.current_release_id.clone(),
                enabled: node.enabled,
                action: item.resolution.action,
                allow_prerelease: item.resolution.allow_prerelease,
            })
        })
        .collect()
}

fn confirmed_prereleases(
    items: &[InstanceUpgradeItem],
) -> HashSet<(NodeKey, String)> {
    items
        .iter()
        .flat_map(|item| {
            item.resolution.confirmed_prerelease_dependencies.iter()
        })
        .map(|confirmation| {
            (
                NodeKey::new(confirmation.provider, &confirmation.project_id),
                confirmation.version_id.clone(),
            )
        })
        .collect()
}

fn solve_upgrade(
    roots: &[RootRequest],
    installed: &[InstalledNode],
    catalog: &UpgradeCatalog,
    fixed: &HashMap<NodeKey, String>,
    confirmed_prereleases: &HashSet<(NodeKey, String)>,
) -> SolveOutcome {
    let mut requirements = roots
        .iter()
        .map(|root| Requirement {
            key: root.key.clone(),
            version_id: fixed.get(&root.key).cloned(),
            explicit_prerelease: fixed.contains_key(&root.key),
            preserve_unsafe: root.action != InstanceUpgradeAction::Upgrade,
            root_content_id: root.content_id.clone(),
            root_key: root.key.clone(),
            origins: Vec::new(),
        })
        .collect::<Vec<_>>();
    requirements.sort_by_key(|requirement| {
        std::cmp::Reverse(requirement.preserve_unsafe)
    });
    let mut state = SearchState::default();
    let mut solutions = Vec::new();
    search_solutions(
        requirements,
        HashMap::new(),
        HashMap::new(),
        HashSet::new(),
        roots,
        catalog,
        confirmed_prereleases,
        &mut state,
        &mut solutions,
    );
    let mut issues = Vec::new();
    let candidate_limit_reached = solutions.is_empty()
        && catalog.values().any(|pool| pool.exploration_limited);
    if state.limit_reached || candidate_limit_reached {
        issues.push(issue(
            InstanceUpgradeIssueCode::SearchLimitReached,
            if state.limit_reached {
                "Upgrade dependency search reached its global state limit"
            } else {
                "Upgrade dependency search exhausted its bounded candidate exploration and cannot prove the plan is unsatisfiable"
            },
            None,
            None,
            None,
        ));
    }
    if solutions.is_empty() && !state.limit_reached && !candidate_limit_reached
    {
        issues.push(state.first_issue.unwrap_or_else(|| {
            issue(
                InstanceUpgradeIssueCode::DependencyConflict,
                "No globally compatible dependency solution exists",
                None,
                None,
                None,
            )
        }));
    }
    let _ = installed;
    SolveOutcome { solutions, issues }
}

fn search_solutions(
    mut requirements: Vec<Requirement>,
    assignments: HashMap<NodeKey, UpgradeCandidate>,
    assignment_origins: HashMap<
        NodeKey,
        Vec<InstanceUpgradeDependencyRequirement>,
    >,
    preserved_unsafe: HashSet<NodeKey>,
    roots: &[RootRequest],
    catalog: &UpgradeCatalog,
    confirmed_prereleases: &HashSet<(NodeKey, String)>,
    state: &mut SearchState,
    solutions: &mut Vec<SolverResult>,
) {
    if state.visited >= MAX_SEARCH_STATES {
        state.limit_reached = true;
        return;
    }
    state.visited += 1;
    let Some(requirement) = requirements.pop() else {
        solutions.push(SolverResult {
            assignments,
            preserved_unsafe,
        });
        return;
    };
    if let Some(selected) = assignments.get(&requirement.key) {
        let exact_matches = requirement
            .version_id
            .as_ref()
            .is_none_or(|version_id| version_id == &selected.version_id);
        let safe_assignment_satisfies_unsafe = requirement.preserve_unsafe
            && !preserved_unsafe.contains(&requirement.key);
        if exact_matches || safe_assignment_satisfies_unsafe {
            let mut next_origins = assignment_origins;
            next_origins
                .entry(requirement.key.clone())
                .or_default()
                .extend(requirement.origins);
            search_solutions(
                requirements,
                assignments,
                next_origins,
                preserved_unsafe,
                roots,
                catalog,
                confirmed_prereleases,
                state,
                solutions,
            );
        } else {
            let mut details = assignment_origins
                .get(&requirement.key)
                .cloned()
                .unwrap_or_default();
            details.extend(requirement.origins.clone());
            record_issue(
                state,
                issue_with_requirements(
                    InstanceUpgradeIssueCode::DependencyConflict,
                    format!(
                        "{} requires conflicting exact dependency versions",
                        requirement.key.label()
                    ),
                    Some(&requirement.key),
                    Some(&requirement.key.project_id),
                    None,
                    details,
                ),
            );
        }
        return;
    }

    let candidates = candidates_for_requirement(
        &requirement,
        roots,
        catalog,
        confirmed_prereleases,
    );
    if candidates.is_empty() {
        let prerelease_candidate = catalog
            .get(&requirement.key)
            .into_iter()
            .flat_map(|pool| &pool.candidates)
            .find(|candidate| {
                candidate.compatible
                    && candidate.channel.is_prerelease()
                    && requirement.version_id.as_ref().is_none_or(
                        |version_id| version_id == &candidate.version_id,
                    )
            });
        if requirement.preserve_unsafe {
            search_solutions(
                requirements,
                assignments,
                assignment_origins,
                preserved_unsafe,
                roots,
                catalog,
                confirmed_prereleases,
                state,
                solutions,
            );
            return;
        }
        let root_requirement =
            roots.iter().any(|root| root.key == requirement.key);
        let code = if prerelease_candidate.is_some() {
            InstanceUpgradeIssueCode::PrereleaseOnly
        } else if root_requirement {
            InstanceUpgradeIssueCode::NoCompatibleRelease
        } else {
            InstanceUpgradeIssueCode::MissingRequiredDependency
        };
        let mut details = requirement.origins.clone();
        if let Some(candidate) = prerelease_candidate {
            for detail in &mut details {
                detail.candidate_release_id =
                    Some(candidate.version_id.clone());
            }
        }
        record_issue(
            state,
            issue_with_requirements(
                code,
                format!(
                    "No compatible release satisfies required project {}",
                    requirement.key.label()
                ),
                Some(&requirement.key),
                Some(&requirement.key.project_id),
                None,
                details,
            ),
        );
        return;
    }
    for candidate in candidates {
        if let Some(conflict) =
            incompatible_with_assignments(candidate, &assignments)
        {
            let mut details = requirement.origins.clone();
            details.push(InstanceUpgradeDependencyRequirement {
                root_content_id: requirement.root_content_id.clone(),
                root_provider: requirement.root_key.provider,
                root_project_id: requirement.root_key.project_id.clone(),
                parent_provider: candidate.key.provider,
                parent_project_id: candidate.key.project_id.clone(),
                parent_release_id: candidate.version_id.clone(),
                dependency_provider: conflict.provider,
                dependency_project_id: conflict.project_id.clone(),
                required_release_id: None,
                candidate_release_id: assignments
                    .get(&conflict)
                    .map(|selected| selected.version_id.clone()),
            });
            if let Some(conflicting_root) =
                roots.iter().find(|root| root.key == conflict)
                && let Some(selected) = assignments.get(&conflict)
            {
                details.push(InstanceUpgradeDependencyRequirement {
                    root_content_id: conflicting_root.content_id.clone(),
                    root_provider: conflicting_root.key.provider,
                    root_project_id: conflicting_root.key.project_id.clone(),
                    parent_provider: selected.key.provider,
                    parent_project_id: selected.key.project_id.clone(),
                    parent_release_id: selected.version_id.clone(),
                    dependency_provider: candidate.key.provider,
                    dependency_project_id: candidate.key.project_id.clone(),
                    required_release_id: None,
                    candidate_release_id: Some(candidate.version_id.clone()),
                });
            }
            record_issue(
                state,
                issue_with_requirements(
                    InstanceUpgradeIssueCode::IncompatibleDependency,
                    format!(
                        "{} is incompatible with {}",
                        candidate.key.label(),
                        conflict.label()
                    ),
                    Some(&candidate.key),
                    Some(&candidate.key.project_id),
                    Some(&conflict.project_id),
                    details,
                ),
            );
            continue;
        }
        let mut next_assignments = assignments.clone();
        next_assignments.insert(candidate.key.clone(), candidate.clone());
        let mut next_origins = assignment_origins.clone();
        next_origins.insert(candidate.key.clone(), requirement.origins.clone());
        let mut next_preserved = preserved_unsafe.clone();
        if requirement.preserve_unsafe {
            next_preserved.insert(candidate.key.clone());
        }
        let mut next_requirements = requirements.clone();
        for dependency in &candidate.dependencies {
            if dependency.kind == CandidateDependencyKind::Required {
                let origin = InstanceUpgradeDependencyRequirement {
                    root_content_id: requirement.root_content_id.clone(),
                    root_provider: requirement.root_key.provider,
                    root_project_id: requirement.root_key.project_id.clone(),
                    parent_provider: candidate.key.provider,
                    parent_project_id: candidate.key.project_id.clone(),
                    parent_release_id: candidate.version_id.clone(),
                    dependency_provider: dependency.key.provider,
                    dependency_project_id: dependency.key.project_id.clone(),
                    required_release_id: dependency.version_id.clone(),
                    candidate_release_id: None,
                };
                next_requirements.push(Requirement {
                    key: dependency.key.clone(),
                    version_id: dependency.version_id.clone(),
                    explicit_prerelease: false,
                    preserve_unsafe: requirement.preserve_unsafe,
                    root_content_id: requirement.root_content_id.clone(),
                    root_key: requirement.root_key.clone(),
                    origins: vec![origin],
                });
            }
        }
        search_solutions(
            next_requirements,
            next_assignments,
            next_origins,
            next_preserved,
            roots,
            catalog,
            confirmed_prereleases,
            state,
            solutions,
        );
    }
}

fn candidates_for_requirement<'a>(
    requirement: &Requirement,
    roots: &[RootRequest],
    catalog: &'a UpgradeCatalog,
    confirmed_prereleases: &HashSet<(NodeKey, String)>,
) -> Vec<&'a UpgradeCandidate> {
    let root = roots.iter().find(|root| root.key == requirement.key);
    let candidates = catalog
        .get(&requirement.key)
        .into_iter()
        .flat_map(|pool| &pool.candidates)
        .filter(|candidate| {
            if let Some(version_id) = requirement.version_id.as_deref()
                && candidate.version_id != version_id
            {
                return false;
            }
            match root {
                Some(root)
                    if matches!(
                        root.action,
                        InstanceUpgradeAction::Keep
                            | InstanceUpgradeAction::Disable
                    ) =>
                {
                    candidate.version_id == root.current_release_id
                }
                Some(root) => {
                    candidate.compatible
                        && (!candidate.channel.is_prerelease()
                            || root.allow_prerelease
                            || requirement.explicit_prerelease)
                }
                None => {
                    if requirement.preserve_unsafe {
                        candidate.installed_current
                    } else {
                        candidate.compatible
                            && (!candidate.channel.is_prerelease()
                                || confirmed_prereleases.contains(&(
                                    candidate.key.clone(),
                                    candidate.version_id.clone(),
                                )))
                    }
                }
            }
        })
        .collect::<Vec<_>>();
    candidates
}

fn incompatible_with_assignments(
    candidate: &UpgradeCandidate,
    assignments: &HashMap<NodeKey, UpgradeCandidate>,
) -> Option<NodeKey> {
    for dependency in &candidate.dependencies {
        if dependency.kind == CandidateDependencyKind::Incompatible
            && assignments.get(&dependency.key).is_some_and(|selected| {
                dependency
                    .version_id
                    .as_ref()
                    .is_none_or(|version_id| version_id == &selected.version_id)
            })
        {
            return Some(dependency.key.clone());
        }
    }
    assignments.values().find_map(|selected| {
        selected.dependencies.iter().find_map(|dependency| {
            if dependency.kind == CandidateDependencyKind::Incompatible
                && dependency.key == candidate.key
                && dependency.version_id.as_ref().is_none_or(|version_id| {
                    version_id == &candidate.version_id
                })
            {
                Some(selected.key.clone())
            } else {
                None
            }
        })
    })
}

fn record_issue(state: &mut SearchState, issue: InstanceUpgradeIssue) {
    if state.first_issue.is_none() {
        state.first_issue = Some(issue);
    }
}

fn compare_newest(
    left: &SolverResult,
    right: &SolverResult,
    roots: &[RootRequest],
) -> Ordering {
    let left_score = newest_score(left, roots);
    let right_score = newest_score(right, roots);
    left_score.cmp(&right_score)
}

fn newest_score(
    solution: &SolverResult,
    roots: &[RootRequest],
) -> (u8, i64, i64, std::cmp::Reverse<usize>) {
    let root_keys = roots
        .iter()
        .map(|root| root.key.clone())
        .collect::<HashSet<_>>();
    let stable = solution
        .assignments
        .values()
        .map(|candidate| candidate.channel.rank())
        .min()
        .unwrap_or(0);
    let root_freshness = solution
        .assignments
        .iter()
        .filter(|(key, _)| root_keys.contains(*key))
        .map(|(_, candidate)| candidate.published.timestamp())
        .sum();
    let dependency_freshness = solution
        .assignments
        .iter()
        .filter(|(key, _)| !root_keys.contains(*key))
        .map(|(_, candidate)| candidate.published.timestamp())
        .sum();
    let changed = roots
        .iter()
        .filter(|root| {
            solution
                .assignments
                .get(&root.key)
                .is_some_and(|candidate| {
                    candidate.version_id != root.current_release_id
                })
        })
        .count();
    (
        stable,
        root_freshness,
        dependency_freshness,
        std::cmp::Reverse(changed),
    )
}

fn compare_minimal(
    left: &SolverResult,
    right: &SolverResult,
    roots: &[RootRequest],
    installed: &[InstalledNode],
) -> Ordering {
    minimal_score(left, roots, installed)
        .cmp(&minimal_score(right, roots, installed))
}

fn minimal_score(
    solution: &SolverResult,
    roots: &[RootRequest],
    installed: &[InstalledNode],
) -> (usize, usize, usize, usize, std::cmp::Reverse<i64>) {
    let root_keys = roots
        .iter()
        .map(|root| root.key.clone())
        .collect::<HashSet<_>>();
    let installed_by_key = installed
        .iter()
        .flat_map(|node| {
            node.aliases.iter().map(move |alias| {
                (alias.key.clone(), (node, alias.current_release_id.as_str()))
            })
        })
        .collect::<HashMap<_, _>>();
    let root_replacements = roots
        .iter()
        .filter(|root| {
            solution
                .assignments
                .get(&root.key)
                .is_some_and(|candidate| {
                    candidate.version_id != root.current_release_id
                })
        })
        .count();
    let dependency_replacements = solution
        .assignments
        .iter()
        .filter(|(key, candidate)| {
            !root_keys.contains(*key)
                && installed_by_key.get(*key).is_some_and(|(_, current)| {
                    *current != candidate.version_id
                })
        })
        .count();
    let dependency_additions = solution
        .assignments
        .keys()
        .filter(|key| {
            !root_keys.contains(*key) && !installed_by_key.contains_key(*key)
        })
        .count();
    let auto_removals = installed
        .iter()
        .filter(|node| {
            node.auto_dependency
                && node.migratable
                && !node
                    .aliases
                    .iter()
                    .any(|alias| solution.assignments.contains_key(&alias.key))
                && !installed
                    .iter()
                    .any(|other| other.key == node.key && other.user_owned)
        })
        .count();
    let freshness = solution
        .assignments
        .values()
        .map(|candidate| candidate.published.timestamp())
        .sum();
    (
        root_replacements,
        dependency_replacements,
        dependency_additions,
        auto_removals,
        std::cmp::Reverse(freshness),
    )
}

fn materialize_solution(
    kind: InstanceUpgradeSolutionKind,
    solution: &SolverResult,
    roots: &[RootRequest],
    installed: &[InstalledNode],
) -> InstanceUpgradeSolution {
    let root_keys = roots
        .iter()
        .map(|root| root.key.clone())
        .collect::<HashSet<_>>();
    let enabled = solution_enabled_nodes(solution, roots);
    let installed_by_key = installed
        .iter()
        .flat_map(|node| {
            node.aliases.iter().map(move |alias| {
                (alias.key.clone(), (node, alias.current_release_id.as_str()))
            })
        })
        .collect::<HashMap<_, _>>();
    let mut selections = roots
        .iter()
        .map(|root| {
            let candidate = solution.assignments.get(&root.key);
            let target_release_id =
                candidate.map(|value| value.version_id.clone());
            let action = if root.action == InstanceUpgradeAction::Disable {
                InstanceUpgradeAction::Disable
            } else if target_release_id.as_deref()
                == Some(root.current_release_id.as_str())
            {
                InstanceUpgradeAction::Keep
            } else {
                InstanceUpgradeAction::Upgrade
            };
            InstanceUpgradeSelection {
                content_id: root.content_id.clone(),
                provider: Some(root.key.provider),
                project_id: Some(root.key.project_id.clone()),
                current_release_id: Some(root.current_release_id.clone()),
                target_release_id,
                action,
                enabled: root.enabled
                    && action != InstanceUpgradeAction::Disable,
            }
        })
        .collect::<Vec<_>>();
    selections.sort_by(|left, right| left.content_id.cmp(&right.content_id));

    let mut dependency_changes = solution
        .assignments
        .iter()
        .filter(|(key, _)| !root_keys.contains(*key))
        .map(|(key, candidate)| {
            let current_release_id = installed_by_key
                .get(key)
                .map(|(_, release_id)| (*release_id).to_string());
            let kind = match current_release_id.as_deref() {
                None => InstanceUpgradeDependencyChangeKind::Add,
                Some(current) if current == candidate.version_id => {
                    InstanceUpgradeDependencyChangeKind::Keep
                }
                Some(_) => InstanceUpgradeDependencyChangeKind::Upgrade,
            };
            InstanceUpgradeDependencyChange {
                provider: key.provider,
                project_id: key.project_id.clone(),
                current_release_id,
                target_release_id: Some(candidate.version_id.clone()),
                kind,
                enabled: enabled.contains(key),
            }
        })
        .collect::<Vec<_>>();
    for node in installed
        .iter()
        .filter(|node| node.auto_dependency && node.migratable)
    {
        if node
            .aliases
            .iter()
            .any(|alias| solution.assignments.contains_key(&alias.key))
            || installed
                .iter()
                .any(|other| other.key == node.key && other.user_owned)
        {
            continue;
        }
        dependency_changes.push(InstanceUpgradeDependencyChange {
            provider: node.key.provider,
            project_id: node.key.project_id.clone(),
            current_release_id: Some(node.current_release_id.clone()),
            target_release_id: None,
            kind: InstanceUpgradeDependencyChangeKind::Remove,
            enabled: false,
        });
    }
    dependency_changes.sort_by(|left, right| {
        left.provider
            .as_str()
            .cmp(right.provider.as_str())
            .then_with(|| left.project_id.cmp(&right.project_id))
    });
    let mut warnings = roots
        .iter()
        .filter_map(|root| {
            let candidate = solution.assignments.get(&root.key)?;
            (root.action == InstanceUpgradeAction::Keep
                && !candidate.compatible)
                .then(|| {
                    issue(
                        InstanceUpgradeIssueCode::KeepIncompatible,
                        format!(
                            "{} remains incompatible with target environment",
                            root.key.label()
                        ),
                        Some(&root.key),
                        Some(&root.key.project_id),
                        None,
                    )
                })
        })
        .collect::<Vec<_>>();
    for key in &solution.preserved_unsafe {
        let Some(candidate) = solution.assignments.get(key) else {
            continue;
        };
        if candidate.compatible
            || warnings.iter().any(|warning| {
                warning.provider == Some(key.provider)
                    && warning.project_id.as_deref()
                        == Some(key.project_id.as_str())
            })
        {
            continue;
        }
        warnings.push(issue(
            InstanceUpgradeIssueCode::KeepIncompatible,
            format!(
                "{} is preserved despite target incompatibility",
                key.label()
            ),
            Some(key),
            Some(&key.project_id),
            None,
        ));
    }
    for (key, candidate) in &solution.assignments {
        if !candidate.channel.is_prerelease()
            || solution.preserved_unsafe.contains(key)
        {
            continue;
        }
        warnings.push(issue(
            InstanceUpgradeIssueCode::PrereleaseOnly,
            format!(
                "{} uses explicitly confirmed prerelease {}",
                key.label(),
                candidate.version_id
            ),
            Some(key),
            Some(&key.project_id),
            None,
        ));
    }
    InstanceUpgradeSolution {
        kind,
        selections,
        dependency_changes,
        warnings,
    }
}

fn solution_enabled_nodes(
    solution: &SolverResult,
    roots: &[RootRequest],
) -> HashSet<NodeKey> {
    let mut enabled = HashSet::new();
    let mut queue = roots
        .iter()
        .filter(|root| {
            root.enabled && root.action != InstanceUpgradeAction::Disable
        })
        .map(|root| root.key.clone())
        .collect::<VecDeque<_>>();
    while let Some(key) = queue.pop_front() {
        if !enabled.insert(key.clone()) {
            continue;
        }
        if let Some(candidate) = solution.assignments.get(&key) {
            for dependency in &candidate.dependencies {
                if dependency.kind == CandidateDependencyKind::Required {
                    queue.push_back(dependency.key.clone());
                }
            }
        }
    }
    enabled
}

fn blocking_issue_for_item(
    item: &InstanceUpgradeItem,
    fixed_prerelease: bool,
) -> Option<InstanceUpgradeIssue> {
    if item.resolution.action != InstanceUpgradeAction::Upgrade {
        return None;
    }
    let code = match item.status {
        InstanceUpgradeItemStatus::PrereleaseOnly
            if !item.resolution.allow_prerelease && !fixed_prerelease =>
        {
            InstanceUpgradeIssueCode::PrereleaseOnly
        }
        InstanceUpgradeItemStatus::NoCompatibleRelease => {
            InstanceUpgradeIssueCode::NoCompatibleRelease
        }
        InstanceUpgradeItemStatus::NoCompatibleShaderRuntime => {
            InstanceUpgradeIssueCode::NoCompatibleShaderRuntime
        }
        InstanceUpgradeItemStatus::ShaderRuntimeMissing => {
            InstanceUpgradeIssueCode::ShaderRuntimeMissing
        }
        InstanceUpgradeItemStatus::ShaderRuntimeUnknown => {
            InstanceUpgradeIssueCode::ShaderRuntimeUnknown
        }
        _ => return None,
    };
    Some(InstanceUpgradeIssue {
        code,
        message: format!(
            "{} cannot be upgraded without user resolution",
            item.relative_path
        ),
        content_id: Some(item.content_id.clone()),
        provider: item.provider,
        project_id: item.project_id.clone(),
        conflicting_project_id: None,
        dependency_requirements: Vec::new(),
    })
}

fn item_warnings(items: &[InstanceUpgradeItem]) -> Vec<InstanceUpgradeIssue> {
    item_warnings_with_fixed(items, &HashMap::new())
}

fn item_warnings_with_fixed(
    items: &[InstanceUpgradeItem],
    fixed: &HashMap<NodeKey, String>,
) -> Vec<InstanceUpgradeIssue> {
    items
        .iter()
        .filter_map(|item| {
            let code = match item.status {
                InstanceUpgradeItemStatus::Unidentified => {
                    InstanceUpgradeIssueCode::Unidentified
                }
                InstanceUpgradeItemStatus::UnsupportedContentType => {
                    InstanceUpgradeIssueCode::UnsupportedContentType
                }
                InstanceUpgradeItemStatus::PrereleaseOnly => {
                    InstanceUpgradeIssueCode::PrereleaseOnly
                }
                InstanceUpgradeItemStatus::NoCompatibleRelease
                    if item.resolution.action
                        != InstanceUpgradeAction::Upgrade =>
                {
                    InstanceUpgradeIssueCode::KeepIncompatible
                }
                _ => return None,
            };
            let message = match code {
                InstanceUpgradeIssueCode::PrereleaseOnly
                    if item.resolution.allow_prerelease
                        || item
                            .provider
                            .zip(item.project_id.as_ref())
                            .is_some_and(|(provider, project_id)| {
                                fixed.contains_key(&NodeKey::new(
                                    provider, project_id,
                                ))
                            }) =>
                {
                    format!(
                        "{} uses a confirmed prerelease",
                        item.relative_path
                    )
                }
                InstanceUpgradeIssueCode::PrereleaseOnly => format!(
                    "{} requires explicit prerelease confirmation",
                    item.relative_path
                ),
                _ => format!("{} will be preserved", item.relative_path),
            };
            Some(InstanceUpgradeIssue {
                code,
                message,
                content_id: Some(item.content_id.clone()),
                provider: item.provider,
                project_id: item.project_id.clone(),
                conflicting_project_id: None,
                dependency_requirements: Vec::new(),
            })
        })
        .collect()
}

fn apply_solver_issues_to_items(
    items: &mut [InstanceUpgradeItem],
    issues: &[InstanceUpgradeIssue],
) {
    for issue in issues {
        let status = match issue.code {
            InstanceUpgradeIssueCode::DependencyConflict => {
                InstanceUpgradeItemStatus::DependencyConflict
            }
            InstanceUpgradeIssueCode::MissingRequiredDependency => {
                InstanceUpgradeItemStatus::MissingRequiredDependency
            }
            InstanceUpgradeIssueCode::IncompatibleDependency => {
                InstanceUpgradeItemStatus::IncompatibleDependency
            }
            _ => continue,
        };
        for item in items.iter_mut().filter(|item| {
            issue.project_id.as_deref() == item.project_id.as_deref()
                || issue.conflicting_project_id.as_deref()
                    == item.project_id.as_deref()
                || issue.dependency_requirements.iter().any(|requirement| {
                    requirement.root_content_id == item.content_id
                })
        }) {
            item.status = status;
        }
    }
}

fn issue(
    code: InstanceUpgradeIssueCode,
    message: impl Into<String>,
    key: Option<&NodeKey>,
    project_id: Option<&str>,
    conflicting_project_id: Option<&str>,
) -> InstanceUpgradeIssue {
    InstanceUpgradeIssue {
        code,
        message: message.into(),
        content_id: None,
        provider: key.map(|key| key.provider),
        project_id: project_id.map(str::to_string),
        conflicting_project_id: conflicting_project_id.map(str::to_string),
        dependency_requirements: Vec::new(),
    }
}

fn issue_with_requirements(
    code: InstanceUpgradeIssueCode,
    message: impl Into<String>,
    key: Option<&NodeKey>,
    project_id: Option<&str>,
    conflicting_project_id: Option<&str>,
    dependency_requirements: Vec<InstanceUpgradeDependencyRequirement>,
) -> InstanceUpgradeIssue {
    InstanceUpgradeIssue {
        dependency_requirements,
        ..issue(code, message, key, project_id, conflicting_project_id)
    }
}

fn deduplicate_issues(issues: &mut Vec<InstanceUpgradeIssue>) {
    let mut seen = HashSet::new();
    issues.retain(|issue| {
        seen.insert((
            format!("{:?}", issue.code),
            issue.content_id.clone(),
            issue.project_id.clone(),
            issue.conflicting_project_id.clone(),
        ))
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::path::Path;

    fn key(project_id: &str) -> NodeKey {
        NodeKey::new(ContentProvider::Modrinth, project_id)
    }

    fn candidate(
        project_id: &str,
        version_id: &str,
        published: i64,
    ) -> UpgradeCandidate {
        UpgradeCandidate {
            key: key(project_id),
            version_id: version_id.to_string(),
            published: Utc.timestamp_opt(published, 0).single().unwrap(),
            channel: CandidateChannel::Release,
            compatible: true,
            installed_current: false,
            dependencies: Vec::new(),
        }
    }

    fn required(
        project_id: &str,
        version_id: Option<&str>,
    ) -> CandidateDependency {
        CandidateDependency {
            key: key(project_id),
            version_id: version_id.map(str::to_string),
            kind: CandidateDependencyKind::Required,
        }
    }

    fn incompatible(project_id: &str) -> CandidateDependency {
        CandidateDependency {
            key: key(project_id),
            version_id: None,
            kind: CandidateDependencyKind::Incompatible,
        }
    }

    fn root(project_id: &str, current: &str, enabled: bool) -> RootRequest {
        RootRequest {
            content_id: format!("entry-{project_id}"),
            key: key(project_id),
            current_release_id: current.to_string(),
            enabled,
            action: InstanceUpgradeAction::Upgrade,
            allow_prerelease: false,
        }
    }

    fn installed(
        project_id: &str,
        current: &str,
        auto_dependency: bool,
        user_owned: bool,
    ) -> InstalledNode {
        InstalledNode {
            content_id: format!("entry-{project_id}"),
            key: key(project_id),
            current_release_id: current.to_string(),
            project_type: ProjectType::Mod,
            enabled: true,
            auto_dependency,
            user_owned,
            migratable: true,
            aliases: vec![InstalledAlias {
                key: key(project_id),
                current_release_id: current.to_string(),
            }],
        }
    }

    fn catalog<const N: usize>(
        entries: [(NodeKey, Vec<UpgradeCandidate>); N],
    ) -> UpgradeCatalog {
        entries
            .into_iter()
            .map(|(key, candidates)| {
                (
                    key,
                    CandidatePool {
                        candidates,
                        exploration_limited: false,
                    },
                )
            })
            .collect()
    }

    fn solve(roots: &[RootRequest], catalog: &UpgradeCatalog) -> SolveOutcome {
        solve_upgrade(roots, &[], catalog, &HashMap::new(), &HashSet::new())
    }

    #[test]
    fn single_mod_cross_minecraft_upgrade_selects_target_candidate() {
        let roots = vec![root("a", "a-old", true)];
        let catalog = catalog([(key("a"), vec![candidate("a", "a-new", 2)])]);
        let outcome = solve(&roots, &catalog);
        assert_eq!(outcome.solutions.len(), 1);
        assert_eq!(
            outcome.solutions[0].assignments[&key("a")].version_id,
            "a-new"
        );
    }

    #[test]
    fn minimal_change_keeps_current_compatible_version() {
        let roots = vec![root("a", "a-old", true)];
        let catalog = catalog([(
            key("a"),
            vec![candidate("a", "a-new", 2), candidate("a", "a-old", 1)],
        )]);
        let outcome = solve(&roots, &catalog);
        let minimal = outcome
            .solutions
            .iter()
            .min_by(|left, right| compare_minimal(left, right, &roots, &[]))
            .unwrap();
        assert_eq!(minimal.assignments[&key("a")].version_id, "a-old");
    }

    #[test]
    fn newest_solution_uses_higher_stable_version() {
        let roots = vec![root("a", "a-old", true)];
        let catalog = catalog([(
            key("a"),
            vec![candidate("a", "a-new", 2), candidate("a", "a-old", 1)],
        )]);
        let outcome = solve(&roots, &catalog);
        let newest = outcome
            .solutions
            .iter()
            .max_by(|left, right| compare_newest(left, right, &roots))
            .unwrap();
        assert_eq!(newest.assignments[&key("a")].version_id, "a-new");
    }

    #[test]
    fn prerelease_only_is_not_selected_without_confirmation() {
        let roots = vec![root("a", "a-old", true)];
        let mut beta = candidate("a", "a-beta", 2);
        beta.channel = CandidateChannel::Beta;
        let catalog = catalog([(key("a"), vec![beta])]);
        let outcome = solve(&roots, &catalog);
        assert!(outcome.solutions.is_empty());
    }

    #[test]
    fn required_dependency_is_added() {
        let roots = vec![root("a", "a-old", true)];
        let mut a = candidate("a", "a-new", 2);
        a.dependencies.push(required("x", None));
        let catalog = catalog([
            (key("a"), vec![a]),
            (key("x"), vec![candidate("x", "x-one", 1)]),
        ]);
        let outcome = solve(&roots, &catalog);
        let solution = materialize_solution(
            InstanceUpgradeSolutionKind::Newest,
            &outcome.solutions[0],
            &roots,
            &[],
        );
        assert_eq!(
            solution.dependency_changes[0].kind,
            InstanceUpgradeDependencyChangeKind::Add
        );
    }

    #[test]
    fn transitive_dependency_closure_is_resolved() {
        let roots = vec![root("a", "a-old", true)];
        let mut a = candidate("a", "a-new", 3);
        a.dependencies.push(required("x", None));
        let mut x = candidate("x", "x-one", 2);
        x.dependencies.push(required("y", None));
        let catalog = catalog([
            (key("a"), vec![a]),
            (key("x"), vec![x]),
            (key("y"), vec![candidate("y", "y-one", 1)]),
        ]);
        assert_eq!(solve(&roots, &catalog).solutions[0].assignments.len(), 3);
    }

    #[test]
    fn multiple_roots_share_one_dependency_assignment() {
        let roots = vec![root("a", "a-old", true), root("b", "b-old", true)];
        let mut a = candidate("a", "a-new", 3);
        a.dependencies.push(required("x", Some("x-one")));
        let mut b = candidate("b", "b-new", 3);
        b.dependencies.push(required("x", Some("x-one")));
        let catalog = catalog([
            (key("a"), vec![a]),
            (key("b"), vec![b]),
            (key("x"), vec![candidate("x", "x-one", 1)]),
        ]);
        assert_eq!(solve(&roots, &catalog).solutions[0].assignments.len(), 3);
    }

    #[test]
    fn latest_conflict_backtracks_to_older_root_candidate() {
        let roots = vec![root("a", "a-old", true), root("b", "b-old", true)];
        let mut newest = candidate("a", "a-two", 3);
        newest.dependencies.push(incompatible("b"));
        let catalog = catalog([
            (key("a"), vec![newest, candidate("a", "a-one", 2)]),
            (key("b"), vec![candidate("b", "b-one", 1)]),
        ]);
        let outcome = solve(&roots, &catalog);
        assert_eq!(outcome.solutions.len(), 1);
        assert_eq!(
            outcome.solutions[0].assignments[&key("a")].version_id,
            "a-one"
        );
    }

    #[test]
    fn complete_dependency_conflict_has_no_solution() {
        let roots = vec![root("a", "a-old", true), root("b", "b-old", true)];
        let mut a = candidate("a", "a-one", 1);
        a.dependencies.push(incompatible("b"));
        let catalog = catalog([
            (key("a"), vec![a]),
            (key("b"), vec![candidate("b", "b-one", 1)]),
        ]);
        assert!(solve(&roots, &catalog).solutions.is_empty());
    }

    #[test]
    fn missing_required_dependency_is_blocking() {
        let roots = vec![root("a", "a-old", true)];
        let mut a = candidate("a", "a-one", 1);
        a.dependencies.push(required("missing", None));
        let catalog = catalog([(key("a"), vec![a])]);
        let outcome = solve(&roots, &catalog);
        assert_eq!(
            outcome.issues[0].code,
            InstanceUpgradeIssueCode::MissingRequiredDependency
        );
    }

    #[test]
    fn incompatible_dependency_edge_is_blocking() {
        let roots = vec![root("a", "a-old", true), root("x", "x-old", true)];
        let mut a = candidate("a", "a-one", 1);
        a.dependencies.push(incompatible("x"));
        let catalog = catalog([
            (key("a"), vec![a]),
            (key("x"), vec![candidate("x", "x-one", 1)]),
        ]);
        let outcome = solve(&roots, &catalog);
        assert!(outcome.solutions.is_empty());
        assert_eq!(
            outcome.issues[0].code,
            InstanceUpgradeIssueCode::IncompatibleDependency
        );
    }

    #[test]
    fn orphaned_auto_dependency_is_suggested_for_removal() {
        let roots = vec![root("a", "a-old", true)];
        let catalog = catalog([(key("a"), vec![candidate("a", "a-one", 1)])]);
        let outcome = solve(&roots, &catalog);
        let nodes = vec![installed("x", "x-old", true, false)];
        let solution = materialize_solution(
            InstanceUpgradeSolutionKind::Newest,
            &outcome.solutions[0],
            &roots,
            &nodes,
        );
        assert_eq!(
            solution.dependency_changes[0].kind,
            InstanceUpgradeDependencyChangeKind::Remove
        );
    }

    #[test]
    fn user_owned_dependency_identity_is_never_removed() {
        let roots = vec![root("a", "a-old", true)];
        let catalog = catalog([(key("a"), vec![candidate("a", "a-one", 1)])]);
        let outcome = solve(&roots, &catalog);
        let nodes = vec![
            installed("x", "x-auto", true, false),
            installed("x", "x-user", false, true),
        ];
        let solution = materialize_solution(
            InstanceUpgradeSolutionKind::Newest,
            &outcome.solutions[0],
            &roots,
            &nodes,
        );
        assert!(solution.dependency_changes.is_empty());
    }

    #[test]
    fn disabled_root_remains_disabled_after_upgrade() {
        let roots = vec![root("a", "a-old", false)];
        let catalog = catalog([(key("a"), vec![candidate("a", "a-one", 1)])]);
        let outcome = solve(&roots, &catalog);
        let solution = materialize_solution(
            InstanceUpgradeSolutionKind::Newest,
            &outcome.solutions[0],
            &roots,
            &[],
        );
        assert!(!solution.selections[0].enabled);
    }

    #[test]
    fn disabled_only_dependency_remains_disabled() {
        let roots = vec![root("a", "a-old", false)];
        let mut a = candidate("a", "a-one", 2);
        a.dependencies.push(required("x", None));
        let catalog = catalog([
            (key("a"), vec![a]),
            (key("x"), vec![candidate("x", "x-one", 1)]),
        ]);
        let outcome = solve(&roots, &catalog);
        let solution = materialize_solution(
            InstanceUpgradeSolutionKind::Newest,
            &outcome.solutions[0],
            &roots,
            &[],
        );
        assert!(!solution.dependency_changes[0].enabled);
    }

    #[test]
    fn unidentified_local_jar_is_preserved_with_warning() {
        let item = test_item(InstanceUpgradeItemStatus::Unidentified);
        let warnings = item_warnings(&[item]);
        assert_eq!(warnings[0].code, InstanceUpgradeIssueCode::Unidentified);
    }

    #[test]
    fn optifine_to_iris_shader_accepts_iris_release() {
        let version = test_version(vec!["iris"], vec!["26.1"]);
        assert!(modrinth_version_matches(
            &version,
            ProjectType::ShaderPack,
            &test_environment(ShaderRuntime::Iris)
        ));
    }

    #[test]
    fn optifine_to_iris_shader_rejects_optifine_only_release() {
        let version = test_version(vec!["optifine"], vec!["26.1"]);
        assert!(!modrinth_version_matches(
            &version,
            ProjectType::ShaderPack,
            &test_environment(ShaderRuntime::Iris)
        ));
    }

    #[test]
    fn source_shader_runtime_uses_trusted_modrinth_alias_for_curseforge_item() {
        let mut item = snapshot_item_for_test(
            Some(ContentProvider::CurseForge),
            Some("123"),
            Some("123"),
        );
        item.content.as_mut().unwrap().provider_refs = vec![
            ContentProviderRef::from_database("curseforge", "123", Some("123"))
                .unwrap(),
            ContentProviderRef::from_database(
                "modrinth",
                "YL57xq9U",
                Some("iris-version"),
            )
            .unwrap(),
        ];
        let snapshot = InstanceContentSnapshot {
            instance_id: "instance".to_string(),
            revision: 1,
            pack: None,
            items: vec![item],
            pending_manual_downloads: Vec::new(),
            warnings: Vec::new(),
        };
        assert_eq!(source_shader_runtime(&[], &snapshot), ShaderRuntime::Iris);
        let (_, installed) = snapshot_upgrade_items(&snapshot);
        assert!(installed[0].aliases.iter().any(|alias| {
            alias.key.provider == ContentProvider::Modrinth
                && alias.key.project_id == "YL57xq9U"
        }));
    }

    #[test]
    fn unverified_local_mod_makes_shader_runtime_unknown() {
        let snapshot = InstanceContentSnapshot {
            instance_id: "instance".to_string(),
            revision: 1,
            pack: None,
            items: vec![snapshot_item_for_test(None, None, None)],
            pending_manual_downloads: Vec::new(),
            warnings: Vec::new(),
        };
        assert_eq!(
            source_shader_runtime(&[], &snapshot),
            ShaderRuntime::Unknown
        );
    }

    #[test]
    fn valid_custom_fixed_version_is_selected() {
        let roots = vec![root("a", "old", true)];
        let catalog = catalog([(
            key("a"),
            vec![candidate("a", "two", 2), candidate("a", "one", 1)],
        )]);
        let fixed = HashMap::from([(key("a"), "one".to_string())]);
        let outcome =
            solve_upgrade(&roots, &[], &catalog, &fixed, &HashSet::new());
        assert_eq!(
            outcome.solutions[0].assignments[&key("a")].version_id,
            "one"
        );
    }

    #[test]
    fn custom_fixed_version_conflict_returns_detail() {
        let roots = vec![root("a", "old", true), root("b", "old", true)];
        let mut a = candidate("a", "two", 2);
        a.dependencies.push(incompatible("b"));
        let catalog = catalog([
            (key("a"), vec![a, candidate("a", "one", 1)]),
            (key("b"), vec![candidate("b", "one", 1)]),
        ]);
        let fixed = HashMap::from([(key("a"), "two".to_string())]);
        let outcome =
            solve_upgrade(&roots, &[], &catalog, &fixed, &HashSet::new());
        assert!(outcome.solutions.is_empty());
        assert_eq!(
            outcome.issues[0].conflicting_project_id.as_deref(),
            Some("b")
        );
        assert!(
            outcome.issues[0]
                .dependency_requirements
                .iter()
                .any(|detail| detail.root_project_id == "a")
        );
        assert!(
            outcome.issues[0]
                .dependency_requirements
                .iter()
                .any(|detail| detail.root_project_id == "b")
        );
    }

    #[test]
    fn compatible_current_outside_exploration_limit_remains_minimal() {
        let roots = vec![root("a", "current", true)];
        let mut candidates = (1..=6)
            .map(|index| candidate("a", &format!("new-{index}"), 20 - index))
            .collect::<Vec<_>>();
        let mut current = candidate("a", "current", 1);
        current.installed_current = true;
        candidates.push(current);
        let catalog = catalog([(key("a"), candidates)]);
        let outcome = solve(&roots, &catalog);
        let minimal = outcome
            .solutions
            .iter()
            .min_by(|left, right| compare_minimal(left, right, &roots, &[]))
            .unwrap();
        assert_eq!(minimal.assignments[&key("a")].version_id, "current");
    }

    #[test]
    fn custom_fixed_candidate_outside_exploration_limit_is_not_truncated() {
        let roots = vec![root("a", "old", true)];
        let mut candidates = (1..=6)
            .map(|index| candidate("a", &format!("new-{index}"), 20 - index))
            .collect::<Vec<_>>();
        candidates.push(candidate("a", "fixed-seven", 1));
        let catalog = catalog([(key("a"), candidates)]);
        let fixed = HashMap::from([(key("a"), "fixed-seven".to_string())]);
        let outcome =
            solve_upgrade(&roots, &[], &catalog, &fixed, &HashSet::new());
        assert_eq!(
            outcome.solutions[0].assignments[&key("a")].version_id,
            "fixed-seven"
        );
    }

    #[test]
    fn custom_fixed_version_from_another_project_is_rejected() {
        let version = test_version(vec!["fabric"], vec!["26.1"]);
        let error = validate_modrinth_custom_fixed(
            &key("different-project"),
            &version,
            ProjectType::Mod,
            &test_environment(ShaderRuntime::Iris),
        )
        .unwrap_err();
        assert!(error.to_string().contains("belongs to project"));
    }

    #[test]
    fn custom_fixed_version_incompatible_with_target_is_rejected() {
        let version = test_version(vec!["fabric"], vec!["1.20.1"]);
        let error = validate_modrinth_custom_fixed(
            &key("project"),
            &version,
            ProjectType::Mod,
            &test_environment(ShaderRuntime::Iris),
        )
        .unwrap_err();
        assert!(error.to_string().contains("not compatible"));
    }

    #[test]
    fn candidate_limit_returns_limit_issue_instead_of_false_conflict() {
        let roots = vec![root("a", "old", true)];
        let mut pool = CandidatePool {
            candidates: vec![candidate("a", "one", 1)],
            exploration_limited: true,
        };
        pool.candidates[0]
            .dependencies
            .push(required("missing", None));
        let catalog = HashMap::from([(key("a"), pool)]);
        let outcome = solve(&roots, &catalog);
        assert_eq!(
            outcome.issues[0].code,
            InstanceUpgradeIssueCode::SearchLimitReached
        );
    }

    #[test]
    fn provider_candidate_filter_finds_compatible_item_after_first_fifty() {
        let provider_files = (0..=50).collect::<Vec<_>>();
        let (selected, limited) =
            bounded_compatible_candidates(&provider_files, |file| *file == 50);
        assert_eq!(selected, vec![50]);
        assert!(!limited);
    }

    #[test]
    fn exact_dependency_conflict_contains_both_root_provenances() {
        let roots = vec![root("a", "old-a", true), root("b", "old-b", true)];
        let mut a = candidate("a", "a-one", 2);
        a.dependencies.push(required("x", Some("x-two")));
        let mut b = candidate("b", "b-one", 2);
        b.dependencies.push(required("x", Some("x-three")));
        let catalog = catalog([
            (key("a"), vec![a]),
            (key("b"), vec![b]),
            (
                key("x"),
                vec![candidate("x", "x-two", 1), candidate("x", "x-three", 1)],
            ),
        ]);
        let outcome = solve(&roots, &catalog);
        let details = &outcome.issues[0].dependency_requirements;
        assert_eq!(details.len(), 2);
        assert!(details.iter().any(|detail| {
            detail.root_project_id == "a"
                && detail.required_release_id.as_deref() == Some("x-two")
        }));
        assert!(details.iter().any(|detail| {
            detail.root_project_id == "b"
                && detail.required_release_id.as_deref() == Some("x-three")
        }));
    }

    #[test]
    fn transitive_missing_dependency_reports_root_and_direct_parent() {
        let roots = vec![root("a", "old", true)];
        let mut a = candidate("a", "a-one", 3);
        a.dependencies.push(required("x", None));
        let mut x = candidate("x", "x-one", 2);
        x.dependencies
            .push(required("missing", Some("missing-one")));
        let catalog = catalog([(key("a"), vec![a]), (key("x"), vec![x])]);
        let outcome = solve(&roots, &catalog);
        let detail = &outcome.issues[0].dependency_requirements[0];
        assert_eq!(detail.root_project_id, "a");
        assert_eq!(detail.parent_project_id, "x");
        assert_eq!(detail.parent_release_id, "x-one");
        assert_eq!(detail.dependency_project_id, "missing");
    }

    #[test]
    fn version_scoped_incompatible_edge_only_rejects_exact_version() {
        let roots = vec![root("a", "old-a", true), root("x", "old-x", true)];
        let mut a = candidate("a", "a-one", 2);
        a.dependencies.push(CandidateDependency {
            key: key("x"),
            version_id: Some("x-two".to_string()),
            kind: CandidateDependencyKind::Incompatible,
        });
        let compatible_catalog = catalog([
            (key("a"), vec![a.clone()]),
            (key("x"), vec![candidate("x", "x-three", 1)]),
        ]);
        assert!(!solve(&roots, &compatible_catalog).solutions.is_empty());
        let incompatible_catalog = catalog([
            (key("a"), vec![a]),
            (key("x"), vec![candidate("x", "x-two", 1)]),
        ]);
        assert!(solve(&roots, &incompatible_catalog).solutions.is_empty());
    }

    #[test]
    fn keep_incompatible_root_preserves_unsafe_dependency_closure() {
        let mut keep = root("a", "a-old", true);
        keep.action = InstanceUpgradeAction::Keep;
        let mut a = candidate("a", "a-old", 2);
        a.compatible = false;
        a.installed_current = true;
        a.dependencies.push(required("x", Some("x-old")));
        let mut x = candidate("x", "x-old", 1);
        x.compatible = false;
        x.installed_current = true;
        let catalog = catalog([(key("a"), vec![a]), (key("x"), vec![x])]);
        let nodes = vec![installed("x", "x-old", true, false)];
        let outcome = solve(&[keep.clone()], &catalog);
        assert_eq!(outcome.solutions.len(), 1);
        let solution = materialize_solution(
            InstanceUpgradeSolutionKind::Newest,
            &outcome.solutions[0],
            &[keep],
            &nodes,
        );
        assert!(solution.warnings.len() >= 2);
        assert!(solution.dependency_changes.iter().all(|change| {
            change.kind != InstanceUpgradeDependencyChangeKind::Remove
        }));
    }

    #[test]
    fn disable_incompatible_root_keeps_old_dependency_disabled() {
        let mut disabled = root("a", "a-old", true);
        disabled.action = InstanceUpgradeAction::Disable;
        let mut a = candidate("a", "a-old", 2);
        a.compatible = false;
        a.installed_current = true;
        a.dependencies.push(required("x", Some("x-old")));
        let mut x = candidate("x", "x-old", 1);
        x.compatible = false;
        x.installed_current = true;
        let catalog = catalog([(key("a"), vec![a]), (key("x"), vec![x])]);
        let outcome = solve(&[disabled.clone()], &catalog);
        let solution = materialize_solution(
            InstanceUpgradeSolutionKind::Newest,
            &outcome.solutions[0],
            &[disabled],
            &[],
        );
        assert!(!solution.selections[0].enabled);
        assert!(!solution.dependency_changes[0].enabled);
    }

    #[test]
    fn enabled_root_dependency_wins_over_disabled_preserved_dependency() {
        let mut disabled = root("d", "d-old", true);
        disabled.action = InstanceUpgradeAction::Disable;
        let enabled = root("a", "a-old", true);
        let mut d = candidate("d", "d-old", 3);
        d.installed_current = true;
        d.compatible = false;
        d.dependencies.push(required("x", Some("x-old")));
        let mut a = candidate("a", "a-new", 3);
        a.dependencies.push(required("x", Some("x-new")));
        let mut old_x = candidate("x", "x-old", 1);
        old_x.installed_current = true;
        old_x.compatible = false;
        let catalog = catalog([
            (key("d"), vec![d]),
            (key("a"), vec![a]),
            (key("x"), vec![candidate("x", "x-new", 2), old_x]),
        ]);
        let roots = vec![disabled, enabled];
        let outcome = solve(&roots, &catalog);
        let selected = &outcome.solutions[0];
        assert_eq!(selected.assignments[&key("x")].version_id, "x-new");
        let solution = materialize_solution(
            InstanceUpgradeSolutionKind::Newest,
            selected,
            &roots,
            &[],
        );
        assert!(
            solution
                .dependency_changes
                .iter()
                .find(|change| { change.project_id == "x" })
                .unwrap()
                .enabled
        );
    }

    #[test]
    fn prerelease_dependency_requires_and_records_exact_confirmation() {
        let roots = vec![root("a", "old", true)];
        let mut a = candidate("a", "a-one", 2);
        a.dependencies.push(required("x", Some("x-beta")));
        let mut beta = candidate("x", "x-beta", 1);
        beta.channel = CandidateChannel::Beta;
        let catalog = catalog([(key("a"), vec![a]), (key("x"), vec![beta])]);
        let blocked = solve(&roots, &catalog);
        assert_eq!(
            blocked.issues[0].code,
            InstanceUpgradeIssueCode::PrereleaseOnly
        );
        assert_eq!(
            blocked.issues[0].dependency_requirements[0]
                .candidate_release_id
                .as_deref(),
            Some("x-beta")
        );
        let confirmations = HashSet::from([(key("x"), "x-beta".to_string())]);
        let allowed = solve_upgrade(
            &roots,
            &[],
            &catalog,
            &HashMap::new(),
            &confirmations,
        );
        assert_eq!(allowed.solutions.len(), 1);
    }

    #[test]
    fn custom_fixed_beta_root_is_an_explicit_confirmation() {
        let roots = vec![root("a", "old", true)];
        let mut beta = candidate("a", "a-beta", 1);
        beta.channel = CandidateChannel::Beta;
        let catalog = catalog([(key("a"), vec![beta])]);
        let fixed = HashMap::from([(key("a"), "a-beta".to_string())]);
        assert_eq!(
            solve_upgrade(&roots, &[], &catalog, &fixed, &HashSet::new(),)
                .solutions
                .len(),
            1
        );
    }

    #[test]
    fn trusted_cross_provider_alias_prevents_duplicate_dependency_add() {
        let cf_root = NodeKey::new(ContentProvider::CurseForge, "root-cf");
        let cf_dep = NodeKey::new(ContentProvider::CurseForge, "dep-cf");
        let roots = vec![RootRequest {
            content_id: "root".to_string(),
            key: cf_root.clone(),
            current_release_id: "root-old".to_string(),
            enabled: true,
            action: InstanceUpgradeAction::Upgrade,
            allow_prerelease: false,
        }];
        let root_candidate = UpgradeCandidate {
            key: cf_root.clone(),
            version_id: "root-new".to_string(),
            published: Utc.timestamp_opt(2, 0).single().unwrap(),
            channel: CandidateChannel::Release,
            compatible: true,
            installed_current: false,
            dependencies: vec![CandidateDependency {
                key: cf_dep.clone(),
                version_id: Some("dep-file".to_string()),
                kind: CandidateDependencyKind::Required,
            }],
        };
        let dep_candidate = UpgradeCandidate {
            key: cf_dep.clone(),
            version_id: "dep-file".to_string(),
            published: Utc.timestamp_opt(1, 0).single().unwrap(),
            channel: CandidateChannel::Release,
            compatible: true,
            installed_current: true,
            dependencies: Vec::new(),
        };
        let catalog = catalog([
            (cf_root, vec![root_candidate.clone()]),
            (cf_dep.clone(), vec![dep_candidate]),
        ]);
        let installed_dep = InstalledNode {
            content_id: "dep-entry".to_string(),
            key: key("dep-mr"),
            current_release_id: "dep-mr-version".to_string(),
            project_type: ProjectType::Mod,
            enabled: true,
            auto_dependency: true,
            user_owned: false,
            migratable: true,
            aliases: vec![
                InstalledAlias {
                    key: key("dep-mr"),
                    current_release_id: "dep-mr-version".to_string(),
                },
                InstalledAlias {
                    key: cf_dep,
                    current_release_id: "dep-file".to_string(),
                },
            ],
        };
        let outcome = solve(&roots, &catalog);
        let solution = materialize_solution(
            InstanceUpgradeSolutionKind::Newest,
            &outcome.solutions[0],
            &roots,
            &[installed_dep],
        );
        assert_eq!(
            solution.dependency_changes[0].kind,
            InstanceUpgradeDependencyChangeKind::Keep
        );
    }

    #[test]
    fn planner_snapshot_analysis_does_not_mutate_instance_state() {
        let snapshot = InstanceContentSnapshot {
            instance_id: "instance".to_string(),
            revision: 7,
            pack: None,
            items: vec![InstanceContentSnapshotItem {
                file_id: Some("file".to_string()),
                entry_id: Some("entry".to_string()),
                member_id: None,
                ownership_kind: ContentOwnershipKind::UserAdded,
                materialization_state:
                    crate::state::PackMemberMaterializationState::Present,
                override_kind: crate::state::PackMemberOverrideKind::None,
                expected_relative_path: "mods/a.jar".to_string(),
                required: true,
                project_type: ProjectType::Mod,
                provider: Some(ContentProvider::Modrinth),
                provider_project_id: Some("a".to_string()),
                provider_release_id: Some("old".to_string()),
                content: None,
                capabilities: crate::state::ContentItemCapabilities::default(),
                dependency: Some(crate::state::ContentDependencyInfo {
                    auto_dependency: false,
                    ..Default::default()
                }),
            }],
            pending_manual_downloads: Vec::new(),
            warnings: Vec::new(),
        };
        let before = serde_json::to_value(&snapshot).unwrap();
        let _ = snapshot_upgrade_items(&snapshot);
        assert_eq!(serde_json::to_value(&snapshot).unwrap(), before);
    }

    #[derive(Debug, Eq, PartialEq)]
    struct PlannerStateDigest {
        metadata: String,
        revision: u64,
        instance_files: String,
        content_entries: String,
        provider_refs: String,
        dependency_edges: String,
        disk_files: Vec<(String, Vec<u8>)>,
    }

    #[tokio::test]
    async fn real_planner_sees_untracked_disk_file_without_mutating_instance() {
        let temp = tempfile::tempdir().unwrap();
        let directories = crate::state::DirectoryInfo {
            settings_dir: temp.path().to_path_buf(),
            config_dir: temp.path().to_path_buf(),
            app_identifier: "upgrade-planner-test".to_string(),
        };
        std::fs::create_dir_all(directories.instances_dir()).unwrap();
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(temp.path().join("state.db"))
                    .create_if_missing(true)
                    .foreign_keys(true),
            )
            .await
            .unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        let state = crate::state::test_state(directories, pool.clone())
            .await
            .unwrap();
        let instance = crate::state::instances::create_instance(
            crate::state::instances::CreateInstance {
                name: "Planner Dry Run".to_string(),
                path: Some("planner-dry-run".to_string()),
                game_version: "1.21.4".to_string(),
                loader: crate::state::ModLoader::Vanilla,
                loader_version: None,
                icon_path: None,
                link: InstanceLink::Unmanaged,
                symlink_target: None,
            },
            &state,
        )
        .await
        .unwrap();
        let instance_dir =
            state.directories.instances_dir().join(&instance.path);
        let mods = instance_dir.join("mods");
        std::fs::create_dir_all(&mods).unwrap();
        std::fs::write(mods.join("new.jar"), b"not-a-real-jar").unwrap();

        let before =
            planner_state_digest(&pool, &instance_dir, &instance.id).await;
        let plan = create_instance_upgrade_plan(
            &instance.id,
            InstanceUpgradeEnvironment {
                game_version: "1.21.5".to_string(),
                mod_loader: crate::state::ModLoader::Vanilla,
                mod_loader_version: None,
                shader_runtime: ShaderRuntime::None,
            },
            &state,
        )
        .await
        .unwrap();
        let after =
            planner_state_digest(&pool, &instance_dir, &instance.id).await;

        assert!(plan.items.iter().any(|item| {
            item.relative_path == "mods/new.jar"
                && item.status == InstanceUpgradeItemStatus::Unidentified
        }));
        assert_eq!(after, before);
    }

    async fn planner_state_digest(
        pool: &sqlx::SqlitePool,
        instance_dir: &Path,
        instance_id: &str,
    ) -> PlannerStateDigest {
        let metadata = super::super::get_instance_metadata(instance_id, pool)
            .await
            .unwrap()
            .unwrap();
        let content_set_id = metadata.applied_content_set.id.clone();
        PlannerStateDigest {
            metadata: serde_json::to_string(&metadata).unwrap(),
            revision: metadata.applied_content_set.revision,
            instance_files: snapshot_table(
                pool,
                "instance_files",
                "instance_id = ?",
                instance_id,
            )
            .await,
            content_entries: snapshot_table(
                pool,
                "instance_content_entries",
                "content_set_id = ?",
                &content_set_id,
            )
            .await,
            provider_refs: snapshot_table(
                pool,
                "instance_content_provider_refs",
                "content_entry_id IN (SELECT id FROM instance_content_entries WHERE content_set_id = ?)",
                &content_set_id,
            )
            .await,
            dependency_edges: snapshot_table(
                pool,
                "instance_content_dependencies",
                "content_set_id = ?",
                &content_set_id,
            )
            .await,
            disk_files: disk_file_snapshot(instance_dir),
        }
    }

    async fn snapshot_table(
        pool: &sqlx::SqlitePool,
        table: &str,
        predicate: &str,
        value: &str,
    ) -> String {
        let columns = sqlx::query_scalar::<_, String>(&format!(
            "SELECT name FROM pragma_table_info('{table}') ORDER BY cid"
        ))
        .fetch_all(pool)
        .await
        .unwrap();
        let json_columns = columns
            .iter()
            .map(|column| format!("\"{}\"", column.replace('"', "\"\"")))
            .collect::<Vec<_>>()
            .join(", ");
        let query = format!(
            "SELECT COALESCE(json_group_array(json_array({json_columns})), '[]') FROM (SELECT * FROM \"{table}\" WHERE {predicate} ORDER BY rowid)"
        );
        sqlx::query_scalar::<_, String>(&query)
            .bind(value)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    fn disk_file_snapshot(root: &Path) -> Vec<(String, Vec<u8>)> {
        fn visit(
            root: &Path,
            current: &Path,
            files: &mut Vec<(String, Vec<u8>)>,
        ) {
            for entry in std::fs::read_dir(current).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    visit(root, &path, files);
                } else {
                    files.push((
                        path.strip_prefix(root)
                            .unwrap()
                            .to_string_lossy()
                            .replace('\\', "/"),
                        std::fs::read(path).unwrap(),
                    ));
                }
            }
        }
        let mut files = Vec::new();
        visit(root, root, &mut files);
        files.sort_by(|left, right| left.0.cmp(&right.0));
        files
    }

    fn test_item(status: InstanceUpgradeItemStatus) -> InstanceUpgradeItem {
        InstanceUpgradeItem {
            content_id: "local".to_string(),
            relative_path: "mods/local.jar".to_string(),
            project_type: ProjectType::Mod,
            provider: None,
            project_id: None,
            current_release_id: None,
            current_enabled: true,
            auto_dependency: false,
            status,
            resolution: InstanceUpgradeResolution {
                content_id: "local".to_string(),
                action: InstanceUpgradeAction::Keep,
                allow_prerelease: false,
                confirmed_prerelease_dependencies: Vec::new(),
            },
            candidate_release_ids: Vec::new(),
        }
    }

    fn snapshot_item_for_test(
        provider: Option<ContentProvider>,
        project_id: Option<&str>,
        release_id: Option<&str>,
    ) -> InstanceContentSnapshotItem {
        InstanceContentSnapshotItem {
            file_id: Some("file".to_string()),
            entry_id: Some("entry".to_string()),
            member_id: None,
            ownership_kind: ContentOwnershipKind::UserAdded,
            materialization_state: PackMemberMaterializationState::Present,
            override_kind: PackMemberOverrideKind::None,
            expected_relative_path: "mods/runtime.jar".to_string(),
            required: false,
            project_type: ProjectType::Mod,
            provider,
            provider_project_id: project_id.map(str::to_string),
            provider_release_id: release_id.map(str::to_string),
            content: Some(ContentItem {
                file_name: "runtime.jar".to_string(),
                file_path: "mods/runtime.jar".to_string(),
                id: "hash".to_string(),
                size: 1,
                enabled: true,
                project_type: ProjectType::Mod,
                project: None,
                version: None,
                owner: None,
                update: None,
                date_added: None,
                provider_refs: Vec::new(),
                origin_provider: provider,
                rollback: None,
                environment: None,
                source_kind: Some(ContentSourceKind::Local),
                external: provider.is_none(),
                loader: None,
            }),
            capabilities: ContentItemCapabilities::default(),
            dependency: Some(crate::state::ContentDependencyInfo {
                auto_dependency: false,
                ..Default::default()
            }),
        }
    }

    fn test_environment(
        shader_runtime: ShaderRuntime,
    ) -> InstanceUpgradeEnvironment {
        InstanceUpgradeEnvironment {
            game_version: "26.1".to_string(),
            mod_loader: crate::state::ModLoader::Fabric,
            mod_loader_version: None,
            shader_runtime,
        }
    }

    fn test_version(loaders: Vec<&str>, game_versions: Vec<&str>) -> Version {
        Version {
            id: "version".to_string(),
            project_id: "project".to_string(),
            author_id: "author".to_string(),
            featured: false,
            name: "Version".to_string(),
            version_number: "1".to_string(),
            changelog: None,
            changelog_url: None,
            date_published: Utc.timestamp_opt(1, 0).single().unwrap(),
            downloads: 0,
            version_type: "release".to_string(),
            files: Vec::new(),
            dependencies: Vec::new(),
            game_versions: game_versions
                .into_iter()
                .map(str::to_string)
                .collect(),
            loaders: loaders.into_iter().map(str::to_string).collect(),
        }
    }
}
