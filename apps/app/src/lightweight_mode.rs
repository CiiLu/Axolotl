use serde::Serialize;
use std::sync::Mutex;
use tauri::{
    AppHandle, Listener, Manager, WebviewUrl, WebviewWindowBuilder,
    menu::{
        CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu,
    },
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    window::WindowBuilder,
};

const MAIN_WINDOW_LABEL: &str = "main";
const LIGHTWEIGHT_HOST_WINDOW_LABEL: &str = "lightweight-host";
const TRAY_ID: &str = "main";
const LAUNCH_INSTANCE_PREFIX: &str = "launch-instance:";

struct TrayLabels {
    show_launcher: &'static str,
    launch_instance: &'static str,
    lightweight_mode: &'static str,
    quit: &'static str,
    running_prefix: &'static str,
}

fn tray_labels(locale: &str) -> TrayLabels {
    if locale.eq_ignore_ascii_case("zh-CN") {
        TrayLabels {
            show_launcher: "显示 Axolotl 启动器",
            launch_instance: "启动实例",
            lightweight_mode: "轻量模式",
            quit: "退出",
            running_prefix: "正在运行：",
        }
    } else if locale.eq_ignore_ascii_case("zh-TW") {
        TrayLabels {
            show_launcher: "顯示 Axolotl 啟動器",
            launch_instance: "啟動實例",
            lightweight_mode: "輕量模式",
            quit: "結束",
            running_prefix: "正在執行：",
        }
    } else {
        TrayLabels {
            show_launcher: "Show Axolotl Launcher",
            launch_instance: "Launch instance",
            lightweight_mode: "Lightweight mode",
            quit: "Quit",
            running_prefix: "Running: ",
        }
    }
}

struct LightweightModeState {
    active: bool,
    route: String,
    running_processes: usize,
    pending_crash: Option<PendingCrash>,
}

impl Default for LightweightModeState {
    fn default() -> Self {
        Self {
            active: false,
            route: "/".to_string(),
            running_processes: 0,
            pending_crash: None,
        }
    }
}

#[derive(Serialize)]
pub struct PendingCrash {
    pub instance_id: String,
    pub uuid: String,
}

#[derive(Default)]
pub struct LightweightMode(Mutex<LightweightModeState>);

impl LightweightMode {
    fn enter(&self, app: &AppHandle) -> Result<(), String> {
        let state = self.0.lock().map_err(|error| error.to_string())?;
        if state.active {
            return Ok(());
        }
        if state.running_processes == 0 {
            return Err(
                "Lightweight mode requires a running Minecraft instance"
                    .to_string(),
            );
        }
        drop(state);
        create_lightweight_host_window(app)?;
        let mut state = self.0.lock().map_err(|error| error.to_string())?;
        if state.active {
            drop(state);
            destroy_lightweight_host_window(app);
            return Ok(());
        }

        state.active = true;
        drop(state);
        if let Err(error) = destroy_main_window(app) {
            if let Ok(mut state) = self.0.lock() {
                state.active = false;
            }
            destroy_lightweight_host_window(app);
            return Err(error);
        }
        update_tray_menu(app, true, true);
        Ok(())
    }

    pub fn exit(&self, app: &AppHandle) -> Result<(), String> {
        let state = self.0.lock().map_err(|error| error.to_string())?;
        if !state.active {
            return show_main_window(app);
        }

        let route = state.route.clone();
        drop(state);
        create_main_window(app, &route)?;
        destroy_lightweight_host_window(app);
        if let Ok(mut state) = self.0.lock() {
            state.active = false;
        }
        update_tray_menu(app, false, self.has_running_processes());
        Ok(())
    }

    fn is_active(&self) -> bool {
        self.0.lock().map(|state| state.active).unwrap_or(false)
    }

    fn has_running_processes(&self) -> bool {
        self.0
            .lock()
            .map(|state| state.running_processes > 0)
            .unwrap_or(false)
    }

