//! Theseus state management system
use crate::util::fetch::{FetchSemaphore, IoSemaphore};
use dashmap::DashMap;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use tokio::sync::Mutex as AsyncMutex;
use tokio::sync::{OnceCell, Semaphore};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::state::instances::watcher::FileWatcher;
use sqlx::SqlitePool;

// Submodules
mod dirs;
pub use self::dirs::*;

mod instance_types;
pub use self::instance_types::*;

pub(crate) mod instances;
pub use self::instances::*;

mod settings;
pub use self::settings::*;

mod installer_settings;

mod process;
pub use self::process::*;

mod java_globals;
pub use self::java_globals::*;

mod discovered_javas;
pub use self::discovered_javas::*;

mod discord;
pub use self::discord::*;

mod minecraft_auth;
pub use self::minecraft_auth::*;

pub mod minecraft_skins;

mod cache;
pub use self::cache::*;

mod friends;
pub use self::friends::*;

mod tunnel;
pub use self::tunnel::*;

pub mod db;
pub(crate) mod db_backup;
mod mr_auth;

pub use self::mr_auth::*;

mod legacy_converter;

pub mod attached_world_data;
pub mod server_join_log;

// Global state
// RwLock on state only has concurrent reads, except for config dir change which takes control of the State
static LAUNCHER_STATE: OnceCell<Arc<State>> = OnceCell::const_new();
const MAX_CONCURRENT_INSTALL_JOBS: usize = 1;
pub struct State {
    /// Information on the location of files used in the launcher
    pub directories: DirectoryInfo,

    /// Semaphore used to limit concurrent network requests and avoid errors
    pub fetch_semaphore: FetchSemaphore,
    /// Global capacity for file transfers. Metadata and API requests use their
    /// own semaphores so they cannot delay an active installation.
    pub download_semaphore: FetchSemaphore,
    /// Semaphore used to limit concurrent I/O and avoid errors
    pub io_semaphore: IoSemaphore,
    /// Semaphore to limit concurrent API requests. This is separate from the fetch semaphore
    /// to keep API functionality while the app is performing intensive tasks.
    pub api_semaphore: FetchSemaphore,
    minecraft_metadata_source: AtomicU8,
    minecraft_file_source: AtomicU8,
    modrinth_source: AtomicU8,
    curseforge_source: AtomicU8,
    download_concurrency_target: AtomicUsize,
    download_concurrency_limit: AtomicUsize,
    fetch_concurrency_limit: AtomicUsize,
    api_concurrency_limit: AtomicUsize,
    pub(crate) install_job_semaphore: Semaphore,
    pub(crate) install_db_semaphore: Semaphore,
    pub(crate) install_job_cancellations: DashMap<Uuid, CancellationToken>,

    /// Discord RPC
    pub discord_rpc: DiscordGuard,

    /// Process manager
    pub process_manager: ProcessManager,

    // NOTE: we explicitly must NOT store the app identifier in the state object,
    // because creating the state object is fallible (e.g. database missing),
    // but we rely on the app identifier to create the state (data dir).
    //
    // /// App identifier string (like com.modrinth.AxolotlLauncher)
    // pub app_identifier: String,
    /// Friends socket
    pub friends_socket: FriendsSocket,

    pub restart_after_pending_update: AtomicBool,

    /// Per-instance locks serializing content writes against instance
    /// deletion, so a delete can never commit between a command loading an
    /// instance and writing rows that reference it.
    pub(crate) instance_locks: InstanceLockManager,

    pub(crate) pool: SqlitePool,

    pub(crate) file_watcher: FileWatcher,
}

/// Per-instance lock registry with task-local reentrancy.
///
/// Instance deletion removes the `instances` row (and cascades through content
/// files, entries, provider refs and update checks), while content commands
/// load the instance first and write those rows later. Without serialization a
/// concurrent delete can commit between the load and the write, surfacing as a
/// raw `SQLITE_CONSTRAINT_FOREIGNKEY` (code 787) error. The lock is reentrant
/// for the task that already holds it, since commands compose (update →
/// check → sync → record), while concurrent tasks are serialized per instance.
#[derive(Default)]
pub(crate) struct InstanceLockManager {
    locks: DashMap<String, Arc<AsyncMutex<()>>>,
    held_by_owner: std::sync::Mutex<HashMap<LockOwner, HashSet<String>>>,
}

