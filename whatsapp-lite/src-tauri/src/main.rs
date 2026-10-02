// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod tray;
mod window;

use serde::{Deserialize, Serialize};
use std::fs;
use tauri::menu::{AboutMetadataBuilder, CheckMenuItemBuilder, MenuBuilder, SubmenuBuilder};
use tauri::webview::NewWindowResponse;
use tauri::{AppHandle, Manager, Runtime, WebviewWindowBuilder};
use tauri_plugin_autostart::init as autostart_init;
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

const MENU_AUTOSTART_TOGGLE_ID: &str = "menu_autostart_toggle";
const MENU_DEEP_LINK_TOGGLE_ID: &str = "menu_deep_link_toggle";
const MENU_DOWNLOAD_FOLDER_PICK_ID: &str = "menu_download_folder_pick";
const MENU_DOWNLOAD_FOLDER_RESET_ID: &str = "menu_download_folder_reset";

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
struct DownloadSettings {
    download_path: Option<String>,
}

fn read_download_settings<R: Runtime>(app_handle: &AppHandle<R>) -> DownloadSettings {
    let config_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let config_file = config_dir.join("download_settings.json");
    if !config_file.exists() {
        return DownloadSettings::default();
    }
    fs::read_to_string(&config_file)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

fn write_download_settings<R: Runtime>(
    app_handle: &AppHandle<R>,
    settings: &DownloadSettings,
) -> std::io::Result<()> {
    let config_dir = app_handle.path().app_data_dir().unwrap_or_default();
    fs::create_dir_all(&config_dir)?;
    let content = serde_json::to_string_pretty(settings).unwrap_or_default();
    fs::write(config_dir.join("download_settings.json"), content)?;
    Ok(())
}

fn sanitize_filename(filename: &str) -> String {
    let invalid_chars = ['\\', '/', ':', '*', '?', '"', '<', '>', '|'];
    let mut clean = filename
        .chars()
        .filter(|c| !invalid_chars.contains(c))
        .collect::<String>();
    if clean.trim().is_empty() {
        clean = "download".to_string();
    }
    clean
}

type DownloadReservations =
    std::sync::Arc<std::sync::Mutex<std::collections::HashSet<std::path::PathBuf>>>;

fn reserve_unique_path(
    target_dir: &std::path::Path,
    file_name: &str,
    reservations: &std::sync::Mutex<std::collections::HashSet<std::path::PathBuf>>,
) -> std::path::PathBuf {
    let clean_name = sanitize_filename(file_name);
    let mut reserved = reservations
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut path = target_dir.join(&clean_name);
    if !path.exists() && !reserved.contains(&path) {
        reserved.insert(path.clone());
        return path;
    }

    let file_path = std::path::Path::new(&clean_name);
    let stem = file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e))
        .unwrap_or_default();

    let mut counter = 1;
    loop {
        let new_name = format!("{} ({}){}", stem, counter, ext);
        path = target_dir.join(&new_name);
        if !path.exists() && !reserved.contains(&path) {
            reserved.insert(path.clone());
            return path;
        }
        counter += 1;
    }
}

fn release_download_reservation(
    reservations: &std::sync::Mutex<std::collections::HashSet<std::path::PathBuf>>,
    path: Option<&std::path::Path>,
    success: bool,
) {
    let mut reserved = reservations
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    if let Some(path) = path {
        reserved.remove(path);
    } else if success {
        reserved.retain(|path| !path.exists());
    }
}