    fn process_event(&self, app: &AppHandle, payload: ProcessEventPayload) {
        if payload.lightweight_replay {
            return;
        }

        let restore_window = {
            let mut state = match self.0.lock() {
                Ok(state) => state,
                Err(error) => {
                    tracing::error!(
                        "Failed to lock lightweight mode state: {error}"
                    );
                    return;
                }
            };
            match payload.event.as_str() {
                "launched" => {
                    state.running_processes += 1;
                    None
                }
                "finished" => {
                    state.running_processes =
                        state.running_processes.saturating_sub(1);
                    let crashed = payload.crashed == Some(true);
                    if state.active && crashed {
                        state.pending_crash = Some(PendingCrash {
                            instance_id: payload.instance_id,
                            uuid: payload.uuid,
                        });
                    }
                    let should_restore = if state.active {
                        crashed || state.running_processes == 0
                    } else {
                        state.running_processes == 0
                    };
                    if should_restore {
                        let was_lightweight = state.active;
                        state.active = false;
                        Some((state.route.clone(), was_lightweight))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        };

        update_tray_menu(app, self.is_active(), self.has_running_processes());
        match restore_window {
            Some((route, was_lightweight)) => {
                let result = if was_lightweight {
                    create_main_window(app, &route).map(|()| {
                        destroy_lightweight_host_window(app);
                    })
                } else {
                    show_main_window(app)
                };
                if let Err(error) = result {
                    tracing::error!(
                        "Failed to restore launcher after Minecraft exited: {error}"
                    );
                }
            }
            None if payload.event == "launched" => {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let settings = match theseus::settings::get().await {
                        Ok(settings) => settings,
                        Err(error) => {
                            tracing::warn!(
                                "Failed to read lightweight mode setting: {error}"
                            );
                            return;
                        }
                    };
                    if settings.enter_lightweight_mode_on_game_launch {
                        let state = app.state::<LightweightMode>();
                        if let Err(error) = state.enter(&app) {
                            tracing::warn!(
                                "Failed to enter lightweight mode: {error}"
                            );
                        }
                    } else if settings.hide_on_process_start
                        && let Some(window) =
                            app.get_webview_window(MAIN_WINDOW_LABEL)
                        && let Err(error) = window.minimize()
                    {
                        tracing::warn!(
                            "Failed to minimize launcher after Minecraft started: {error}"
                        );
                    }
                });
            }
            None => {}
        }
    }

    fn set_route(&self, route: String) {
        if route.starts_with('/') {
            if let Ok(mut state) = self.0.lock() {
                state.route = route;
            }
        }
    }

    fn take_pending_crash(&self) -> Option<PendingCrash> {
        self.0
            .lock()
            .ok()
            .and_then(|mut state| state.pending_crash.take())
    }
}

#[derive(serde::Deserialize)]
struct ProcessEventPayload {
    instance_id: String,
    uuid: String,
    event: String,
    crashed: Option<bool>,
    #[serde(default)]
    lightweight_replay: bool,
}

#[tauri::command]
pub fn lightweight_mode_frontend_ready(
    app: AppHandle,
    route: String,
) -> Result<Option<PendingCrash>, String> {
    let state = app.state::<LightweightMode>();
    state.set_route(route);
    let pending_crash = state.take_pending_crash();
    update_tray_menu(&app, state.is_active(), state.has_running_processes());
    Ok(pending_crash)
}

#[tauri::command]
pub fn lightweight_mode_set_route(app: AppHandle, route: String) {
    app.state::<LightweightMode>().set_route(route);
}

fn destroy_main_window(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        window.destroy().map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn create_lightweight_host_window(app: &AppHandle) -> Result<(), String> {
    if app.get_window(LIGHTWEIGHT_HOST_WINDOW_LABEL).is_none() {
        WindowBuilder::new(app, LIGHTWEIGHT_HOST_WINDOW_LABEL)
            .title("Axolotl Launcher")
            .visible(false)
            .focused(false)
            .focusable(false)
            .skip_taskbar(true)
            .build()
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn destroy_lightweight_host_window(app: &AppHandle) {
    if let Some(window) = app.get_window(LIGHTWEIGHT_HOST_WINDOW_LABEL)
        && let Err(error) = window.destroy()
    {
        tracing::warn!("Failed to destroy lightweight host window: {error}");
    }
}

fn create_main_window(app: &AppHandle, route: &str) -> Result<(), String> {
    if app.get_webview_window(MAIN_WINDOW_LABEL).is_none() {
        let mut builder = WebviewWindowBuilder::new(
            app,
            MAIN_WINDOW_LABEL,
            WebviewUrl::App(route.into()),
        )
        .title("Axolotl Launcher")
        .inner_size(1280.0, 800.0)
        .min_inner_size(1100.0, 700.0)
        .resizable(true)
        .transparent(true)
        .visible(false);
        #[cfg(not(target_os = "macos"))]
        {
            builder = builder.decorations(false);
        }
        builder.build().map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn show_main_window(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        window.show().map_err(|error| error.to_string())?;
        window.unminimize().map_err(|error| error.to_string())?;
        window.set_focus().map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn update_tray_menu(
    app: &AppHandle,
    active: bool,
    has_running_processes: bool,
) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(error) =
            rebuild_tray_menu(&app, active, has_running_processes).await
        {
            tracing::warn!("Failed to update tray menu: {error}");
        }
    });
}

async fn rebuild_tray_menu(
    app: &AppHandle,
    active: bool,
    has_running_processes: bool,
) -> Result<(), String> {
    let locale = theseus::settings::get()
        .await
        .map(|settings| settings.locale)
        .unwrap_or_default();
    let labels = tray_labels(&locale);
    let show_launcher = MenuItem::with_id(
        app,
        "show-launcher",
        labels.show_launcher,
        true,
        None::<&str>,
    )
    .map_err(|error| error.to_string())?;
    let lightweight_mode = CheckMenuItem::with_id(
        app,
        "lightweight-mode",
        labels.lightweight_mode,
        has_running_processes,
        active,
        None::<&str>,
    )
    .map_err(|error| error.to_string())?;
    let quit = MenuItem::with_id(app, "quit", labels.quit, true, None::<&str>)
        .map_err(|error| error.to_string())?;
    let first_separator = PredefinedMenuItem::separator(app)
        .map_err(|error| error.to_string())?;
    let second_separator = PredefinedMenuItem::separator(app)
        .map_err(|error| error.to_string())?;
    let instances = theseus::instance::list().await.unwrap_or_else(|error| {
        tracing::debug!("Tray instance list is unavailable: {error}");
        Vec::new()
    });
    let mut instance_items = Vec::with_capacity(instances.len());
    let running_instance_ids = theseus::process::get_all()
        .await
        .map(|processes| {
            processes
                .into_iter()
                .map(|process| process.instance_id)
                .collect::<std::collections::HashSet<_>>()
        })
        .unwrap_or_default();
    for instance in instances {
        let running = running_instance_ids.contains(&instance.instance.id);
        instance_items.push(
            MenuItem::with_id(
                app,
                format!("{LAUNCH_INSTANCE_PREFIX}{}", instance.instance.id),
                if running {
                    format!(
                        "{}{}",
                        labels.running_prefix, instance.instance.name
                    )
                } else {
                    instance.instance.name.clone()
                },
                !running,
                None::<&str>,
            )
            .map_err(|error| error.to_string())?,
        );
    }
    let instance_references: Vec<&dyn IsMenuItem<_>> = instance_items
        .iter()
        .map(|item| item as &dyn IsMenuItem<_>)
        .collect();
    let launch_instances = Submenu::with_items(
        app,
        labels.launch_instance,
        true,
        &instance_references,
    )
    .map_err(|error| error.to_string())?;
    let menu = Menu::with_items(
        app,
        &[
            &show_launcher,
            &first_separator,
            &launch_instances,
            &lightweight_mode,
            &second_separator,
            &quit,
        ],
    )
    .map_err(|error| error.to_string())?;
    app.tray_by_id(TRAY_ID)
        .ok_or_else(|| "Tray icon is unavailable".to_string())?
        .set_menu(Some(menu))
        .map_err(|error| error.to_string())
}

fn handle_menu_event(app: &AppHandle, id: &str) {
    match id {
        "show-launcher" => {
            let _ = app.state::<LightweightMode>().exit(app);
        }
        "lightweight-mode" => {
            let state = app.state::<LightweightMode>();
            if state.is_active() {
                let _ = state.exit(app);
            } else if let Err(error) = state.enter(app) {
                tracing::debug!(
                    "Lightweight mode was not entered from tray: {error}"
                );
            }
        }
        "quit" => app.exit(0),
        instance_id if instance_id.starts_with(LAUNCH_INSTANCE_PREFIX) => {
            let instance_id = instance_id
                .trim_start_matches(LAUNCH_INSTANCE_PREFIX)
                .to_string();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = theseus::instance::run(
                    &instance_id,
                    theseus::instance::QuickPlayType::None,
                    false,
                )
                .await
                {
                    tracing::error!(
                        "Failed to launch tray instance {instance_id}: {error}"
                    );
                }
            });
        }
        _ => {}
    }
}

pub fn init(app: &AppHandle) {
    app.manage(LightweightMode::default());
    let tray = TrayIconBuilder::with_id(TRAY_ID)
        .icon(
            app.default_window_icon()
                .expect("missing default app icon")
                .clone(),
        )
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| handle_menu_event(app, event.id.as_ref()))
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                let _ = tray
                    .app_handle()
                    .state::<LightweightMode>()
                    .exit(&tray.app_handle());
            }
        })
        .build(app)
        .expect("failed to create system tray");
    let _tray = tray;
    update_tray_menu(app, false, false);
    let app_handle = app.clone();
    app.listen("process", move |event| {
        let Ok(payload) =
            serde_json::from_str::<ProcessEventPayload>(event.payload())
        else {
            return;
        };
        app_handle
            .state::<LightweightMode>()
            .process_event(&app_handle, payload);
    });
    let app_handle = app.clone();
    app.listen("instance", move |_| {
        let state = app_handle.state::<LightweightMode>();
        update_tray_menu(
            &app_handle,
            state.is_active(),
            state.has_running_processes(),
        );
    });
}
