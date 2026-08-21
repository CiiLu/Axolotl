use tauri::{
	menu::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
	tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
	AppHandle, Listener, Manager,
};

const TRAY_ID: &str = "main";
const LAUNCH_INSTANCE_PREFIX: &str = "launch-instance:";

pub fn init(app: &AppHandle) {
	let tray = TrayIconBuilder::with_id(TRAY_ID)
		.icon(app.default_window_icon().expect("missing default app icon").clone())
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
				show_main_window(&tray.app_handle());
			}
		})
		.build(app)
		.expect("failed to create system tray");
	let _tray = tray;
	update_menu(app);
	let app_handle = app.clone();
	app.listen("instance", move |_| update_menu(&app_handle));
}

fn show_main_window(app: &AppHandle) {
	if let Some(window) = app.get_webview_window("main") {
		let _ = window.show();
		let _ = window.unminimize();
		let _ = window.set_focus();
	}
}

fn handle_menu_event(app: &AppHandle, id: &str) {
	match id {
		"show-launcher" => show_main_window(app),
		"quit" => app.exit(0),
		instance_id if instance_id.starts_with(LAUNCH_INSTANCE_PREFIX) => {
			let instance_id = instance_id.trim_start_matches(LAUNCH_INSTANCE_PREFIX).to_string();
			tauri::async_runtime::spawn(async move {
				if let Err(error) = theseus::instance::run(
					&instance_id,
					theseus::instance::QuickPlayType::None,
					false,
				)
				.await
				{
					tracing::error!("Failed to launch tray instance {instance_id}: {error}");
				}
			});
		}
		_ => {}
	}
}

fn update_menu(app: &AppHandle) {
	let app = app.clone();
	tauri::async_runtime::spawn(async move {
		let instances = theseus::instance::list().await.unwrap_or_else(|error| {
			tracing::debug!("Tray instance list is unavailable: {error}");
			Vec::new()
		});
		let show_launcher = MenuItem::with_id(&app, "show-launcher", "Show Axolotl Launcher", true, None::<&str>);
		let quit = MenuItem::with_id(&app, "quit", "Quit", true, None::<&str>);
		let separator = PredefinedMenuItem::separator(&app);
		let (Ok(show_launcher), Ok(quit), Ok(separator)) = (show_launcher, quit, separator) else {
			return;
		};
		let instance_items = instances
			.into_iter()
			.filter_map(|instance| {
				MenuItem::with_id(
					&app,
					format!("{LAUNCH_INSTANCE_PREFIX}{}", instance.instance.id),
					instance.instance.name,
					true,
					None::<&str>,
				)
				.ok()
			})
			.collect::<Vec<_>>();
		let instance_references = instance_items
			.iter()
			.map(|item| item as &dyn IsMenuItem<_>)
			.collect::<Vec<_>>();
		let Ok(launch_instances) = Submenu::with_items(&app, "Launch instance", true, &instance_references) else {
			return;
		};
		let Ok(menu) = Menu::with_items(&app, &[&show_launcher, &separator, &launch_instances, &quit]) else {
			return;
		};
		if let Some(tray) = app.tray_by_id(TRAY_ID) {
			let _ = tray.set_menu(Some(menu));
		}
	});
}