fn resolve_custom_download_path(
    target_dir: &std::path::Path,
    file_name: &str,
    reservations: &std::sync::Mutex<std::collections::HashSet<std::path::PathBuf>>,
) -> std::io::Result<std::path::PathBuf> {
    use std::io::{Error, ErrorKind};
    use std::sync::atomic::{AtomicU64, Ordering};

    static PROBE_ID: AtomicU64 = AtomicU64::new(0);

    if !fs::metadata(target_dir)?.is_dir() {
        return Err(Error::new(
            ErrorKind::NotADirectory,
            "the configured download path is not a directory",
        ));
    }

    let probe_path = loop {
        let id = PROBE_ID.fetch_add(1, Ordering::Relaxed);
        let path = target_dir.join(format!(
            ".whatsapp-tauri-write-check-{}-{id}",
            std::process::id()
        ));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => {
                drop(file);
                break path;
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    };
    fs::remove_file(probe_path)?;

    Ok(reserve_unique_path(target_dir, file_name, reservations))
}

fn read_deep_link_enabled<R: Runtime>(app_handle: &AppHandle<R>) -> bool {
    let config_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let config_file = config_dir.join("deep_link.enabled");
    if !config_file.exists() {
        return true;
    }
    fs::read_to_string(&config_file)
        .map(|content| content.trim() == "true")
        .unwrap_or(true)
}

fn write_deep_link_enabled<R: Runtime>(
    app_handle: &AppHandle<R>,
    enabled: bool,
) -> std::io::Result<()> {
    let config_dir = app_handle.path().app_data_dir().unwrap_or_default();
    fs::create_dir_all(&config_dir)?;
    fs::write(
        config_dir.join("deep_link.enabled"),
        if enabled { "true" } else { "false" },
    )?;
    Ok(())
}

fn parse_whatsapp_link(link: &str) -> Option<tauri::Url> {
    let link = tauri::Url::parse(link).ok()?;
    if link.scheme() != "whatsapp"
        || !link.username().is_empty()
        || link.password().is_some()
        || link.port().is_some()
        || !link.path().is_empty()
        || link.fragment().is_some()
    {
        return None;
    }

    let route = link.host_str()?;
    let mut phone = None;
    let mut code = None;
    for (key, value) in link.query_pairs() {
        match (route, key.as_ref()) {
            ("send", "phone") if phone.is_none() => phone = Some(value.into_owned()),
            ("chat", "code") if code.is_none() => code = Some(value.into_owned()),
            ("send", "phone") | ("chat", "code") => return None,
            _ => {}
        }
    }

    let mut target = tauri::Url::parse("https://web.whatsapp.com/").ok()?;
    match route {
        "send" => {
            let phone = phone?;
            if phone.is_empty()
                || phone.len() > 20
                || !phone.bytes().all(|byte| byte.is_ascii_digit())
            {
                return None;
            }
            target.set_path("/send");
            target.query_pairs_mut().append_pair("phone", &phone);
        }
        "chat" => {
            let code = code?;
            if code.is_empty()
                || code.len() > 256
                || !code
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
            {
                return None;
            }
            target.set_path("/accept");
            target.query_pairs_mut().append_pair("code", &code);
        }
        _ => return None,
    }
    Some(target)
}

fn is_internal_url(url_str: &str) -> bool {
    let Ok(url) = tauri::Url::parse(url_str) else {
        return false;
    };
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none()
        && matches!(
            url.host_str(),
            Some("web.whatsapp.com" | "wa.me" | "api.whatsapp.com" | "chat.whatsapp.com")
        )
}

fn handle_link<R: Runtime>(app: &AppHandle<R>, url: &str) {
    if !read_deep_link_enabled(app) {
        return;
    }
    if let Some(web_url) = parse_whatsapp_link(url) {
        if let Some(win) = app.get_webview_window("main") {
            let _ = win.navigate(web_url);
            let _ = win.show();
            let _ = win.set_focus();
        }
    }
}

fn build_native_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<tauri::menu::Menu<R>> {
    let autostart = app.autolaunch().is_enabled().unwrap_or(false);
    let deep_link = read_deep_link_enabled(app);
    let dl_settings = read_download_settings(app);

    let folder_label = match &dl_settings.download_path {
        Some(path) => format!("Folder: {}", path),
        None => "Folder: System Downloads (Default)".to_string(),
    };

    let settings = SubmenuBuilder::new(app, "Settings")
        .item(
            &CheckMenuItemBuilder::with_id(MENU_AUTOSTART_TOGGLE_ID, "Start with System")
                .checked(autostart)
                .build(app)?,
        )
        .item(
            &CheckMenuItemBuilder::with_id(MENU_DEEP_LINK_TOGGLE_ID, "Handle WhatsApp Links")
                .checked(deep_link)
                .build(app)?,
        )
        .separator()
        .item(
            &tauri::menu::MenuItemBuilder::with_id(
                MENU_DOWNLOAD_FOLDER_PICK_ID,
                "Select Custom Download Folder...",
            )
            .build(app)?,
        )
        .item(
            &tauri::menu::MenuItemBuilder::with_id(MENU_DOWNLOAD_FOLDER_RESET_ID, &folder_label)
                .build(app)?,
        )
        .build()?;

    let about_meta = AboutMetadataBuilder::new()
        .name(Some("WhatsApp Tauri".to_string()))
        .version(Some(app.package_info().version.to_string()))
        .copyright(Some("Copyright © 2026 Wpnnt".to_string()))
        .authors(Some(vec!["Wpnnt".to_string()]))
        .website(Some("https://github.com/Wpnnt".to_string()))
        .comments(Some(
            "A lightweight, secure, and native WhatsApp Desktop client built with Tauri v2."
                .to_string(),
        ))
        .license(Some("MIT License".to_string()))
        .icon(app.default_window_icon().cloned())
        .build();

    let help = SubmenuBuilder::new(app, "Help")
        .about(Some(about_meta))
        .build()?;
    MenuBuilder::new(app).item(&settings).item(&help).build()
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(autostart_init(MacosLauncher::default(), None))
        .plugin(tauri_plugin_single_instance::init(|app, argv, _| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
                for arg in argv {
                    if arg.starts_with("whatsapp://") || arg.contains("chat.whatsapp.com") {
                        handle_link(app, &arg);
                    }
                }
            }
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if event.state() == ShortcutState::Pressed
                        && shortcut == &Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyW)
                    {
                        if let Some(w) = app.get_webview_window("main") {
                            if w.is_visible().unwrap_or(false) {
                                let _ = w.hide();
                            } else {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    }
                })
                .build(),
        )
        .setup(|app| {

            let handle = app.handle().clone();

            // Programmatic Window Creation (Definitive Tauri v2 Pattern)
            let h_nav = handle.clone();
            let h_new = handle.clone();
            let h_download = handle.clone();
            let h_notif = handle.clone();
            let download_reservations: DownloadReservations =
                std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashSet::new()));
            let download_reservations_for_handler = download_reservations.clone();

            let w = WebviewWindowBuilder::from_config(app.handle(), &app.config().app.windows[0])?
                .disable_drag_drop_handler()
                .initialization_script(r#"(function(){try{const s=document.createElement('style');s.innerHTML='#app,.app-wrapper{height:100%!important;width:100%!important;position:absolute;top:0;left:0;}';document.head.appendChild(s);}catch(e){}try{if('Notification'in window&&Notification.permission!=='granted'){Notification.requestPermission().catch(function(){});}}catch(e){}})();"#)
                // 1. Intercept normal frame-level navigation
                .on_navigation(move |url: &tauri::Url| {
                    let url_str = url.as_str();
                    if is_internal_url(url_str) {
                        return true;
                    }

                    let u = url_str.to_string();
                    let h = h_nav.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = h.opener().open_url(u, None::<&str>);
                    });
                    false
                })
                // 2. Intercept new window requests (target="_blank", window.open)
                .on_new_window(move |url: tauri::Url, _| {
                    let url_str = url.as_str();
                    if is_internal_url(url_str) {
                        NewWindowResponse::Allow
                    } else {
                        let u = url_str.to_string();
                        let h = h_new.clone();
                        tauri::async_runtime::spawn(async move {
                            let _ = h.opener().open_url(u, None::<&str>);
                        });
                        NewWindowResponse::Deny
                    }
                })
                // 3. Intercept Webview downloads (Download Location, Prompt & Sanitization Options)
                .on_download(move |_webview, event| {
                    match event {
                        tauri::webview::DownloadEvent::Requested { url: _, destination } => {
                            let settings = read_download_settings(&h_download);
                            let raw_filename = destination
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("download")
                                .to_string();

                            if let Some(ref custom_path) = settings.download_path {
                                let custom_dir = std::path::PathBuf::from(custom_path);
                                match resolve_custom_download_path(
                                    &custom_dir,
                                    &raw_filename,
                                    &download_reservations_for_handler,
                                ) {
                                    Ok(path) => *destination = path,
                                    Err(error) => {
                                        let message = format!(
                                            "Download canceled. The custom folder '{}' is unavailable: {error}",
                                            custom_dir.display()
                                        );
                                        eprintln!("{message}");
                                        let _ = h_download
                                            .notification()
                                            .builder()
                                            .title("Download Failed")
                                            .body(message)
                                            .show();
                                        return false;
                                    }
                                }
                            } else if let Ok(sys_downloads) = h_download.path().download_dir() {
                                *destination = reserve_unique_path(
                                    &sys_downloads,
                                    &raw_filename,
                                    &download_reservations_for_handler,
                                );
                            }
                            true
                        }
                        tauri::webview::DownloadEvent::Finished { url: _, path, success } => {
                            release_download_reservation(
                                &download_reservations_for_handler,
                                path.as_deref(),
                                success,
                            );
                            if success {
                                if let Some(downloaded_path) = path {
                                    let file_name = downloaded_path
                                        .file_name()
                                        .and_then(|n| n.to_str())
                                        .unwrap_or("File");
                                    let path_str = downloaded_path.to_string_lossy();
                                    let _ = h_notif
                                        .notification()
                                        .builder()
                                        .title("Download Completed")
                                        .body(format!("{}: saved to {}", file_name, path_str))
                                        .show();
                                }
                            }
                            true
                        }
                        _ => true,
                    }
                })
                .build()?;

            #[cfg(debug_assertions)]
            w.open_devtools();

            #[cfg(any(windows, target_os = "linux"))]
            {
                if read_deep_link_enabled(&handle) {
                    let _ = app.deep_link().register_all();
                } else {
                    let _ = app.deep_link().unregister("whatsapp");
                }
            }

            let h_deep = handle.clone();
            app.deep_link().on_open_url(move |event| {
                for url in event.urls() {
                    handle_link(&h_deep, url.as_str());
                }
            });

            if read_deep_link_enabled(&handle) {
                if let Ok(Some(urls)) = app.deep_link().get_current() {
                    for url in urls {
                        let u = url.as_str().to_string();
                        let h = handle.clone();
                        tauri::async_runtime::spawn(async move {
                            std::thread::sleep(std::time::Duration::from_millis(1500));
                            handle_link(&h, &u);
                        });
                    }
                }
            }

            let menu = build_native_menu(&handle)?;
            let _ = app.set_menu(menu.clone());
            let _ = w.set_menu(menu);

            let _ = handle
                .global_shortcut()
                .register(Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyW));
            let _ = tray::setup_tray(&handle);

            Ok(())
        })
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            if id == MENU_AUTOSTART_TOGGLE_ID {
                let state = !app.autolaunch().is_enabled().unwrap_or(false);
                let _ = if state { app.autolaunch().enable() } else { app.autolaunch().disable() };
            } else if id == MENU_DEEP_LINK_TOGGLE_ID {
                let state = !read_deep_link_enabled(app);
                let _ = write_deep_link_enabled(app, state);
                #[cfg(any(windows, target_os = "linux"))]
                {
                    if state {
                        let _ = app.deep_link().register_all();
                    } else {
                        let _ = app.deep_link().unregister("whatsapp");
                    }
                }
            } else if id == MENU_DOWNLOAD_FOLDER_PICK_ID {
                let app_handle = app.clone();
                app.dialog().file().pick_folder(move |folder_path| {
                    if let Some(path) = folder_path {
                        if let Some(path_buf) = path.as_path() {
                            let mut settings = read_download_settings(&app_handle);
                            settings.download_path = Some(path_buf.to_string_lossy().to_string());
                            let _ = write_download_settings(&app_handle, &settings);
                            if let Some(w) = app_handle.get_webview_window("main") {
                                if let Ok(m) = build_native_menu(&app_handle) {
                                    let _ = w.set_menu(m);
                                }
                            }
                        }
                    }
                });
            } else if id == MENU_DOWNLOAD_FOLDER_RESET_ID {
                let mut settings = read_download_settings(app);
                settings.download_path = None;
                let _ = write_download_settings(app, &settings);
            }

            if let Some(w) = app.get_webview_window("main") {
                if let Ok(m) = build_native_menu(app) {
                    let _ = w.set_menu(m);
                }
            }
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod url_validation_tests {
    use super::{
        is_internal_url, parse_whatsapp_link, release_download_reservation, reserve_unique_path,
        resolve_custom_download_path,
    };
    use std::collections::HashSet;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Mutex;

    fn temporary_test_dir() -> std::path::PathBuf {
        static TEST_ID: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "whatsapp-tauri-test-{}-{}",
            std::process::id(),
            TEST_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn allows_only_approved_https_hosts() {
        for host in [
            "web.whatsapp.com",
            "wa.me",
            "api.whatsapp.com",
            "chat.whatsapp.com",
        ] {
            assert!(is_internal_url(&format!("https://{host}/")), "{host}");
        }
    }

    #[test]
    fn rejects_lookalike_hosts_and_unapproved_url_components() {
        for url in [
            "http://web.whatsapp.com/",
            "https://web.whatsapp.com.evil.example/",
            "https://evil.example/?next=web.whatsapp.com",
            "https://wa.me.evil.example/",
            "https://user@web.whatsapp.com/",
            "https://web.whatsapp.com:8443/",
        ] {
            assert!(!is_internal_url(url), "{url}");
        }
    }

    #[test]
    fn parses_supported_send_and_chat_deep_links_safely() {
        let send = parse_whatsapp_link("whatsapp://send?phone=15551234567").unwrap();
        assert_eq!(
            send.as_str(),
            "https://web.whatsapp.com/send?phone=15551234567"
        );

        let chat = parse_whatsapp_link("whatsapp://chat?code=AbC_123-xy").unwrap();
        assert_eq!(
            chat.as_str(),
            "https://web.whatsapp.com/accept?code=AbC_123-xy"
        );

        let encoded = parse_whatsapp_link("whatsapp://chat?code=AbC%5F123").unwrap();
        assert_eq!(
            encoded.as_str(),
            "https://web.whatsapp.com/accept?code=AbC_123"
        );
    }

    #[test]
    fn rejects_unsupported_or_malformed_deep_links() {
        for link in [
            "whatsapp://chatty?phone=15551234567",
            "whatsapp://send?phone=1555%22%3Balert(1)%2F%2F",
            "whatsapp://send?phone=abc",
            "whatsapp://send?phone=15551234567&phone=15557654321",
            "whatsapp://chat?code=abc%22%3Balert(1)",
            "whatsapp://chat?code=",
            "https://web.whatsapp.com/send?phone=15551234567",
        ] {
            assert!(parse_whatsapp_link(link).is_none(), "{link}");
        }
    }

    #[test]
    fn custom_download_path_requires_an_existing_directory() {
        let root = temporary_test_dir();
        let missing = root.join("missing");
        let reservations = Mutex::new(HashSet::new());
        assert!(resolve_custom_download_path(&missing, "file.txt", &reservations).is_err());

        let file = root.join("not-a-directory");
        std::fs::write(&file, "").unwrap();
        assert!(resolve_custom_download_path(&file, "file.txt", &reservations).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn custom_download_path_checks_writability_and_avoids_overwriting() {
        let dir = temporary_test_dir();
        std::fs::write(dir.join("file.txt"), "existing").unwrap();

        let reservations = Mutex::new(HashSet::new());
        let destination = resolve_custom_download_path(&dir, "file.txt", &reservations).unwrap();
        assert_eq!(destination, dir.join("file (1).txt"));
        assert!(!dir.read_dir().unwrap().any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".whatsapp-tauri-write-check-")));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn simultaneous_same_name_reservations_choose_distinct_nonexistent_paths() {
        let dir = temporary_test_dir();
        let reservations = Mutex::new(HashSet::new());

        let first = reserve_unique_path(&dir, "file.txt", &reservations);
        let second = reserve_unique_path(&dir, "file.txt", &reservations);

        assert_ne!(first, second);
        assert_eq!(first, dir.join("file.txt"));
        assert_eq!(second, dir.join("file (1).txt"));
        assert!(!first.exists());
        assert!(!second.exists());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn pathless_success_releases_completed_reservations_only() {
        let dir = temporary_test_dir();
        let completed = dir.join("completed.txt");
        let active = dir.join("active.txt");
        std::fs::write(&completed, "done").unwrap();
        let reservations = Mutex::new(HashSet::from([completed.clone(), active.clone()]));

        release_download_reservation(&reservations, None, true);

        let reserved = reservations.lock().unwrap();
        assert!(!reserved.contains(&completed));
        assert!(reserved.contains(&active));
        drop(reserved);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
