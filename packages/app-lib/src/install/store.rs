use super::model::{
    InstallJobKind, InstallJobSnapshot, InstallJobState, InstallJobStatus,
};
use crate::state::{InstanceInstallStage, State};
use chrono::{DateTime, TimeZone, Utc};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct InstallJobRecord {
    pub id: Uuid,
    pub instance_id: Option<String>,
    pub kind: InstallJobKind,
    pub status: InstallJobStatus,
    pub state: InstallJobState,
    pub created: DateTime<Utc>,
    pub modified: DateTime<Utc>,
    pub finished: Option<DateTime<Utc>>,
    pub dismissed: bool,
}

#[derive(Debug)]
struct InstallJobRow {
    pub id: String,
    pub instance_id: Option<String>,
    pub kind: String,
    pub status: String,
    pub state: String,
    pub created: i64,
    pub modified: i64,
    pub finished: Option<i64>,
    pub dismissed: i64,
}

impl InstallJobRecord {
    pub fn snapshot(&self) -> InstallJobSnapshot {
        let summary = self.state.download_summary();
        let items = self.state.download_items();
        let recorded_instance_id = instance_id(&self.state);
        let instance_deleted = self.state.instance_deleted()
            || (self.status == InstallJobStatus::Succeeded
                && self.instance_id.is_none()
                && recorded_instance_id.is_some());
        InstallJobSnapshot {
            job_id: self.id,
            instance_id: self.instance_id.clone().or(recorded_instance_id),
            instance_deleted,
            kind: self.kind,
            status: self.status,
            execution_mode: self.state.execution_mode(self.status),
            provider: self.state.provider(),
            target: self.state.target.clone(),
            phase: self.state.progress.phase,
            progress: self.state.progress.progress.clone(),
            details: self.state.progress.details.clone(),
            display: self.state.display.clone(),
            error: self.state.error.clone(),
            rollback_error: self.state.rollback_error.clone(),
            pause_reason: self.state.pause_reason.clone(),
            created: self.created,
            modified: self.modified,
            finished: self.finished,
            summary,
            items,
        }
    }
}

pub async fn insert(
    id: Uuid,
    state: &InstallJobState,
    status: InstallJobStatus,
    app_state: &State,
) -> crate::Result<InstallJobRecord> {
    let now = Utc::now();
    let kind = state.request.kind();
    let json = serde_json::to_string(state)?;
    let status_value = status.as_str();
    let kind_value = kind.as_str();
    let instance_id = instance_id(state);
    let id_value = id.to_string();
    let created = now.timestamp();
    let modified = created;

    sqlx::query!(
        "
		INSERT INTO install_jobs (
			id, instance_id, kind, status, state, created, modified, finished, dismissed
		)
		VALUES (?, ?, ?, ?, ?, ?, ?, NULL, 0)
		",
        id_value,
        instance_id,
        kind_value,
        status_value,
        json,
        created,
        modified,
    )
    .execute(&app_state.pool)
    .await?;

    sync_download_details(id, state, app_state).await?;

    get(id, app_state).await?.ok_or_else(|| {
        crate::ErrorKind::OtherError(format!(
            "Install job {id} was not inserted"
        ))
        .into()
    })
}

pub async fn get(
    id: Uuid,
    app_state: &State,
) -> crate::Result<Option<InstallJobRecord>> {
    let id = id.to_string();
    let row = sqlx::query_as!(
        InstallJobRow,
        "
		SELECT
			id AS \"id!: String\",
			instance_id,
			kind AS \"kind!: String\",
			status AS \"status!: String\",
			state AS \"state!: String\",
			created AS \"created!: i64\",
			modified AS \"modified!: i64\",
			finished,
			dismissed AS \"dismissed!: i64\"
		FROM install_jobs
		WHERE id = ?
		",
        id,
    )
    .fetch_optional(&app_state.pool)
    .await?;

    row.map(row_to_record).transpose()
}

