use std::{
    env, fs,
    io::Write,
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::{Command, ExitStatus},
    thread,
    time::Duration,
};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use native_dialog::DialogBuilder;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tao::{
    dpi::{LogicalPosition, LogicalSize},
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy},
    window::{Theme, WindowBuilder},
};
use wry::{NewWindowResponse, WebView, WebViewBuilder, http::Request};

const HTML: &str = include_str!("installer.html");
const LOGO: &[u8] = include_bytes!("../../app/icons/128x128.png");
const MAIN_BINARY_NAME: &str = "Axolotl Launcher.exe";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Bootstrap {
    version: String,
    install_dir: String,
    resource_dir: String,
    fresh_install: bool,
    language: Language,
    logo_data_url: String,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
enum Language {
    En,
    ZhCn,
}

#[derive(Debug)]
struct Arguments {
    installer: PathBuf,
    bootstrap: Bootstrap,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstallRequest {
    install_dir: String,
    resource_dir: String,
    desktop_shortcut: bool,
    launch_after: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
enum PathTarget {
    Install,
    Resource,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "command", rename_all = "camelCase")]
enum UiCommand {
    Minimize,
    DragWindow,
    Close,
    Browse { target: PathTarget, current: String },
    Install(InstallRequest),
    Finish { launch: bool },
}

#[derive(Debug)]
enum UserEvent {
    Minimize,
    DragWindow,
    Close,
    Browse { target: PathTarget, current: String },
    Install(InstallRequest),
    Progress(u8),
    Finished(Result<(), InstallFailure>),
    Finish { launch: bool },
}

#[derive(Debug)]
struct InstallFailure {
    exit_code: Option<i32>,
    message: String,
}

pub fn run() -> Result<(), String> {
    let arguments = parse_arguments()?;
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let window = WindowBuilder::new()
        .with_title("Axolotl Launcher")
        .with_inner_size(LogicalSize::new(940.0, 620.0))
        .with_min_inner_size(LogicalSize::new(940.0, 620.0))
        .with_max_inner_size(LogicalSize::new(940.0, 620.0))
        .with_resizable(false)
        .with_decorations(false)
        .with_theme(Some(Theme::Dark))
        .with_visible(false)
        .build(&event_loop)
        .map_err(|error| {
            format!("creating installer window failed: {error}")
        })?;

    if let Some(monitor) = window.current_monitor() {
        let screen = monitor.size().to_logical::<f64>(monitor.scale_factor());
        window.set_outer_position(LogicalPosition::new(
            (screen.width - 940.0).max(0.0) / 2.0,
            (screen.height - 620.0).max(0.0) / 2.0,
        ));
    }

    let bootstrap =
        serde_json::to_string(&arguments.bootstrap).map_err(|error| {
            format!("serializing installer settings failed: {error}")
        })?;
    let proxy = event_loop.create_proxy();
    let ipc_proxy = proxy.clone();
    let handler = move |request: Request<String>| {
        if let Ok(command) = serde_json::from_str::<UiCommand>(request.body()) {
            let event = match command {
                UiCommand::Minimize => UserEvent::Minimize,
                UiCommand::DragWindow => UserEvent::DragWindow,
                UiCommand::Close => UserEvent::Close,
                UiCommand::Browse { target, current } => {
                    UserEvent::Browse { target, current }
                }
                UiCommand::Install(request) => UserEvent::Install(request),
                UiCommand::Finish { launch } => UserEvent::Finish { launch },
            };
            let _ = ipc_proxy.send_event(event);
        }
    };

    let mut webview = Some(
        WebViewBuilder::new()
            .with_html(HTML)
            .with_initialization_script(format!(
                "window.__AXOLOTL_INSTALLER__ = {bootstrap};"
            ))
            .with_background_color((22, 24, 28, 255))
            .with_ipc_handler(handler)
            .with_new_window_req_handler(|_, _| NewWindowResponse::Deny)
            .with_navigation_handler(|url| url.starts_with("data:"))
            .with_devtools(false)
            .build(&window)
            .map_err(|error| format!("starting WebView2 failed: {error}"))?,
    );

    window.set_visible(true);
    window.set_focus();

    let installer = arguments.installer;
    let fresh_install = arguments.bootstrap.fresh_install;
    let mut installing = false;
    let mut install_dir = PathBuf::from(&arguments.bootstrap.install_dir);
    let mut launch_after_install = false;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => {}
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            }
            | Event::UserEvent(UserEvent::Close)
                if !installing =>
            {
                webview.take();
                *control_flow = ControlFlow::Exit;
            }
            Event::UserEvent(UserEvent::Minimize) => window.set_minimized(true),
            Event::UserEvent(UserEvent::DragWindow) => {
                let _ = window.drag_window();
            }
            Event::UserEvent(UserEvent::Browse { target, current }) => {
                let title = match target {
                    PathTarget::Install => {
                        "Select the program installation location"
                    }
                    PathTarget::Resource => "Select the application directory",
                };
                let mut dialog =
                    DialogBuilder::file().set_title(title).set_owner(&window);
                let current_path = PathBuf::from(current);
                if let Some(location) = dialog_initial_location(&current_path) {
                    dialog = dialog.set_location(&location);
                }
                if let Ok(Some(path)) = dialog.open_single_dir().show() {
                    send_to_webview(
                        webview.as_ref(),
                        json!({
                            "type": "pathSelected",
                            "target": target,
                            "path": path.to_string_lossy(),
                        }),
                    );
                }
            }
            Event::UserEvent(UserEvent::Install(request)) => {
                if installing {
                    return;
                }
                match validate_request(&request, fresh_install) {
                    Ok(()) => {
                        install_dir = PathBuf::from(&request.install_dir);
                        launch_after_install = request.launch_after;
                        installing = true;
                        send_to_webview(
                            webview.as_ref(),
                            json!({ "type": "installStarted" }),
                        );
                        start_install(
                            installer.clone(),
                            request,
                            proxy.clone(),
                        );
                    }
                    Err((field, code)) => send_to_webview(
                        webview.as_ref(),
                        json!({
                            "type": "validationError",
                            "field": field,
                            "code": code,
                        }),
                    ),
                }
            }
            Event::UserEvent(UserEvent::Progress(progress)) => {
                send_to_webview(
                    webview.as_ref(),
                    json!({ "type": "progress", "value": progress }),
                );
            }
            Event::UserEvent(UserEvent::Finished(result)) => {
                installing = false;
                match result {
                    Ok(()) if launch_after_install => {
                        match launch_main_process(&install_dir) {
                            Ok(()) => {
                                webview.take();
                                *control_flow = ControlFlow::Exit;
                            }
                            Err(error) => send_to_webview(
                                webview.as_ref(),
                                json!({
                                    "type": "launchFailed",
                                    "message": error,
                                }),
                            ),
                        }
                    }
                    Ok(()) => send_to_webview(
                        webview.as_ref(),
                        json!({ "type": "installFinished" }),
                    ),
                    Err(error) => send_to_webview(
                        webview.as_ref(),
                        json!({
                            "type": "installFailed",
                            "exitCode": error.exit_code,
                            "message": error.message,
                        }),
                    ),
                }
            }
            Event::UserEvent(UserEvent::Finish { launch }) => {
                if launch && let Err(error) = launch_main_process(&install_dir)
                {
                    send_to_webview(
                        webview.as_ref(),
                        json!({
                            "type": "launchFailed",
                            "message": error,
                        }),
                    );
                    return;
                }
                webview.take();
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}

fn parse_arguments() -> Result<Arguments, String> {
    let mut args = env::args_os().skip(1);
    let mut installer = None;
    let mut version = None;
    let mut install_dir = None;
    let mut resource_dir = None;
    let mut fresh_install = true;
    let mut language = Language::En;

    while let Some(argument) = args.next() {
        let value = args.next().ok_or_else(|| {
            format!("missing value for {}", argument.to_string_lossy())
        })?;
        match argument.to_string_lossy().as_ref() {
            "--installer" => installer = Some(PathBuf::from(value)),
            "--version" => version = Some(value.to_string_lossy().into_owned()),
            "--install-dir" => {
                install_dir = Some(value.to_string_lossy().into_owned())
            }
            "--resource-dir" => {
                resource_dir = Some(value.to_string_lossy().into_owned())
            }
            "--fresh-install" => fresh_install = value != "0",
            "--language" => {
                language = if value == "2052" {
                    Language::ZhCn
                } else {
                    Language::En
                };
            }
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
    }

    let installer =
        installer.ok_or_else(|| "missing --installer".to_string())?;
    if !installer.is_file() {
        return Err("installer executable does not exist".to_string());
    }

    Ok(Arguments {
        installer,
        bootstrap: Bootstrap {
            version: version.ok_or_else(|| "missing --version".to_string())?,
            install_dir: install_dir
                .ok_or_else(|| "missing --install-dir".to_string())?,
            resource_dir: resource_dir
                .ok_or_else(|| "missing --resource-dir".to_string())?,
            fresh_install,
            language,
            logo_data_url: format!(
                "data:image/png;base64,{}",
                BASE64.encode(LOGO)
            ),
        },
    })
}

fn validate_request(
    request: &InstallRequest,
    fresh_install: bool,
) -> Result<(), (&'static str, &'static str)> {
    let install_dir = PathBuf::from(request.install_dir.trim());
    if !install_dir.is_absolute() || request.install_dir.contains('"') {
        return Err(("install", "absolutePath"));
    }

    if !fresh_install {
        return Ok(());
    }

    let resource_dir = PathBuf::from(request.resource_dir.trim());
    if !resource_dir.is_absolute() || request.resource_dir.contains('"') {
        return Err(("resource", "absolutePath"));
    }
    if resource_dir.parent().is_none() {
        return Err(("resource", "driveRoot"));
    }

    let install_key = normalized_path_key(&install_dir);
    let resource_key = normalized_path_key(&resource_dir);
    if resource_key == install_key
        || resource_key.starts_with(&format!("{install_key}\\"))
    {
        return Err(("resource", "insideInstall"));
    }

    if fs::create_dir_all(&resource_dir).is_err() {
        return Err(("resource", "notWritable"));
    }
    let write_test = resource_dir
        .join(format!(".axolotl-write-test-{}", std::process::id()));
    if fs::write(&write_test, b"Axolotl Launcher").is_err() {
        return Err(("resource", "notWritable"));
    }
    let _ = fs::remove_file(write_test);

    Ok(())
}

fn normalized_path_key(path: &Path) -> String {
    path.to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .replace('/', "\\")
        .to_lowercase()
}

fn dialog_initial_location(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .find(|candidate| candidate.is_dir())
        .map(Path::to_path_buf)
}

fn launch_main_process(install_dir: &Path) -> Result<(), String> {
    Command::new(install_dir.join(MAIN_BINARY_NAME))
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn start_install(
    installer: PathBuf,
    request: InstallRequest,
    proxy: EventLoopProxy<UserEvent>,
) {
    thread::spawn(move || {
        let status_path = env::temp_dir().join(format!(
            "axolotl-installer-{}-{}.status",
            std::process::id(),
            thread_id_suffix()
        ));
        let _ = fs::remove_file(&status_path);

        let result = match spawn_installer(&installer, &request, &status_path) {
            Ok(mut child) => {
                wait_for_installer(&mut child, &status_path, &proxy)
            }
            Err(error) => Err(InstallFailure {
                exit_code: None,
                message: error.to_string(),
            }),
        };
        let _ = fs::remove_file(status_path);
        let _ = proxy.send_event(UserEvent::Finished(result));
    });
}

fn spawn_installer(
    installer: &Path,
    request: &InstallRequest,
    status_path: &Path,
) -> std::io::Result<std::process::Child> {
    let arguments = installer_arguments(request, status_path);
    if install_dir_requires_elevation(Path::new(request.install_dir.trim())) {
        let command = elevated_powershell_command(installer, &arguments);
        return Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &command])
            .spawn();
    }

    let mut command = Command::new(installer);
    for argument in arguments {
        command.raw_arg(argument);
    }
    command.spawn()
}

fn installer_arguments(
    request: &InstallRequest,
    status_path: &Path,
) -> Vec<String> {
    let mut arguments = vec![
        "/S".to_string(),
        nsis_value_option("INSTALL_DIR", request.install_dir.trim()),
        nsis_value_option("RESOURCE_DIR", request.resource_dir.trim()),
        nsis_value_option("STATUS_FILE", &status_path.to_string_lossy()),
    ];
    if !request.desktop_shortcut {
        arguments.push("/NO_DESKTOP_SHORTCUT".to_string());
    }
    arguments
}

fn install_dir_requires_elevation(install_dir: &Path) -> bool {
    let Some(existing_directory) =
        install_dir.ancestors().find(|candidate| candidate.is_dir())
    else {
        return true;
    };

    let write_test = existing_directory.join(format!(
        ".axolotl-install-write-test-{}-{}",
        std::process::id(),
        thread_id_suffix(),
    ));
    let can_write = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&write_test)
        .and_then(|mut file| file.write_all(b"Axolotl Launcher"))
        .is_ok();
    let removed = fs::remove_file(&write_test).is_ok();

    !(can_write && removed)
}

fn elevated_powershell_command(
    installer: &Path,
    arguments: &[String],
) -> String {
    let installer = powershell_literal(&installer.to_string_lossy());
    let arguments = powershell_literal(&arguments.join(" "));
    format!(
        "$process = Start-Process -FilePath {installer} -ArgumentList {arguments} -Verb RunAs -WindowStyle Hidden -Wait -PassThru -ErrorAction Stop; if ($null -eq $process) {{ exit 1 }}; exit $process.ExitCode"
    )
}

fn powershell_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn nsis_value_option(name: &str, value: &str) -> String {
    format!(r#"/{name}="{value}""#)
}

fn wait_for_installer(
    child: &mut std::process::Child,
    status_path: &Path,
    proxy: &EventLoopProxy<UserEvent>,
) -> Result<(), InstallFailure> {
    let mut last_progress = 0;
    loop {
        if let Ok(value) = fs::read_to_string(status_path)
            && let Ok(progress) = value.trim().parse::<u8>()
            && progress != last_progress
        {
            last_progress = progress;
            let _ = proxy.send_event(UserEvent::Progress(progress.min(99)));
        }

        match child.try_wait() {
            Ok(Some(status)) => return installer_result(status),
            Ok(None) => thread::sleep(Duration::from_millis(120)),
            Err(error) => {
                return Err(InstallFailure {
                    exit_code: None,
                    message: error.to_string(),
                });
            }
        }
    }
}

fn installer_result(status: ExitStatus) -> Result<(), InstallFailure> {
    if status.success() {
        Ok(())
    } else {
        Err(InstallFailure {
            exit_code: status.code(),
            message: "The NSIS installation core returned an error".to_string(),
        })
    }
}

fn thread_id_suffix() -> String {
    format!("{:?}", thread::current().id())
        .replace("ThreadId(", "")
        .replace(')', "")
}

fn send_to_webview(webview: Option<&WebView>, payload: serde_json::Value) {
    let Some(webview) = webview else {
        return;
    };
    if let Ok(payload) = serde_json::to_string(&payload) {
        let _ = webview.evaluate_script(&format!(
            "window.axolotlInstaller && window.axolotlInstaller.receive({payload});"
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::{
        UiCommand, dialog_initial_location, elevated_powershell_command,
        install_dir_requires_elevation, launch_main_process, nsis_value_option,
        powershell_literal,
    };
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn dialog_location_keeps_existing_directory() {
        let existing = std::env::temp_dir();

        assert_eq!(dialog_initial_location(&existing), Some(existing));
    }

    #[test]
    fn dialog_location_falls_back_to_existing_parent() {
        let existing = std::env::temp_dir();
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after Unix epoch")
            .as_nanos();
        let missing = existing
            .join(format!("axolotl-installer-ui-{unique}"))
            .join("nested");
        assert!(!missing.exists());

        assert_eq!(dialog_initial_location(&missing), Some(existing));
    }

    #[test]
    fn dialog_location_rejects_relative_path_without_existing_ancestor() {
        assert_eq!(dialog_initial_location(&PathBuf::from("")), None);
    }

    #[test]
    fn install_request_includes_launch_after_choice() {
        let command = serde_json::from_str::<UiCommand>(
            r#"{"command":"install","installDir":"C:\\Axolotl","resourceDir":"C:\\AxolotlData","desktopShortcut":true,"launchAfter":true}"#,
        )
        .expect("install request should deserialize");
        let UiCommand::Install(request) = command else {
            panic!("expected install command");
        };

        assert!(request.launch_after);
    }

    #[test]
    fn nsis_option_keeps_name_outside_quoted_value() {
        assert_eq!(
            nsis_value_option(
                "INSTALL_DIR",
                r"C:\Program Files\Axolotl Launcher",
            ),
            r#"/INSTALL_DIR="C:\Program Files\Axolotl Launcher""#,
        );
    }

    #[test]
    fn writable_install_directory_does_not_require_elevation() {
        let directory = std::env::temp_dir().join(format!(
            "axolotl-installer-ui-elevation-{}",
            std::process::id()
        ));
        fs::create_dir_all(&directory)
            .expect("test directory should be created");

        assert!(!install_dir_requires_elevation(
            &directory.join("Axolotl Launcher")
        ));

        fs::remove_dir_all(directory)
            .expect("test directory should be removed");
    }

    #[test]
    fn powershell_literals_escape_apostrophes() {
        assert_eq!(
            powershell_literal("C:\\O'Hare\\setup.exe"),
            "'C:\\O''Hare\\setup.exe'"
        );
    }

    #[test]
    fn elevated_command_preserves_the_nsis_arguments() {
        let arguments = vec![
            "/S".to_string(),
            r#"/INSTALL_DIR="C:\Program Files\Axolotl Launcher""#.to_string(),
        ];
        let command = elevated_powershell_command(
            &PathBuf::from(r"C:\Downloads\Axolotl Launcher Setup.exe"),
            &arguments,
        );

        assert!(command.contains("-Verb RunAs"));
        assert!(
            command.contains(
                r#"/INSTALL_DIR="C:\Program Files\Axolotl Launcher""#
            )
        );
    }

    #[test]
    fn launching_from_missing_install_directory_reports_error() {
        let missing = std::env::temp_dir().join(format!(
            "axolotl-installer-ui-missing-{}",
            std::process::id()
        ));
        assert!(!missing.exists());

        assert!(launch_main_process(&missing).is_err());
    }
}
