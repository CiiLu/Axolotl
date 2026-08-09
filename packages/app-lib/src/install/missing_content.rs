use super::events::InstallProgressReporter;
use super::model::{
    DownloadItemStatus, InstallJobEventKind, InstallJobSnapshot,
    InstallJobState, InstallJobStatus, InstallRequest, InstallTarget,
    MissingModpackFileState,
};
use super::{runner, store};
use crate::State;
use crate::util::fetch::{
    ContentValidation, DownloadRequest, Integrity, ResourceClass,
    download_to_path, verify_file,
};
use path_util::SafeRelativeUtf8UnixPathBuf;
use serde::Serialize;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingModpackContentView {
    pub remaining: usize,
    pub files: Vec<MissingModpackFileView>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingModpackFileView {
    pub item_id: String,
    pub path: String,
    pub expected_size: u64,
    pub status: DownloadItemStatus,
    pub last_error: Option<String>,
    pub browser_urls: Vec<String>,
    pub attempt: Option<u32>,
    pub max_attempts: Option<u32>,
}

pub async fn list_missing_modpack_files(
    job_id: Uuid,
) -> crate::Result<MissingModpackContentView> {
    let state = State::get().await?;
    let job = waiting_job(job_id, &state).await?;
    Ok(missing_content_view(&job.state)?)
}

pub async fn retry_missing_modpack_file(
    job_id: Uuid,
    item_id: String,
) -> crate::Result<InstallJobSnapshot> {
    let state = State::get().await?;
    let _permit = state.install_job_semaphore.acquire().await?;
    let job = waiting_job(job_id, &state).await?;
    let file = pending_file(&job.state, &item_id)?.clone();
    let instance_base = instance_base(&job.state, &state).await?;
    let target = super::recovery::checked_instance_path(
        &instance_base,
        &file.target_path,
    )?;
    let integrity = required_integrity(&file)?;
    let Some(primary_url) = file.download_urls.first() else {
        return Err(crate::ErrorKind::InputError(
            "This modpack file has no automatic download URL".to_string(),
        )
        .into());
    };
    let current = job
        .state
        .download_items()
        .into_iter()
        .find(|item| item.id == item_id)
        .ok_or_else(|| unknown_item_error(&item_id))?;
    let attempt = current.attempt.unwrap_or(0).saturating_add(1);
    let max_attempts = current.max_attempts.unwrap_or(1).max(attempt);
    let reporter = InstallProgressReporter::new(job_id, job.state.clone());
    reporter
        .record_events(vec![InstallJobEventKind::ContentFileDownloadAttempt {
            path: item_id.clone(),
            bytes_total: Some(file.expected_size),
            attempt,
            max_attempts,
        }])
        .await?;

    let result = download_to_path(
        DownloadRequest::new(primary_url, ResourceClass::Modpack)
            .with_candidate_urls(file.download_urls.iter().skip(1).cloned())
            .with_integrity(integrity)
            .with_install_tracking(
                reporter.clone(),
                item_id.clone(),
                file.target_path.clone(),
            ),
        &target,
        &state.download_semaphore,
        &state.pool,
        None,
    )
    .await;
    match result {
        Ok(download) => {
            reporter
                .record_events(vec![
                    InstallJobEventKind::ContentFileRecovered {
                        path: item_id,
                        bytes: download.size,
                    },
                ])
                .await?;
            resume_if_complete(job_id, &state).await
        }
        Err(error) => {
            tracing::warn!(job_id = %job_id, item_id, %error, "Manual required-file retry failed");
            reporter
                .record_events(vec![InstallJobEventKind::ContentFileFailed {
                    path: item_id,
                    reason: "Automatic download failed. Use browser download or choose the required file locally."
                        .to_string(),
                    project_id: None,
                    version_id: None,
                }])
                .await?;
            Err(crate::ErrorKind::InputError(
                "Automatic download failed. Try browser download or choose a local file."
                    .to_string(),
            )
            .into())
        }
    }
}

pub async fn import_missing_modpack_file(
    job_id: Uuid,
    item_id: String,
    selected_file_path: PathBuf,
) -> crate::Result<InstallJobSnapshot> {
    let state = State::get().await?;
    let _permit = state.install_job_semaphore.acquire().await?;
    let job = waiting_job(job_id, &state).await?;
    let file = pending_file(&job.state, &item_id)?.clone();
    let instance_base = instance_base(&job.state, &state).await?;
    let target = super::recovery::checked_instance_path(
        &instance_base,
        &file.target_path,
    )?;
    let integrity = required_integrity(&file)?;
    let reporter = InstallProgressReporter::new(job_id, job.state.clone());
    reporter
        .record_events(vec![
            InstallJobEventKind::ContentFileVerificationStarted {
                path: item_id.clone(),
            },
        ])
        .await?;

    let import = materialize_verified_file(
        &selected_file_path,
        &target,
        &integrity,
        || {
            let reporter = reporter.clone();
            let item_id = item_id.clone();
            async move {
                reporter
                    .record_events(vec![
                        InstallJobEventKind::ContentFileWritingStarted {
                            path: item_id,
                        },
                    ])
                    .await
                    .map(|_| ())
            }
        },
    )
    .await;

    match import {
        Ok(size) => {
            reporter
                .record_events(vec![
                    InstallJobEventKind::ContentFileRecovered {
                        path: item_id,
                        bytes: size,
                    },
                ])
                .await?;
            resume_if_complete(job_id, &state).await
        }
        Err(error) => {
            tracing::warn!(job_id = %job_id, item_id, selected_path = %selected_file_path.display(), %error, "Selected modpack file was rejected");
            let message = user_import_error(&error);
            reporter
                .record_events(vec![InstallJobEventKind::ContentFileFailed {
                    path: item_id,
                    reason: message.clone(),
                    project_id: None,
                    version_id: None,
                }])
                .await?;
            Err(crate::ErrorKind::InputError(message).into())
        }
    }
}

pub(crate) fn browser_download_urls(downloads: &[String]) -> Vec<String> {
    downloads
        .iter()
        .filter_map(|download| {
            let url = reqwest::Url::parse(download).ok()?;
            if !matches!(url.scheme(), "http" | "https")
                || !url.username().is_empty()
                || url.password().is_some()
                || url.query_pairs().any(|(name, _)| {
                    matches!(
                        name.to_ascii_lowercase().as_str(),
                        "auth"
                            | "authorization"
                            | "key"
                            | "api_key"
                            | "apikey"
                            | "access_token"
                            | "sig"
                            | "signature"
                            | "token"
                            | "x-amz-signature"
                            | "x-goog-signature"
                    )
                })
            {
                return None;
            }
            Some(url.to_string())
        })
        .collect()
}

fn missing_content_view(
    job_state: &InstallJobState,
) -> crate::Result<MissingModpackContentView> {
    let content = job_state.missing_content.as_ref().ok_or_else(|| {
        crate::ErrorKind::InputError(
            "This install job has no persisted missing-content context"
                .to_string(),
        )
    })?;
    let items = job_state.download_items();
    let files = content
        .files
        .iter()
        .filter_map(|file| {
            let item = items.iter().find(|item| item.id == file.item_id)?;
            (!matches!(
                item.status,
                DownloadItemStatus::Completed | DownloadItemStatus::Skipped
            ))
            .then(|| MissingModpackFileView {
                item_id: file.item_id.clone(),
                path: file.target_path.clone(),
                expected_size: file.expected_size,
                status: item.status,
                last_error: item.error.clone(),
                browser_urls: file.browser_urls.clone(),
                attempt: item.attempt,
                max_attempts: item.max_attempts,
            })
        })
        .collect::<Vec<_>>();
    Ok(MissingModpackContentView {
        remaining: files.len(),
        files,
    })
}

async fn waiting_job(
    job_id: Uuid,
    state: &State,
) -> crate::Result<store::InstallJobRecord> {
    let job = store::get_required(job_id, state).await?;
    if job.status != InstallJobStatus::WaitingForUser {
        return Err(crate::ErrorKind::InputError(
            "Missing modpack files can only be resolved while the job is waiting for user action"
                .to_string(),
        )
        .into());
    }
    if job.state.missing_content.is_none() {
        return Err(crate::ErrorKind::InputError(
            "This job is not waiting for required Modrinth pack content"
                .to_string(),
        )
        .into());
    }
    if !matches!(
        job.state.request,
        InstallRequest::CreateModpackInstance { .. }
            | InstallRequest::InstallPackToExistingInstance { .. }
    ) {
        return Err(crate::ErrorKind::InputError(
            "Missing-content resolution is only available for modpack install jobs"
                .to_string(),
        )
        .into());
    }
    Ok(job)
}

fn pending_file<'a>(
    job_state: &'a InstallJobState,
    item_id: &str,
) -> crate::Result<&'a MissingModpackFileState> {
    SafeRelativeUtf8UnixPathBuf::try_from(item_id.to_string())?;
    let file = job_state
        .missing_content
        .as_ref()
        .and_then(|content| {
            content.files.iter().find(|file| file.item_id == item_id)
        })
        .ok_or_else(|| unknown_item_error(item_id))?;
    let item = job_state
        .download_items()
        .into_iter()
        .find(|item| item.id == item_id)
        .ok_or_else(|| unknown_item_error(item_id))?;
    if item.status != DownloadItemStatus::Failed {
        return Err(crate::ErrorKind::InputError(
            "Only failed required modpack files can be resolved".to_string(),
        )
        .into());
    }
    Ok(file)
}