pub async fn list(
    include_finished: bool,
    app_state: &State,
) -> crate::Result<Vec<InstallJobRecord>> {
    let rows = if include_finished {
        sqlx::query_as!(
            InstallJobRow,
            "
			SELECT
				id AS \"id!: String\",
				instance_id,
				kind AS \"kind!: String\",
				status AS \"status!: String\",
				state AS \"state!: String\",
				created AS \"created!: i64\",
				modified AS \"modified!: i64\",
				finished,
				dismissed AS \"dismissed!: i64\"
			FROM install_jobs
			WHERE dismissed = 0
			ORDER BY created ASC
			",
        )
        .fetch_all(&app_state.pool)
        .await?
    } else {
        sqlx::query_as!(
			InstallJobRow,
			"
			SELECT
				id AS \"id!: String\",
				instance_id,
				kind AS \"kind!: String\",
				status AS \"status!: String\",
				state AS \"state!: String\",
				created AS \"created!: i64\",
				modified AS \"modified!: i64\",
				finished,
				dismissed AS \"dismissed!: i64\"
			FROM install_jobs
			WHERE dismissed = 0 AND status IN ('queued', 'running', 'failed', 'interrupted')
			ORDER BY created ASC
			",
		)
		.fetch_all(&app_state.pool)
		.await?
    };

    let mut rows = rows;
    if !include_finished {
        use sqlx::Row;
        let active_rows = sqlx::query(
            "SELECT id, instance_id, kind, status, state, created, modified,
                    finished, dismissed
             FROM install_jobs
             WHERE dismissed = 0
               AND status IN ('canceling', 'waiting_for_user')
             ORDER BY created ASC",
        )
        .fetch_all(&app_state.pool)
        .await?;
        for row in active_rows {
            rows.push(InstallJobRow {
                id: row.try_get("id")?,
                instance_id: row.try_get("instance_id")?,
                kind: row.try_get("kind")?,
                status: row.try_get("status")?,
                state: row.try_get("state")?,
                created: row.try_get("created")?,
                modified: row.try_get("modified")?,
                finished: row.try_get("finished")?,
                dismissed: row.try_get("dismissed")?,
            });
        }
        rows.sort_unstable_by_key(|row| row.created);
    }

    rows.into_iter().map(row_to_record).collect()
}

pub async fn list_interrupted_candidates(
    app_state: &State,
) -> crate::Result<Vec<InstallJobRecord>> {
    let mut rows = sqlx::query_as!(
        InstallJobRow,
        "
		SELECT
			id AS \"id!: String\",
			instance_id,
			kind AS \"kind!: String\",
			status AS \"status!: String\",
			state AS \"state!: String\",
			created AS \"created!: i64\",
			modified AS \"modified!: i64\",
			finished,
			dismissed AS \"dismissed!: i64\"
		FROM install_jobs
		WHERE status IN ('queued', 'running')
		ORDER BY created ASC
		",
    )
    .fetch_all(&app_state.pool)
    .await?;

    use sqlx::Row;
    let canceling_rows = sqlx::query(
        "SELECT id, instance_id, kind, status, state, created, modified,
                finished, dismissed
         FROM install_jobs
         WHERE status = 'canceling'
         ORDER BY created ASC",
    )
    .fetch_all(&app_state.pool)
    .await?;
    for row in canceling_rows {
        rows.push(InstallJobRow {
            id: row.try_get("id")?,
            instance_id: row.try_get("instance_id")?,
            kind: row.try_get("kind")?,
            status: row.try_get("status")?,
            state: row.try_get("state")?,
            created: row.try_get("created")?,
            modified: row.try_get("modified")?,
            finished: row.try_get("finished")?,
            dismissed: row.try_get("dismissed")?,
        });
    }

    rows.into_iter().map(row_to_record).collect()
}

pub async fn update_state(
    id: Uuid,
    state: &InstallJobState,
    app_state: &State,
) -> crate::Result<InstallJobRecord> {
    let now = Utc::now();
    let json = serde_json::to_string(state)?;
    let instance_id = instance_id(state);
    let id_value = id.to_string();
    let modified = now.timestamp();

    sqlx::query(
        "
		UPDATE install_jobs
		SET instance_id = (SELECT id FROM instances WHERE id = ?),
			state = ?, modified = ?
		WHERE id = ?
		",
    )
    .bind(instance_id)
    .bind(json)
    .bind(modified)
    .bind(id_value)
    .execute(&app_state.pool)
    .await?;

    sync_download_details(id, state, app_state).await?;

    get_required(id, app_state).await
}

