use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

use dashmap::DashMap;
use tokio::sync::Mutex;

use crate::state::instances::adapters::sqlite::content_rows;
use crate::state::{
    InstanceUpgradeFixedConstraint, InstanceUpgradePlan,
    InstanceUpgradeResolution, InstanceUpgradeSolutionChoice,
    InstanceUpgradeSolutionKind, State,
};

type StoredUpgradePlan = Arc<Mutex<InstanceUpgradePlan>>;

static INSTANCE_UPGRADE_PLANS: LazyLock<DashMap<String, StoredUpgradePlan>> =
    LazyLock::new(DashMap::new);

#[tracing::instrument]
pub async fn plan_instance_upgrade(
    instance_id: &str,
    target_environment: crate::state::InstanceUpgradeEnvironment,
) -> crate::Result<InstanceUpgradePlan> {
    let state = State::get().await?;
    let plan = crate::state::instances::commands::create_instance_upgrade_plan(
        instance_id,
        target_environment,
        &state,
    )
    .await?;
    INSTANCE_UPGRADE_PLANS
        .insert(plan.id.clone(), Arc::new(Mutex::new(plan.clone())));
    Ok(plan)
}

#[tracing::instrument]
pub async fn get_instance_upgrade_plan(
    plan_id: &str,
) -> crate::Result<InstanceUpgradePlan> {
    let state = State::get().await?;
    let handle = stored_plan_handle(plan_id)?;
    let plan = handle.lock().await;
    if let Err(error) = ensure_current_revision(&plan, &state).await {
        drop(plan);
        INSTANCE_UPGRADE_PLANS.remove(plan_id);
        return Err(error);
    }
    Ok(plan.clone())
}

#[tracing::instrument]
pub async fn update_instance_upgrade_resolution(
    plan_id: &str,
    resolution: InstanceUpgradeResolution,
) -> crate::Result<InstanceUpgradePlan> {
    let state = State::get().await?;
    let handle = stored_plan_handle(plan_id)?;
    let mut stored = handle.lock().await;
    let source = ensure_current_revision(&stored, &state).await?;
    let mut plan = stored.clone();
    let item = plan
        .items
        .iter_mut()
        .find(|item| item.content_id == resolution.content_id)
        .ok_or_else(|| {
            crate::ErrorKind::InputError(format!(
                "Upgrade plan has no content item {}",
                resolution.content_id
            ))
        })?;
    item.resolution = resolution;
    let (kind, constraints) = selected_kind_and_constraints(&plan);
    crate::state::instances::commands::recompute_instance_upgrade_plan_from_source(
        &mut plan,
        &constraints,
        kind,
        source,
        &state,
    )
    .await?;
    *stored = plan.clone();
    Ok(plan)
}

#[tracing::instrument]
pub async fn select_instance_upgrade_solution(
    plan_id: &str,
    choice: InstanceUpgradeSolutionChoice,
) -> crate::Result<InstanceUpgradePlan> {
    let state = State::get().await?;
    let handle = stored_plan_handle(plan_id)?;
    let mut stored = handle.lock().await;
    ensure_current_revision(&stored, &state).await?;
    let mut plan = stored.clone();
    plan.selected_solution = match choice {
        InstanceUpgradeSolutionChoice::Newest => plan.newest_solution.clone(),
        InstanceUpgradeSolutionChoice::MinimalChange => {
            plan.minimal_change_solution.clone()
        }
        InstanceUpgradeSolutionChoice::Custom => Some(
            plan.selected_solution
                .clone()
                .filter(|solution| {
                    solution.kind == InstanceUpgradeSolutionKind::Custom
                })
                .ok_or_else(|| {
                    crate::ErrorKind::InputError(
                        "No custom upgrade solution has been resolved"
                            .to_string(),
                    )
                })?,
        ),
    };
    plan.dependency_changes = plan
        .selected_solution
        .as_ref()
        .map(|solution| solution.dependency_changes.clone())
        .unwrap_or_default();
    *stored = plan.clone();
    Ok(plan)
}

#[tracing::instrument]
pub async fn resolve_custom_instance_upgrade_solution(
    plan_id: &str,
    fixed_constraints: Vec<InstanceUpgradeFixedConstraint>,
) -> crate::Result<InstanceUpgradePlan> {
    let state = State::get().await?;
    let handle = stored_plan_handle(plan_id)?;
    let mut stored = handle.lock().await;
    let source = ensure_current_revision(&stored, &state).await?;
    let mut plan = stored.clone();
    validate_fixed_constraints(&plan, &fixed_constraints)?;
    crate::state::instances::commands::recompute_instance_upgrade_plan_from_source(
        &mut plan,
        &fixed_constraints,
        InstanceUpgradeSolutionKind::Custom,
        source,
        &state,
    )
    .await?;
    plan.custom_constraints = fixed_constraints;
    *stored = plan.clone();
    Ok(plan)
}

