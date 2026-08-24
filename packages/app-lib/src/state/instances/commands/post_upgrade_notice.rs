use sqlx::{Row, SqliteConnection, SqlitePool};

use crate::state::{InstancePostUpgradeNotice, InstancePostUpgradeWarning};

pub(crate) async fn get_instance_post_upgrade_notice(
    instance_id: &str,
    pool: &SqlitePool,
) -> crate::Result<Option<InstancePostUpgradeNotice>> {
    let row = sqlx::query(
        "SELECT upgrade_job_id, target_game_version, consecutive_clean_launches, warnings_json FROM instance_post_upgrade_notices WHERE instance_id = ?",
    )
    .bind(instance_id)
    .fetch_optional(pool)
    .await?;
    row.map(|row| {
        let warnings_json: String = row.try_get("warnings_json")?;
        Ok(InstancePostUpgradeNotice {
            instance_id: instance_id.to_string(),
            upgrade_job_id: row.try_get("upgrade_job_id")?,
            target_game_version: row.try_get("target_game_version")?,
            consecutive_clean_launches: row
                .try_get::<i64, _>("consecutive_clean_launches")?
                .clamp(0, u8::MAX as i64)
                as u8,
            warnings: serde_json::from_str(&warnings_json)?,
        })
    })
    .transpose()
}

pub(crate) async fn replace_instance_post_upgrade_notice(
    notice: &InstancePostUpgradeNotice,
    pool: &SqlitePool,
) -> crate::Result<()> {
    let mut connection = pool.acquire().await?;
    replace_instance_post_upgrade_notice_on_connection(notice, &mut connection)
        .await
}

pub(crate) async fn replace_instance_post_upgrade_notice_on_connection(
    notice: &InstancePostUpgradeNotice,
    connection: &mut SqliteConnection,
) -> crate::Result<()> {
    if notice.warnings.is_empty() {
        sqlx::query(
            "DELETE FROM instance_post_upgrade_notices WHERE instance_id = ?",
        )
        .bind(&notice.instance_id)
        .execute(&mut *connection)
        .await?;
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO instance_post_upgrade_notices (instance_id, upgrade_job_id, target_game_version, consecutive_clean_launches, warnings_json) VALUES (?, ?, ?, ?, ?) ON CONFLICT(instance_id) DO UPDATE SET upgrade_job_id = excluded.upgrade_job_id, target_game_version = excluded.target_game_version, consecutive_clean_launches = excluded.consecutive_clean_launches, warnings_json = excluded.warnings_json, modified = CURRENT_TIMESTAMP",
    )
    .bind(&notice.instance_id)
    .bind(&notice.upgrade_job_id)
    .bind(&notice.target_game_version)
    .bind(i64::from(notice.consecutive_clean_launches))
    .bind(serde_json::to_string(&notice.warnings)?)
    .execute(&mut *connection)
    .await?;
    Ok(())
}

pub(crate) async fn dismiss_instance_post_upgrade_notice(
    instance_id: &str,
    pool: &SqlitePool,
) -> crate::Result<()> {
    sqlx::query(
        "DELETE FROM instance_post_upgrade_notices WHERE instance_id = ?",
    )
    .bind(instance_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub(crate) fn next_post_upgrade_notice_after_launch(
    mut notice: InstancePostUpgradeNotice,
    clean: bool,
) -> Option<InstancePostUpgradeNotice> {
    if clean {
        notice.consecutive_clean_launches =
            notice.consecutive_clean_launches.saturating_add(1);
        (notice.consecutive_clean_launches < 2).then_some(notice)
    } else {
        notice.consecutive_clean_launches = 0;
        Some(notice)
    }
}

pub(crate) async fn record_instance_post_upgrade_launch(
    instance_id: &str,
    clean: bool,
    pool: &SqlitePool,
) -> crate::Result<()> {
    let Some(notice) =
        get_instance_post_upgrade_notice(instance_id, pool).await?
    else {
        return Ok(());
    };
    match next_post_upgrade_notice_after_launch(notice, clean) {
        Some(notice) => {
            replace_instance_post_upgrade_notice(&notice, pool).await
        }
        None => dismiss_instance_post_upgrade_notice(instance_id, pool).await,
    }
}

pub(crate) fn post_upgrade_warnings_from_details(
    details: &[crate::install::InstanceUpgradeCompatibilityWarning],
) -> Vec<InstancePostUpgradeWarning> {
    details
        .iter()
        .filter(|warning| {
            warning.content_id.is_some() || warning.relative_path.is_some()
        })
        .map(|warning| InstancePostUpgradeWarning {
            code: warning.code,
            content_id: warning.content_id.clone(),
            relative_path: warning.relative_path.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::InstanceUpgradeIssueCode;

    fn notice(clean_launches: u8) -> InstancePostUpgradeNotice {
        InstancePostUpgradeNotice {
            instance_id: "instance".to_string(),
            upgrade_job_id: "job".to_string(),
            target_game_version: "26.2".to_string(),
            consecutive_clean_launches: clean_launches,
            warnings: vec![InstancePostUpgradeWarning {
                code: InstanceUpgradeIssueCode::KeepIncompatible,
                content_id: Some("content".to_string()),
                relative_path: Some("mods/example.jar".to_string()),
            }],
        }
    }

    #[test]
    fn clean_launch_expires_notice_after_two_consecutive_sessions() {
        let first = next_post_upgrade_notice_after_launch(notice(0), true)
            .expect("first launch keeps notice");
        assert_eq!(first.consecutive_clean_launches, 1);
        assert!(next_post_upgrade_notice_after_launch(first, true).is_none());
    }

    #[test]
    fn failed_launch_resets_consecutive_count() {
        let reset = next_post_upgrade_notice_after_launch(notice(1), false)
            .expect("failed launch keeps notice");
        assert_eq!(reset.consecutive_clean_launches, 0);
    }

    #[tokio::test]
    async fn dismiss_removes_persisted_notice() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query("CREATE TABLE instances (id TEXT PRIMARY KEY NOT NULL)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO instances (id) VALUES ('instance')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE instance_post_upgrade_notices (instance_id TEXT PRIMARY KEY NOT NULL REFERENCES instances(id) ON DELETE CASCADE, upgrade_job_id TEXT NOT NULL, target_game_version TEXT NOT NULL, consecutive_clean_launches INTEGER NOT NULL DEFAULT 0, warnings_json TEXT NOT NULL, created TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, modified TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP)",
        )
        .execute(&pool)
        .await
        .unwrap();

        replace_instance_post_upgrade_notice(&notice(0), &pool)
            .await
            .unwrap();
        assert!(
            get_instance_post_upgrade_notice("instance", &pool)
                .await
                .unwrap()
                .is_some()
        );
        dismiss_instance_post_upgrade_notice("instance", &pool)
            .await
            .unwrap();
        assert!(
            get_instance_post_upgrade_notice("instance", &pool)
                .await
                .unwrap()
                .is_none()
        );
    }
}