pub async fn update_progress_state(
    id: Uuid,
    state: &InstallJobState,
    app_state: &State,
) -> crate::Result<()> {
    let json = serde_json::to_string(state)?;
    let summary = state.download_summary();
    let modified = Utc::now().timestamp();
    let id_value = id.to_string();

    sqlx::query(
        "UPDATE install_jobs
         SET state = ?, modified = ?, provider = ?, files_total = ?,
             files_completed = ?, bytes_total = ?, bytes_downloaded = ?
         WHERE id = ?",
    )
    .bind(json)
    .bind(modified)
    .bind(state.provider().as_str())
    .bind(summary.files_total.map(|value| value as i64))
    .bind(summary.files_completed as i64)
    .bind(summary.bytes_total.map(|value| value as i64))
    .bind(summary.bytes_downloaded as i64)
    .bind(id_value)
    .execute(&app_state.pool)
    .await?;

    Ok(())
}

pub async fn update_status(
    id: Uuid,
    status: InstallJobStatus,
    state: &InstallJobState,
    app_state: &State,
) -> crate::Result<InstallJobRecord> {
    let now = Utc::now();
    let finished = status.is_finished().then_some(now.timestamp());
    let json = serde_json::to_string(state)?;
    let status_value = status.as_str();
    let instance_id = instance_id(state);
    let id_value = id.to_string();
    let modified = now.timestamp();

    sqlx::query(
        "
		UPDATE install_jobs
		SET instance_id = (SELECT id FROM instances WHERE id = ?),
			status = ?, state = ?, modified = ?, finished = ?
		WHERE id = ?
		",
    )
    .bind(instance_id)
    .bind(status_value)
    .bind(json)
    .bind(modified)
    .bind(finished)
    .bind(id_value)
    .execute(&app_state.pool)
    .await?;

    sync_download_details(id, state, app_state).await?;

    get_required(id, app_state).await
}

pub async fn update_status_if(
    id: Uuid,
    expected: InstallJobStatus,
    status: InstallJobStatus,
    state: &InstallJobState,
    app_state: &State,
) -> crate::Result<Option<InstallJobRecord>> {
    let now = Utc::now();
    let finished = status.is_finished().then_some(now.timestamp());
    let json = serde_json::to_string(state)?;
    let instance_id = instance_id(state);
    let id_value = id.to_string();
    let modified = now.timestamp();

    let updated = compare_and_swap_status(
        &app_state.pool,
        &id_value,
        expected,
        status,
        instance_id,
        &json,
        modified,
        finished,
    )
    .await?;

    if !updated {
        return Ok(None);
    }

    sync_download_details(id, state, app_state).await?;
    Ok(Some(get_required(id, app_state).await?))
}

pub async fn complete_running_job(
    id: Uuid,
    instance_id: Option<&str>,
    state: &InstallJobState,
    app_state: &State,
) -> crate::Result<Option<InstallJobRecord>> {
    let now = Utc::now();
    let modified = now.timestamp();
    let json = serde_json::to_string(state)?;
    let id_value = id.to_string();
    let mut transaction = app_state.pool.begin().await?;

    if let Some(instance_id) = instance_id {
        let result = sqlx::query(
            "UPDATE instances
             SET install_stage = ?, modified = ?
             WHERE id = ?",
        )
        .bind(InstanceInstallStage::Installed.as_str())
        .bind(modified)
        .bind(instance_id)
        .execute(&mut *transaction)
        .await?;
        if result.rows_affected() != 1 {
            return Err(crate::ErrorKind::InputError(format!(
                "Install target instance {instance_id} no longer exists"
            ))
            .into());
        }
    }

    let result = sqlx::query(
        "UPDATE install_jobs
         SET instance_id = (SELECT id FROM instances WHERE id = ?),
             status = ?, state = ?, modified = ?, finished = ?
         WHERE id = ? AND status = ?",
    )
    .bind(instance_id)
    .bind(InstallJobStatus::Succeeded.as_str())
    .bind(json)
    .bind(modified)
    .bind(now.timestamp())
    .bind(&id_value)
    .bind(InstallJobStatus::Running.as_str())
    .execute(&mut *transaction)
    .await?;

    if result.rows_affected() != 1 {
        transaction.rollback().await?;
        return Ok(None);
    }
    transaction.commit().await?;

    if let Err(error) = sync_download_details(id, state, app_state).await {
        tracing::warn!(
            job_id = %id,
            error = %error,
            "Install job succeeded, but final download details could not be synchronized"
        );
    }
    Ok(Some(get_required(id, app_state).await?))
}