fn unknown_item_error(item_id: &str) -> crate::Error {
    crate::ErrorKind::InputError(format!(
        "Required modpack item does not belong to this job: {item_id}"
    ))
    .into()
}

fn required_integrity(
    file: &MissingModpackFileState,
) -> crate::Result<Integrity> {
    if file
        .sha1
        .as_ref()
        .is_some_and(|hash| !valid_hex_hash(hash, 40))
        || file
            .sha512
            .as_ref()
            .is_some_and(|hash| !valid_hex_hash(hash, 128))
        || file.sha1.is_none() && file.sha512.is_none()
    {
        return Err(crate::ErrorKind::InputError(
            "Modpack file has invalid or missing cryptographic integrity metadata"
                .to_string(),
        )
        .into());
    }
    Ok(Integrity {
        size: Some(file.expected_size),
        sha1: file.sha1.clone(),
        sha512: file.sha512.clone(),
        content: if file.validate_as_jar {
            ContentValidation::Jar
        } else {
            ContentValidation::None
        },
        ..Integrity::default()
    })
}

fn valid_hex_hash(hash: &str, length: usize) -> bool {
    hash.len() == length && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
}

async fn instance_base(
    job_state: &InstallJobState,
    state: &State,
) -> crate::Result<PathBuf> {
    let instance_id = match &job_state.target {
        InstallTarget::NewInstance {
            instance_id: Some(instance_id),
        }
        | InstallTarget::ExistingInstance { instance_id } => instance_id,
        InstallTarget::NewInstance { instance_id: None } => {
            return Err(crate::ErrorKind::InputError(
                "Install job has no target instance".to_string(),
            )
            .into());
        }
    };
    let instance = crate::state::get_instance(instance_id, &state.pool)
        .await?
        .ok_or_else(|| {
            crate::ErrorKind::InputError(format!(
                "Unknown target instance {instance_id}"
            ))
        })?;
    Ok(state
        .directories
        .instances_dir()
        .join(instance.instance.path))
}

