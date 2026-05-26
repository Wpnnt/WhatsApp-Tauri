// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod tray;
mod window;

use tauri::menu::{AboutMetadataBuilder, CheckMenuItemBuilder, MenuBuilder, SubmenuBuilder};
use tauri::webview::NewWindowResponse;
use tauri::{AppHandle, Manager, Runtime, WebviewWindowBuilder};
use tauri_plugin_autostart::init as autostart_init;
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_opener::OpenerExt;
use std::fs;

const MENU_AUTOSTART_TOGGLE_ID: &str = "menu_autostart_toggle";
const MENU_DEEP_LINK_TOGGLE_ID: &str = "menu_deep_link_toggle";

fn read_deep_link_enabled<R: Runtime>(app_handle: &AppHandle<R>) -> bool {
    let config_dir = app_handle.path().app_data_dir().unwrap_or_default();
    let config_file = config_dir.join("deep_link.enabled");
    if !config_file.exists() { return true; }
    fs::read_to_string(&config_file).map(|content| content.trim() == "true").unwrap_or(true)
}

fn write_deep_link_enabled<R: Runtime>(app_handle: &AppHandle<R>, enabled: bool) -> std::io::Result<()> {
    let config_dir = app_handle.path().app_data_dir().unwrap_or_default();
    fs::create_dir_all(&config_dir)?;
    fs::write(config_dir.join("deep_link.enabled"), if enabled { "true" } else { "false" })?;
    Ok(())
}

fn parse_whatsapp_link(link: &str) -> Option<String> {
    if link.starts_with("whatsapp://send") {
        if let Some(p) = link.split("phone=").nth(1) {
            let phone = p.split('&').next().unwrap_or(p).trim_matches('"').trim_matches('\'');
            return Some(format!("https://web.whatsapp.com/send?phone={}", phone));
        }
    } else if link.starts_with("whatsapp://chat") {
        if let Some(c) = link.split("code=").nth(1) {
            let code = c.split('&').next().unwrap_or(c).trim_matches('/').trim_matches('"').trim_matches('\'');
            return Some(format!("https://web.whatsapp.com/accept?code={}", code));
        }
    }
    None
}

// STRICT Internal check (Tauri v2 standard)
fn is_internal_url(url_str: &str) -> bool {
    url_str.contains("web.whatsapp.com") || 
    url_str.contains("wa.me") || 
    url_str.contains("api.whatsapp.com") || 
    url_str.contains("chat.whatsapp.com")
}

fn handle_link<R: Runtime>(app: &AppHandle<R>, url: &str) {
    if !read_deep_link_enabled(app) { return; }
    if let Some(web_url) = parse_whatsapp_link(url) {
        if let Some(win) = app.get_webview_window("main") {
            let _ = win.eval(&format!(r#"window.location.assign("{}");"#, web_url.replace('"', "\\\"")));
            let _ = win.show();
            let _ = win.set_focus();
        }
    }
}

fn build_native_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<tauri::menu::Menu<R>> {
    let autostart = app.autolaunch().is_enabled().unwrap_or(false);
    let deep_link = read_deep_link_enabled(app);
    
    let settings = SubmenuBuilder::new(app, "Settings")
        .item(&CheckMenuItemBuilder::with_id(MENU_AUTOSTART_TOGGLE_ID, "Start with System").checked(autostart).build(app)?)
        .item(&CheckMenuItemBuilder::with_id(MENU_DEEP_LINK_TOGGLE_ID, "Handle WhatsApp Links").checked(deep_link).build(app)?)
        .build()?;

    let about_meta = AboutMetadataBuilder::new()
        .name(Some("WhatsApp Tauri".to_string()))
        .version(Some(app.package_info().version.to_string()))
        .copyright(Some("Copyright © 2026 Wpnnt".to_string()))
        .authors(Some(vec!["Wpnnt".to_string()]))
        .website(Some("https://github.com/Wpnnt".to_string()))
        .comments(Some("A lightweight, secure, and native WhatsApp Desktop client built with Tauri v2.".to_string()))
        .license(Some("MIT License".to_string()))
        .icon(app.default_window_icon().cloned())
        .build();

    let help = SubmenuBuilder::new(app, "Help").about(Some(about_meta)).build()?;
    MenuBuilder::new(app).item(&settings).item(&help).build()
}

fn main() {
    tauri::Builder::default()
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
        .plugin(tauri_plugin_global_shortcut::Builder::new().with_handler(move |app, shortcut, event| {
            if event.state() == ShortcutState::Pressed && shortcut == &Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyW) {
                if let Some(w) = app.get_webview_window("main") {
                    if w.is_visible().unwrap_or(false) { let _ = w.hide(); } else { let _ = w.show(); let _ = w.set_focus(); }
                }
            }
        }).build())
        .setup(|app| {
            let handle = app.handle().clone();
            
            // Programmatic Window Creation (Definitive Tauri v2 Pattern)
            let h_nav = handle.clone();
            let h_new = handle.clone();
            
            let w = WebviewWindowBuilder::from_config(app.handle(), &app.config().app.windows[0])?
                // 1. Intercept normal frame-level navigation
                .on_navigation(move |url| {
                    let url_str = url.as_str();
                    if is_internal_url(url_str) { return true; }
                    
                    let u = url_str.to_string();
                    let h = h_nav.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = h.opener().open_url(u, None::<&str>);
                    });
                    false
                })
                // 2. Intercept new window requests (target="_blank", window.open)
                .on_new_window(move |url, _| {
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
                .build()?;

            #[cfg(any(windows, target_os = "linux"))]
            {
                if read_deep_link_enabled(&handle) { let _ = app.deep_link().register_all(); }
                else { let _ = app.deep_link().unregister("whatsapp"); }
            }

            let h_deep = handle.clone();
            app.deep_link().on_open_url(move |event| {
                for url in event.urls() { handle_link(&h_deep, url.as_str()); }
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

            let _ = w.set_menu(build_native_menu(&handle)?);
            let _ = w.eval(r#"(function(){const s=document.createElement('style');s.innerHTML='._3X_7s,._106uX,._2S6p_{display:none!important;}#app,.app-wrapper{height:100%!important;width:100%!important;position:absolute;top:0;left:0;}';document.head.appendChild(s);})();"#);
            let _ = handle.global_shortcut().register(Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyW));
            let _ = tray::setup_tray(&handle);
            
            Ok(())
        })
        .on_menu_event(|app, event| {
            if event.id().as_ref() == MENU_AUTOSTART_TOGGLE_ID {
                let state = !app.autolaunch().is_enabled().unwrap_or(false);
                let _ = if state { app.autolaunch().enable() } else { app.autolaunch().disable() };
            } else if event.id().as_ref() == MENU_DEEP_LINK_TOGGLE_ID {
                let state = !read_deep_link_enabled(app);
                let _ = write_deep_link_enabled(app, state);
                #[cfg(any(windows, target_os = "linux"))] {
                    if state { let _ = app.deep_link().register_all(); } else { let _ = app.deep_link().unregister("whatsapp"); }
                }
            }
            if let Some(w) = app.get_webview_window("main") { if let Ok(m) = build_native_menu(app) { let _ = w.set_menu(m); } }
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