fn stored_plan_handle(plan_id: &str) -> crate::Result<StoredUpgradePlan> {
    INSTANCE_UPGRADE_PLANS
        .get(plan_id)
        .map(|entry| Arc::clone(entry.value()))
        .ok_or_else(|| {
            crate::ErrorKind::InputError(
                "The instance upgrade plan has expired".to_string(),
            )
            .into()
        })
}

async fn ensure_current_revision(
    plan: &InstanceUpgradePlan,
    state: &State,
) -> crate::Result<crate::state::instances::commands::ReadOnlyUpgradeSource> {
    let current_revision =
        content_rows::get_applied_content_set(&plan.instance_id, &state.pool)
            .await?
            .ok_or_else(|| {
                crate::ErrorKind::InputError(
                    "Instance has no applied content set".to_string(),
                )
            })?
            .revision;
    if let Err(error) =
        ensure_instance_upgrade_revision(plan.source_revision, current_revision)
    {
        return Err(error);
    }
    crate::state::instances::commands::validate_instance_upgrade_plan_source(
        plan, state,
    )
    .await
}

fn ensure_instance_upgrade_revision(
    planned_revision: u64,
    current_revision: u64,
) -> crate::Result<()> {
    if planned_revision == current_revision {
        return Ok(());
    }
    Err(crate::ErrorKind::StaleInstanceUpgradePlan {
        planned_revision,
        current_revision,
    }
    .into())
}

fn selected_kind_and_constraints(
    plan: &InstanceUpgradePlan,
) -> (
    InstanceUpgradeSolutionKind,
    Vec<InstanceUpgradeFixedConstraint>,
) {
    let Some(solution) = plan.selected_solution.as_ref() else {
        return (InstanceUpgradeSolutionKind::Newest, Vec::new());
    };
    if solution.kind != InstanceUpgradeSolutionKind::Custom {
        return (solution.kind, Vec::new());
    }
    (
        InstanceUpgradeSolutionKind::Custom,
        plan.custom_constraints.clone(),
    )
}