async fn materialize_verified_file<F, Fut>(
    selected_file: &Path,
    target: &Path,
    integrity: &Integrity,
    before_write: F,
) -> crate::Result<u64>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = crate::Result<()>>,
{
    let metadata =
        tokio::fs::symlink_metadata(selected_file)
            .await
            .map_err(|_| {
                crate::ErrorKind::InputError(
                    "Unable to read the selected file".to_string(),
                )
            })?;
    if !metadata.is_file() || crate::util::io::is_symlink_or_reparse(&metadata)
    {
        return Err(crate::ErrorKind::InputError(
            "The selected path is not a regular readable file".to_string(),
        )
        .into());
    }
    if integrity.size.is_some_and(|size| size != metadata.len()) {
        return Err(crate::ErrorKind::InputError(
            "The selected file size does not match the modpack requirement"
                .to_string(),
        )
        .into());
    }
    let parent = target.parent().ok_or_else(|| {
        crate::ErrorKind::InputError(
            "Required modpack target has no parent directory".to_string(),
        )
    })?;
    crate::util::io::create_dir_all(parent).await?;
    let staged = tempfile::NamedTempFile::new_in(parent)
        .map_err(|error| crate::util::io::IOError::with_path(error, parent))?
        .into_temp_path();
    crate::util::io::copy(selected_file, &staged).await?;
    let size = verify_file(&staged, integrity).await.map_err(|error| {
        tracing::debug!(%error, selected_path = %selected_file.display(), "Selected file integrity mismatch");
        crate::Error::from(crate::ErrorKind::InputError(
            "The selected file is not the version required by this modpack"
                .to_string(),
        ))
    })?;
    before_write().await?;
    let previous =
        crate::state::materialize_project_download(&staged, target).await?;
    if let Err(error) =
        crate::state::finalize_project_materialization(previous.as_deref())
            .await
    {
        crate::state::restore_project_materialization(
            target,
            previous.as_deref(),
        )
        .await?;
        return Err(error);
    }
    Ok(size)
}