impl InstanceLockManager {
    pub(crate) async fn lock(
        &self,
        instance_id: &str,
    ) -> InstanceLockGuard<'_> {
        let lock = self
            .locks
            .entry(instance_id.to_string())
            .or_insert_with(|| Arc::new(AsyncMutex::new(())))
            .clone();
        let owner = current_lock_owner();

        if self
            .held_by_owner
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .get(&owner)
            .is_some_and(|held| held.contains(instance_id))
        {
            return InstanceLockGuard {
                manager: self,
                inner: None,
                owner,
                instance_id: instance_id.to_string(),
            };
        }

        let inner = lock.lock_owned().await;
        self.held_by_owner
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .entry(owner)
            .or_default()
            .insert(instance_id.to_string());

        InstanceLockGuard {
            manager: self,
            inner: Some(inner),
            owner,
            instance_id: instance_id.to_string(),
        }
    }
}

/// Identity of the async execution context holding an instance lock.
///
/// Task IDs distinguish concurrent tasks on a multi-threaded runtime; when no
/// task context exists (for example the main test future) the thread ID is used
/// so re-entrant calls within the same context are still recognized.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum LockOwner {
    Task(tokio::task::Id),
    Thread(std::thread::ThreadId),
}

fn current_lock_owner() -> LockOwner {
    tokio::task::try_id()
        .map(LockOwner::Task)
        .unwrap_or_else(|| LockOwner::Thread(std::thread::current().id()))
}

/// RAII guard for an instance lock. Only the outermost holder releases the
/// underlying mutex and removes the owner from the reentrancy registry.
pub(crate) struct InstanceLockGuard<'a> {
    manager: &'a InstanceLockManager,
    inner: Option<tokio::sync::OwnedMutexGuard<()>>,
    owner: LockOwner,
    instance_id: String,
}

impl Drop for InstanceLockGuard<'_> {
    fn drop(&mut self) {
        if self.inner.is_some() {
            let mut held = self
                .manager
                .held_by_owner
                .lock()
                .unwrap_or_else(|poison| poison.into_inner());
            let mut remove_owner = false;
            if let Some(held_instances) = held.get_mut(&self.owner) {
                held_instances.remove(&self.instance_id);
                remove_owner = held_instances.is_empty();
            }
            if remove_owner {
                held.remove(&self.owner);
            }
        }
    }
}

fn grow_semaphore(
    semaphore: &Semaphore,
    current_limit: &AtomicUsize,
    target: usize,
) {
    let mut current = current_limit.load(Ordering::Acquire);

    while current < target {
        match current_limit.compare_exchange(
            current,
            target,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => {
                semaphore.add_permits(target - current);
                return;
            }
            Err(updated) => current = updated,
        }
    }
}