fn validate_fixed_constraints(
    plan: &InstanceUpgradePlan,
    constraints: &[InstanceUpgradeFixedConstraint],
) -> crate::Result<()> {
    let mut seen = HashMap::new();
    for constraint in constraints {
        if let Some(previous) = seen.insert(
            (constraint.provider, constraint.project_id.as_str()),
            constraint.version_id.as_str(),
        ) && previous != constraint.version_id.as_str()
        {
            return Err(crate::ErrorKind::InputError(format!(
                "Custom upgrade constraints select multiple versions for {}:{}",
                constraint.provider.as_str(),
                constraint.project_id
            ))
            .into());
        }
        let root_exists = plan.items.iter().any(|item| {
            !item.auto_dependency
                && item.provider == Some(constraint.provider)
                && item.project_id.as_deref()
                    == Some(constraint.project_id.as_str())
        });
        if !root_exists {
            return Err(crate::ErrorKind::InputError(format!(
				"Custom upgrade constraint is not a root content project: {}:{}",
				constraint.provider.as_str(),
				constraint.project_id
			))
			.into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_custom_constraints_are_rejected_before_provider_work() {
        let mut plan = empty_plan();
        plan.items.push(crate::state::InstanceUpgradeItem {
            content_id: "root".to_string(),
            relative_path: "mods/root.jar".to_string(),
            project_type: crate::state::ProjectType::Mod,
            provider: Some(crate::state::ContentProvider::Modrinth),
            project_id: Some("root".to_string()),
            current_release_id: Some("old".to_string()),
            current_enabled: true,
            auto_dependency: false,
            status: crate::state::InstanceUpgradeItemStatus::UpgradeAvailable,
            resolution: crate::state::InstanceUpgradeResolution {
                content_id: "root".to_string(),
                action: crate::state::InstanceUpgradeAction::Upgrade,
                allow_prerelease: false,
                confirmed_prerelease_dependencies: Vec::new(),
            },
            candidate_release_ids: vec!["one".to_string(), "two".to_string()],
        });
        let constraints = vec![
            InstanceUpgradeFixedConstraint {
                provider: crate::state::ContentProvider::Modrinth,
                project_id: "root".to_string(),
                version_id: "one".to_string(),
            },
            InstanceUpgradeFixedConstraint {
                provider: crate::state::ContentProvider::Modrinth,
                project_id: "root".to_string(),
                version_id: "two".to_string(),
            },
        ];
        assert!(validate_fixed_constraints(&plan, &constraints).is_err());
    }

    #[test]
    fn stale_upgrade_plan_revision_is_rejected() {
        assert!(ensure_instance_upgrade_revision(4, 4).is_ok());
        let error = ensure_instance_upgrade_revision(4, 5).unwrap_err();
        assert!(error.to_string().contains("planned revision 4"));
        assert!(error.to_string().contains("current revision 5"));
    }

    #[test]
    fn custom_recompute_uses_only_explicitly_stored_constraints() {
        let mut plan = empty_plan();
        plan.custom_constraints = vec![InstanceUpgradeFixedConstraint {
            provider: crate::state::ContentProvider::Modrinth,
            project_id: "a".to_string(),
            version_id: "a-fixed".to_string(),
        }];
        plan.selected_solution = Some(crate::state::InstanceUpgradeSolution {
            kind: InstanceUpgradeSolutionKind::Custom,
            selections: vec![
                crate::state::InstanceUpgradeSelection {
                    content_id: "a".to_string(),
                    provider: Some(crate::state::ContentProvider::Modrinth),
                    project_id: Some("a".to_string()),
                    current_release_id: Some("a-old".to_string()),
                    target_release_id: Some("a-fixed".to_string()),
                    action: crate::state::InstanceUpgradeAction::Upgrade,
                    enabled: true,
                },
                crate::state::InstanceUpgradeSelection {
                    content_id: "b".to_string(),
                    provider: Some(crate::state::ContentProvider::Modrinth),
                    project_id: Some("b".to_string()),
                    current_release_id: Some("b-old".to_string()),
                    target_release_id: Some("b-auto".to_string()),
                    action: crate::state::InstanceUpgradeAction::Upgrade,
                    enabled: true,
                },
            ],
            dependency_changes: Vec::new(),
            warnings: Vec::new(),
        });
        let (kind, constraints) = selected_kind_and_constraints(&plan);
        assert_eq!(kind, InstanceUpgradeSolutionKind::Custom);
        assert_eq!(constraints, plan.custom_constraints);
        assert_eq!(constraints.len(), 1);
        assert_eq!(constraints[0].project_id, "a");
    }

    #[tokio::test]
    async fn per_plan_mutex_serializes_mutations_without_lost_update() {
        let plan = Arc::new(Mutex::new(empty_plan()));
        let acquired = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let first_plan = Arc::clone(&plan);
        let first_acquired = Arc::clone(&acquired);
        let first_release = Arc::clone(&release);
        let first = tokio::spawn(async move {
            let mut stored = first_plan.lock().await;
            first_acquired.notify_one();
            first_release.notified().await;
            stored
                .custom_constraints
                .push(InstanceUpgradeFixedConstraint {
                    provider: crate::state::ContentProvider::Modrinth,
                    project_id: "a".to_string(),
                    version_id: "a-one".to_string(),
                });
        });
        acquired.notified().await;
        let second_plan = Arc::clone(&plan);
        let second = tokio::spawn(async move {
            let mut stored = second_plan.lock().await;
            stored
                .custom_constraints
                .push(InstanceUpgradeFixedConstraint {
                    provider: crate::state::ContentProvider::Modrinth,
                    project_id: "b".to_string(),
                    version_id: "b-one".to_string(),
                });
        });
        tokio::task::yield_now().await;
        release.notify_one();
        first.await.unwrap();
        second.await.unwrap();
        let stored = plan.lock().await;
        assert_eq!(stored.custom_constraints.len(), 2);
    }

    fn empty_plan() -> InstanceUpgradePlan {
        let environment = crate::state::InstanceUpgradeEnvironment {
            game_version: "1.21.1".to_string(),
            mod_loader: crate::state::ModLoader::Fabric,
            mod_loader_version: None,
            shader_runtime: crate::state::ShaderRuntime::Iris,
        };
        InstanceUpgradePlan {
            id: "plan".to_string(),
            instance_id: "instance".to_string(),
            source_revision: 1,
            source_files: Vec::new(),
            source_environment: environment.clone(),
            target_environment: environment,
            items: Vec::new(),
            dependency_changes: Vec::new(),
            warnings: Vec::new(),
            blocking_issues: Vec::new(),
            newest_solution: None,
            minimal_change_solution: None,
            selected_solution: None,
            custom_constraints: Vec::new(),
        }
    }
}