#[allow(clippy::too_many_arguments)]
async fn compare_and_swap_status(
    pool: &sqlx::SqlitePool,
    id: &str,
    expected: InstallJobStatus,
    status: InstallJobStatus,
    instance_id: Option<String>,
    state_json: &str,
    modified: i64,
    finished: Option<i64>,
) -> crate::Result<bool> {
    let result = sqlx::query(
        "UPDATE install_jobs
         SET instance_id = (SELECT id FROM instances WHERE id = ?),
             status = ?, state = ?, modified = ?, finished = ?
         WHERE id = ? AND status = ?",
    )
    .bind(instance_id)
    .bind(status.as_str())
    .bind(state_json)
    .bind(modified)
    .bind(finished)
    .bind(id)
    .bind(expected.as_str())
    .execute(pool)
    .await?;
    Ok(result.rows_affected() == 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn only_one_waiting_job_resume_can_claim_the_status() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::query("CREATE TABLE instances (id TEXT PRIMARY KEY)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE install_jobs (
                id TEXT PRIMARY KEY,
                instance_id TEXT,
                status TEXT NOT NULL,
                state TEXT NOT NULL,
                modified INTEGER NOT NULL,
                finished INTEGER
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO install_jobs
             (id, status, state, modified) VALUES ('job', 'waiting_for_user', '{}', 0)",
        )
        .execute(&pool)
        .await
        .unwrap();

        let first = compare_and_swap_status(
            &pool,
            "job",
            InstallJobStatus::WaitingForUser,
            InstallJobStatus::Queued,
            None,
            "{}",
            1,
            None,
        );
        let second = compare_and_swap_status(
            &pool,
            "job",
            InstallJobStatus::WaitingForUser,
            InstallJobStatus::Queued,
            None,
            "{}",
            1,
            None,
        );
        let (first, second) = tokio::join!(first, second);

        assert_ne!(first.unwrap(), second.unwrap());
        let status: String = sqlx::query_scalar(
            "SELECT status FROM install_jobs WHERE id = 'job'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(status, "queued");
    }
}

pub async fn dismiss(id: Uuid, app_state: &State) -> crate::Result<()> {
    let id = id.to_string();
    let modified = Utc::now().timestamp();
    sqlx::query!(
        "
		UPDATE install_jobs
		SET dismissed = 1, modified = ?
		WHERE id = ?
		",
        modified,
        id,
    )
    .execute(&app_state.pool)
    .await?;

    Ok(())
}

pub async fn clear_finished(app_state: &State) -> crate::Result<u64> {
    let modified = Utc::now().timestamp();
    let result = sqlx::query(
        "UPDATE install_jobs
         SET dismissed = 1, modified = ?
         WHERE dismissed = 0
           AND status IN ('succeeded', 'failed', 'interrupted', 'canceled')",
    )
    .bind(modified)
    .execute(&app_state.pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn mark_instance_deleted(
    instance_id: &str,
    app_state: &State,
) -> crate::Result<Vec<InstallJobRecord>> {
    use sqlx::Row;

    let rows = sqlx::query(
        "
		SELECT
			id,
			instance_id,
			kind,
			status,
			state,
			created,
			modified,
			finished,
			dismissed
		FROM install_jobs
		WHERE instance_id = ? AND dismissed = 0
		",
    )
    .bind(instance_id)
    .fetch_all(&app_state.pool)
    .await?;

    let mut updated = Vec::new();
    for row in rows {
        let mut record = row_to_record(InstallJobRow {
            id: row.try_get("id")?,
            instance_id: row.try_get("instance_id")?,
            kind: row.try_get("kind")?,
            status: row.try_get("status")?,
            state: row.try_get("state")?,
            created: row.try_get("created")?,
            modified: row.try_get("modified")?,
            finished: row.try_get("finished")?,
            dismissed: row.try_get("dismissed")?,
        })?;
        if record.state.instance_deleted() {
            updated.push(record);
            continue;
        }
        record.state.record_event(
            super::model::InstallJobEventKind::TargetInstanceDeleted {
                instance_id: instance_id.to_string(),
            },
        );
        updated.push(update_state(record.id, &record.state, app_state).await?);
    }
    Ok(updated)
}

pub async fn get_required(
    id: Uuid,
    app_state: &State,
) -> crate::Result<InstallJobRecord> {
    get(id, app_state).await?.ok_or_else(|| {
        crate::ErrorKind::InputError(format!("Unknown install job {id}")).into()
    })
}

fn row_to_record(row: InstallJobRow) -> crate::Result<InstallJobRecord> {
    Ok(InstallJobRecord {
        id: Uuid::parse_str(&row.id).map_err(|err| {
            crate::ErrorKind::InputError(format!(
                "Invalid install job id {}: {err}",
                row.id
            ))
        })?,
        instance_id: row.instance_id,
        kind: InstallJobKind::from_stored_str(&row.kind),
        status: InstallJobStatus::from_stored_str(&row.status),
        state: serde_json::from_str(&row.state)?,
        created: timestamp(row.created),
        modified: timestamp(row.modified),
        finished: row.finished.and_then(optional_timestamp),
        dismissed: row.dismissed != 0,
    })
}

fn instance_id(state: &InstallJobState) -> Option<String> {
    match &state.target {
        super::model::InstallTarget::NewInstance { instance_id } => {
            instance_id.clone()
        }
        super::model::InstallTarget::ExistingInstance { instance_id } => {
            Some(instance_id.clone())
        }
    }
}

fn timestamp(value: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(value, 0)
        .single()
        .unwrap_or_else(Utc::now)
}

fn optional_timestamp(value: i64) -> Option<DateTime<Utc>> {
    Utc.timestamp_opt(value, 0).single()
}

async fn sync_download_details(
    id: Uuid,
    state: &InstallJobState,
    app_state: &State,
) -> crate::Result<()> {
    let id_value = id.to_string();
    let summary = state.download_summary();
    let mut transaction = app_state.pool.begin().await?;
    sqlx::query(
        "UPDATE install_jobs
         SET provider = ?, files_total = ?, files_completed = ?,
             bytes_total = ?, bytes_downloaded = ?
         WHERE id = ?",
    )
    .bind(state.provider().as_str())
    .bind(summary.files_total.map(|value| value as i64))
    .bind(summary.files_completed as i64)
    .bind(summary.bytes_total.map(|value| value as i64))
    .bind(summary.bytes_downloaded as i64)
    .bind(&id_value)
    .execute(&mut *transaction)
    .await?;

    let now = Utc::now().timestamp();
    for item in state.download_items() {
        let finished = matches!(
            item.status,
            super::model::DownloadItemStatus::Completed
                | super::model::DownloadItemStatus::Skipped
                | super::model::DownloadItemStatus::Failed
                | super::model::DownloadItemStatus::Canceled
        )
        .then_some(now);
        let status = format!("{:?}", item.status).to_ascii_lowercase();
        sqlx::query(
            "INSERT INTO install_job_items (
                id, job_id, name, project_id, version_id, status,
                bytes_total, bytes_downloaded,
                attempt, max_attempts, error, manual_url,
                created, modified, finished
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(job_id, id) DO UPDATE SET
                name = excluded.name,
                project_id = excluded.project_id,
                version_id = excluded.version_id,
                status = excluded.status,
                bytes_total = excluded.bytes_total,
                bytes_downloaded = excluded.bytes_downloaded,
                attempt = excluded.attempt,
                max_attempts = excluded.max_attempts,
                error = excluded.error,
                manual_url = excluded.manual_url,
                modified = excluded.modified,
                finished = excluded.finished",
        )
        .bind(item.id)
        .bind(&id_value)
        .bind(item.name)
        .bind(item.project_id)
        .bind(item.version_id)
        .bind(status)
        .bind(item.bytes_total.map(|value| value as i64))
        .bind(item.bytes_downloaded as i64)
        .bind(item.attempt.map(i64::from))
        .bind(item.max_attempts.map(i64::from))
        .bind(item.error)
        .bind(item.manual_url)
        .bind(now)
        .bind(now)
        .bind(finished)
        .execute(&mut *transaction)
        .await?;
    }
    transaction.commit().await?;
    Ok(())
}