async fn shrink_semaphore(
    semaphore: &Semaphore,
    current_limit: &AtomicUsize,
    target: &AtomicUsize,
) {
    loop {
        if current_limit.load(Ordering::Acquire)
            <= target.load(Ordering::Acquire)
        {
            return;
        }

        let Ok(permit) = semaphore.acquire().await else {
            return;
        };

        loop {
            let current = current_limit.load(Ordering::Acquire);
            if current <= target.load(Ordering::Acquire) {
                drop(permit);
                return;
            }

            if current_limit
                .compare_exchange(
                    current,
                    current - 1,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
            {
                permit.forget();
                break;
            }
        }
    }
}

impl State {
    pub async fn init(app_identifier: String) -> crate::Result<()> {
        let state = LAUNCHER_STATE
            .get_or_try_init(move || Self::initialize_state(app_identifier))
            .await?;

        if let Err(e) =
            crate::install::recovery::recover_interrupted_jobs(state).await
        {
            tracing::error!("Error recovering interrupted install jobs: {e}");
        }

        tokio::task::spawn(async move {
            crate::util::fetch::cleanup_stale_partial_downloads(vec![
                state.directories.metadata_dir(),
                state.directories.caches_dir(),
            ]);

            instances::watcher::watch_instances_init(
                &state.file_watcher,
                &state.directories,
                &state.pool,
            )
            .await;

            let res = tokio::try_join!(
                state.discord_rpc.clear_to_default(true),
                instances::refresh_all_instances(),
                Settings::migrate(&state.pool),
                ModrinthCredentials::refresh_all(),
            );

            if let Err(e) = res {
                tracing::error!("Error running discord RPC: {e}");
            }

            // Axolotl does not connect to Modrinth's private friends socket.
        });

        Ok(())
    }

    /// Get the current launcher state, waiting for initialization
    pub async fn get() -> crate::Result<Arc<Self>> {
        if !LAUNCHER_STATE.initialized() {
            tracing::error!(
                "Attempted to get state before it is initialized - this should never happen!"
            );
            while !LAUNCHER_STATE.initialized() {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }

        Ok(Arc::clone(
            LAUNCHER_STATE.get().expect("State is not initialized!"),
        ))
    }

    pub fn initialized() -> bool {
        LAUNCHER_STATE.initialized()
    }

    pub(crate) fn minecraft_metadata_source(&self) -> DownloadSourceMode {
        DownloadSourceMode::from_u8(
            self.minecraft_metadata_source.load(Ordering::Relaxed),
        )
    }

    pub(crate) fn minecraft_file_source(&self) -> DownloadSourceMode {
        DownloadSourceMode::from_u8(
            self.minecraft_file_source.load(Ordering::Relaxed),
        )
    }

    pub(crate) fn modrinth_source(&self) -> DownloadSourceMode {
        DownloadSourceMode::from_u8(
            self.modrinth_source.load(Ordering::Relaxed),
        )
    }

    pub(crate) fn curseforge_source(&self) -> DownloadSourceMode {
        DownloadSourceMode::from_u8(
            self.curseforge_source.load(Ordering::Relaxed),
        )
    }

    pub(crate) fn download_concurrency(&self) -> usize {
        self.download_concurrency_target.load(Ordering::Acquire)
    }

    pub(crate) fn update_download_settings(
        self: &Arc<Self>,
        settings: &Settings,
    ) {
        self.minecraft_metadata_source
            .store(settings.minecraft_metadata_source as u8, Ordering::Relaxed);
        self.minecraft_file_source
            .store(settings.minecraft_file_source as u8, Ordering::Relaxed);
        self.modrinth_source
            .store(settings.modrinth_source as u8, Ordering::Relaxed);
        self.curseforge_source
            .store(settings.curseforge_source as u8, Ordering::Relaxed);
        self.resize_download_concurrency(
            settings.effective_max_concurrent_downloads(),
        );
    }

    fn resize_download_concurrency(self: &Arc<Self>, target: usize) {
        let target = target.clamp(1, 256);
        self.download_concurrency_target
            .store(target, Ordering::Release);

        grow_semaphore(
            &self.fetch_semaphore.0,
            &self.fetch_concurrency_limit,
            target,
        );
        grow_semaphore(
            &self.download_semaphore.0,
            &self.download_concurrency_limit,
            target,
        );
        grow_semaphore(
            &self.api_semaphore.0,
            &self.api_concurrency_limit,
            target,
        );

        if self.fetch_concurrency_limit.load(Ordering::Acquire) > target {
            let state = Arc::clone(self);
            tokio::spawn(async move {
                shrink_semaphore(
                    &state.fetch_semaphore.0,
                    &state.fetch_concurrency_limit,
                    &state.download_concurrency_target,
                )
                .await;
            });
        }

        if self.download_concurrency_limit.load(Ordering::Acquire) > target {
            let state = Arc::clone(self);
            tokio::spawn(async move {
                shrink_semaphore(
                    &state.download_semaphore.0,
                    &state.download_concurrency_limit,
                    &state.download_concurrency_target,
                )
                .await;
            });
        }

        if self.api_concurrency_limit.load(Ordering::Acquire) > target {
            let state = Arc::clone(self);
            tokio::spawn(async move {
                shrink_semaphore(
                    &state.api_semaphore.0,
                    &state.api_concurrency_limit,
                    &state.download_concurrency_target,
                )
                .await;
            });
        }
    }

    pub fn get_if_initialized() -> Option<Arc<Self>> {
        LAUNCHER_STATE.get().map(Arc::clone)
    }

    #[tracing::instrument]
    async fn initialize_state(
        app_identifier: String,
    ) -> crate::Result<Arc<Self>> {
        tracing::info!("Connecting to app database");
        let pool = db::connect(&app_identifier).await?;

        legacy_converter::migrate_legacy_data(&pool).await?;

        tracing::info!("Fetching app settings");
        let mut settings = Settings::get(&pool).await?;
        installer_settings::apply_pending_installer_directory(
            &mut settings,
            &pool,
            &app_identifier,
        )
        .await?;
        let download_concurrency =
            settings.effective_max_concurrent_downloads();
        let fetch_semaphore =
            FetchSemaphore(Semaphore::new(download_concurrency));
        let download_semaphore =
            FetchSemaphore(Semaphore::new(download_concurrency));
        let io_semaphore =
            IoSemaphore(Semaphore::new(settings.max_concurrent_writes));
        let api_semaphore =
            FetchSemaphore(Semaphore::new(download_concurrency));

        tracing::info!("Initializing directories");
        DirectoryInfo::move_launcher_directory(
            &mut settings,
            &pool,
            &io_semaphore,
            &app_identifier,
        )
        .await?;

        let directories =
            DirectoryInfo::init(settings.custom_dir, &app_identifier).await?;

        let discord_rpc = DiscordGuard::init()?;

        tracing::info!("Initializing file watcher");
        let file_watcher = instances::watcher::init_watcher().await?;

        let process_manager = ProcessManager::new();

        let friends_socket = FriendsSocket::new();

        Ok(Arc::new(Self {
            directories,
            fetch_semaphore,
            download_semaphore,
            io_semaphore,
            api_semaphore,
            minecraft_metadata_source: AtomicU8::new(
                settings.minecraft_metadata_source as u8,
            ),
            minecraft_file_source: AtomicU8::new(
                settings.minecraft_file_source as u8,
            ),
            modrinth_source: AtomicU8::new(settings.modrinth_source as u8),
            curseforge_source: AtomicU8::new(settings.curseforge_source as u8),
            download_concurrency_target: AtomicUsize::new(download_concurrency),
            download_concurrency_limit: AtomicUsize::new(download_concurrency),
            fetch_concurrency_limit: AtomicUsize::new(download_concurrency),
            api_concurrency_limit: AtomicUsize::new(download_concurrency),
            install_job_semaphore: Semaphore::new(MAX_CONCURRENT_INSTALL_JOBS),
            install_db_semaphore: Semaphore::new(1),
            install_job_cancellations: DashMap::new(),
            discord_rpc,
            process_manager,
            friends_socket,
            restart_after_pending_update: AtomicBool::new(false),
            instance_locks: InstanceLockManager::default(),
            pool,
            file_watcher,
            // app_identifier,
        }))
    }

    /// Acquire the lock serializing content writes and instance deletion for
    /// the given instance. Reentrant within the task that already holds it.
    pub(crate) async fn lock_instance_content(
        &self,
        instance_id: &str,
    ) -> InstanceLockGuard<'_> {
        self.instance_locks.lock(instance_id).await
    }
}

#[cfg(test)]
mod instance_lock_tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn serializes_concurrent_tasks_for_the_same_instance() {
        let manager = Arc::new(InstanceLockManager::default());
        let first = manager.lock("instance-1").await;

        let manager_for_task = Arc::clone(&manager);
        let mut contender = tokio::spawn(async move {
            let _guard = manager_for_task.lock("instance-1").await;
        });

        assert!(
            tokio::time::timeout(Duration::from_millis(100), &mut contender)
                .await
                .is_err(),
            "a second task must wait for the instance lock"
        );

        drop(first);
        tokio::time::timeout(Duration::from_millis(500), contender)
            .await
            .expect("contender acquires the lock after the holder drops")
            .unwrap();
    }

    #[tokio::test]
    async fn does_not_serialize_different_instances() {
        let manager = InstanceLockManager::default();
        let first = manager.lock("instance-1").await;
        let second = tokio::time::timeout(
            Duration::from_millis(200),
            manager.lock("instance-2"),
        )
        .await
        .expect("a different instance lock must be acquirable immediately");
        drop(first);
        drop(second);
    }

    #[tokio::test]
    async fn is_reentrant_within_the_same_task() {
        let manager = InstanceLockManager::default();
        let outer = manager.lock("instance-1").await;
        let inner = tokio::time::timeout(
            Duration::from_millis(200),
            manager.lock("instance-1"),
        )
        .await
        .expect("re-entering the same task must not deadlock");
        drop(inner);
        drop(outer);
    }

    #[tokio::test]
    async fn releases_the_lock_for_other_tasks_after_drop() {
        let manager = InstanceLockManager::default();
        let outer = manager.lock("instance-1").await;
        drop(outer);

        let acquired = tokio::time::timeout(
            Duration::from_millis(200),
            manager.lock("instance-1"),
        )
        .await
        .expect("the lock must be free after the guard drops");
        drop(acquired);
    }
}