fn user_import_error(error: &crate::Error) -> String {
    match error.raw.as_ref() {
        crate::ErrorKind::InputError(message) => message.clone(),
        _ => "Unable to import the selected file safely".to_string(),
    }
}

async fn resume_if_complete(
    job_id: Uuid,
    state: &State,
) -> crate::Result<InstallJobSnapshot> {
    let job = store::get_required(job_id, state).await?;
    if all_missing_content_resolved(&job.state)? {
        runner::resume_job(job_id).await
    } else {
        Ok(job.snapshot())
    }
}

fn all_missing_content_resolved(
    job_state: &InstallJobState,
) -> crate::Result<bool> {
    Ok(missing_content_view(job_state)?.remaining == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::pack::install_from::CreatePackLocation;
    use crate::install::model::{
        InstallPauseReason, MissingModpackContentState,
    };
    use crate::state::{InstanceLink, ModLoader};
    use sha1_smol::Sha1;

    fn integrity(bytes: &[u8]) -> Integrity {
        Integrity {
            size: Some(bytes.len() as u64),
            sha1: Some(Sha1::from(bytes).hexdigest()),
            ..Integrity::default()
        }
    }

    #[tokio::test]
    async fn verified_import_is_atomic_and_rejects_wrong_hashes() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("instance/mods/example.bin");
        crate::util::io::create_dir_all(target.parent().unwrap())
            .await
            .unwrap();
        crate::util::io::write(&target, b"existing-bad-file")
            .await
            .unwrap();
        let good = root.path().join("example.bin");
        let wrong_same_name = root.path().join("other/example.bin");
        let wrong_same_size = root.path().join("same-size.bin");
        crate::util::io::create_dir_all(wrong_same_name.parent().unwrap())
            .await
            .unwrap();
        crate::util::io::write(&good, b"required-content")
            .await
            .unwrap();
        crate::util::io::write(&wrong_same_name, b"wrong-content!!!")
            .await
            .unwrap();
        crate::util::io::write(&wrong_same_size, b"wrong-content!!!")
            .await
            .unwrap();

        let expected = integrity(b"required-content");
        assert!(
            materialize_verified_file(
                &wrong_same_name,
                &target,
                &expected,
                || async { Ok(()) }
            )
            .await
            .is_err()
        );
        assert_eq!(
            crate::util::io::read(&target).await.unwrap(),
            b"existing-bad-file"
        );
        assert!(
            materialize_verified_file(
                &wrong_same_size,
                &target,
                &expected,
                || async { Ok(()) }
            )
            .await
            .is_err()
        );
        assert_eq!(
            crate::util::io::read(&target).await.unwrap(),
            b"existing-bad-file"
        );

        let size =
            materialize_verified_file(&good, &target, &expected, || async {
                Ok(())
            })
            .await
            .unwrap();
        assert_eq!(size, 16);
        assert_eq!(
            crate::util::io::read(&target).await.unwrap(),
            b"required-content"
        );
    }

    #[test]
    fn browser_urls_only_use_public_manifest_urls() {
        let urls = browser_download_urls(&[
            "https://cdn.modrinth.com/data/project/versions/file.jar"
                .to_string(),
            "https://secret@example.com/file.jar".to_string(),
            "https://mirror.example/file.jar?api_key=secret".to_string(),
            "https://mirror.example/file.jar?x-amz-signature=secret"
                .to_string(),
            "file:///tmp/file.jar".to_string(),
            "https://fallback.example/file.jar".to_string(),
        ]);
        assert_eq!(
            urls,
            vec![
                "https://cdn.modrinth.com/data/project/versions/file.jar",
                "https://fallback.example/file.jar",
            ]
        );
    }

    #[test]
    fn last_recovered_file_is_the_auto_resume_trigger() {
        let file = MissingModpackFileState {
            item_id: "mods/only.bin".to_string(),
            manifest_path: "mods/only.bin".to_string(),
            target_path: "mods/only.bin".to_string(),
            expected_size: 8,
            sha1: Some(Sha1::from(b"required").hexdigest()),
            sha512: None,
            download_urls: vec!["https://cdn.example/only.bin".to_string()],
            browser_urls: vec!["https://cdn.example/only.bin".to_string()],
            validate_as_jar: false,
        };
        let mut job_state =
            InstallJobState::new(InstallRequest::DownloadJava {
                vendor: "test".to_string(),
                version: 21,
            });
        job_state.missing_content = Some(MissingModpackContentState {
            files: vec![file.clone()],
        });
        job_state.record_event(InstallJobEventKind::ContentFileQueued {
            path: file.item_id.clone(),
            bytes_total: Some(file.expected_size),
            max_attempts: 2,
        });
        job_state.record_event(InstallJobEventKind::ContentFileFailed {
            path: file.item_id.clone(),
            reason: "fixture failure".to_string(),
            project_id: None,
            version_id: None,
        });
        assert!(!all_missing_content_resolved(&job_state).unwrap());

        job_state.record_event(InstallJobEventKind::ContentFileRecovered {
            path: file.item_id,
            bytes: file.expected_size,
        });
        assert!(all_missing_content_resolved(&job_state).unwrap());
    }

    #[tokio::test]
    async fn import_api_trusts_persisted_job_context_and_keeps_other_missing_files_waiting()
     {
        crate::event::EventState::init().await.unwrap();
        let root = tempfile::tempdir().unwrap().keep();
        let state = State::init_for_test(root.to_string_lossy().to_string())
            .await
            .unwrap();
        let created = crate::api::instance::create(
            format!("Stage 3 {}", Uuid::new_v4()),
            "1.20.1".to_string(),
            ModLoader::Vanilla,
            None,
            None,
            InstanceLink::Unmanaged,
            None,
        )
        .await
        .unwrap();
        let instance_id = created.instance.id;
        let instance_base = state
            .directories
            .instances_dir()
            .join(created.instance.path);
        let first_bytes = b"required-one";
        let second_bytes = b"required-two";
        let first = MissingModpackFileState {
            item_id: "mods/one.bin".to_string(),
            manifest_path: "mods/one.bin".to_string(),
            target_path: "mods/one.bin".to_string(),
            expected_size: first_bytes.len() as u64,
            sha1: Some(Sha1::from(first_bytes).hexdigest()),
            sha512: None,
            download_urls: vec!["https://cdn.example/one.bin".to_string()],
            browser_urls: vec!["https://cdn.example/one.bin".to_string()],
            validate_as_jar: false,
        };
        let second = MissingModpackFileState {
            item_id: "mods/two.bin".to_string(),
            manifest_path: "mods/two.bin".to_string(),
            target_path: "mods/two.bin".to_string(),
            expected_size: second_bytes.len() as u64,
            sha1: Some(Sha1::from(second_bytes).hexdigest()),
            sha512: None,
            download_urls: vec!["https://cdn.example/two.bin".to_string()],
            browser_urls: vec!["https://cdn.example/two.bin".to_string()],
            validate_as_jar: false,
        };
        let mut job_state = InstallJobState::new(
            InstallRequest::InstallPackToExistingInstance {
                instance_id: instance_id.clone(),
                location: CreatePackLocation::FromFile {
                    path: root.join("stage-3.mrpack"),
                },
                post_install_edit: None,
            },
        );
        job_state.missing_content = Some(MissingModpackContentState {
            files: vec![first.clone(), second.clone()],
        });
        job_state.pause_reason =
            Some(InstallPauseReason::MissingRequiredContent {
                failed_files: 2,
                paths: vec![first.item_id.clone(), second.item_id.clone()],
            });
        for file in [&first, &second] {
            job_state.record_event(InstallJobEventKind::ContentFileQueued {
                path: file.item_id.clone(),
                bytes_total: Some(file.expected_size),
                max_attempts: 2,
            });
            job_state.record_event(
                InstallJobEventKind::ContentFileBrowserOptions {
                    path: file.item_id.clone(),
                    urls: file.browser_urls.clone(),
                },
            );
            job_state.record_event(InstallJobEventKind::ContentFileFailed {
                path: file.item_id.clone(),
                reason: "fixture download failed".to_string(),
                project_id: None,
                version_id: None,
            });
        }
        let job_id = Uuid::new_v4();
        store::insert(
            job_id,
            &job_state,
            InstallJobStatus::WaitingForUser,
            &state,
        )
        .await
        .unwrap();

        let queued_job_id = Uuid::new_v4();
        store::insert(
            queued_job_id,
            &job_state,
            InstallJobStatus::Queued,
            &state,
        )
        .await
        .unwrap();
        assert!(list_missing_modpack_files(queued_job_id).await.is_err());

        let selected = root.join("one-selected.bin");
        crate::util::io::write(&selected, first_bytes)
            .await
            .unwrap();
        assert!(
            import_missing_modpack_file(
                job_id,
                "mods/not-in-job.bin".to_string(),
                selected.clone(),
            )
            .await
            .is_err()
        );
        assert!(
            import_missing_modpack_file(
                job_id,
                "../outside.bin".to_string(),
                selected.clone(),
            )
            .await
            .is_err()
        );

        let target = instance_base.join(&first.target_path);
        crate::util::io::create_dir_all(target.parent().unwrap())
            .await
            .unwrap();
        crate::util::io::write(&target, b"bad-target")
            .await
            .unwrap();
        let snapshot = import_missing_modpack_file(
            job_id,
            first.item_id.clone(),
            selected,
        )
        .await
        .unwrap();
        assert_eq!(snapshot.status, InstallJobStatus::WaitingForUser);
        assert_eq!(crate::util::io::read(&target).await.unwrap(), first_bytes);
        assert_eq!(
            snapshot
                .items
                .iter()
                .find(|item| item.id == first.item_id)
                .unwrap()
                .status,
            DownloadItemStatus::Completed
        );
        assert_eq!(
            list_missing_modpack_files(job_id).await.unwrap().remaining,
            1
        );
        assert!(matches!(
            snapshot.pause_reason,
            Some(InstallPauseReason::MissingRequiredContent {
                failed_files: 1,
                ..
            })
        ));
        assert!(
            import_missing_modpack_file(
                job_id,
                first.item_id,
                root.join("one-selected.bin"),
            )
            .await
            .is_err()
        );

        let second_target = instance_base.join(&second.target_path);
        crate::util::io::write(&second_target, b"keep-this-bad-target")
            .await
            .unwrap();
        let wrong = root.join("wrong-two.bin");
        crate::util::io::write(&wrong, b"wrong--two!!")
            .await
            .unwrap();
        assert!(
            import_missing_modpack_file(job_id, second.item_id.clone(), wrong)
                .await
                .is_err()
        );
        assert_eq!(
            crate::util::io::read(&second_target).await.unwrap(),
            b"keep-this-bad-target"
        );
    }
}
