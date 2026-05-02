// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod tray;
mod window;

use tauri::menu::{AboutMetadataBuilder, CheckMenuItemBuilder, MenuBuilder, SubmenuBuilder};
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_autostart::init as autostart_init;
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

const MENU_AUTOSTART_TOGGLE_ID: &str = "menu_autostart_toggle";

fn build_native_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<tauri::menu::Menu<R>> {
    let autostart_enabled = app.autolaunch().is_enabled().unwrap_or(false);

    let settings = SubmenuBuilder::new(app, "Settings")
        .item(&CheckMenuItemBuilder::with_id(MENU_AUTOSTART_TOGGLE_ID, "Start with System")
            .checked(autostart_enabled)
            .build(app)?)
        .build()?;

    let about_meta = AboutMetadataBuilder::new()
        .name(Some("WhatsApp Tauri"))
        .version(Some(app.package_info().version.to_string()))
        .copyright(Some("© 2026 Wpnnt"))
        .authors(Some(vec!["Wpnnt".to_string()]))
        .website(Some("https://github.com/Wpnnt"))
        .comments(Some("A lightweight, secure, and native WhatsApp Desktop client built with Tauri v2."))
        .icon(app.default_window_icon().cloned())
        .build();

    let help = SubmenuBuilder::new(app, "Help").about(Some(about_meta)).build()?;

    MenuBuilder::new(app)
        .item(&settings)
        .item(&help)
        .build()
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(autostart_init(MacosLauncher::default(), None))
        .plugin(tauri_plugin_global_shortcut::Builder::new().with_handler(move |app, shortcut, event| {
            if event.state() == ShortcutState::Pressed && shortcut == &Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyW) {
                if let Some(w) = app.get_webview_window("main") {
                    if w.is_visible().unwrap_or(false) {
                        let _ = w.hide();
                    } else {
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                }
            }
        }).build())
        .setup(|app| {
            let h = app.handle().clone();
            let w = app.get_webview_window("main").ok_or("main window not found")?;
            
            let menu = build_native_menu(&h)?;
            let _ = w.set_menu(menu);
            let _ = w.eval(include_str!("inject.js"));
            let _ = h.global_shortcut().register(Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyW));
            let _ = tray::setup_tray(&h);
            
            Ok(())
        })
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            if id == MENU_AUTOSTART_TOGGLE_ID {
                let is_enabled = app.autolaunch().is_enabled().unwrap_or(false);
                let target_state = !is_enabled;

                let _ = if target_state {
                    app.autolaunch().enable()
                } else {
                    app.autolaunch().disable()
                };

                if let Some(w) = app.get_webview_window("main") {
                    if let Ok(nm) = build_native_menu(app) {
                        let _ = w.set_menu(nm);
                    }
                }
            }
        })
        .on_window_event(|window, event| {
            match event {
                tauri::WindowEvent::CloseRequested { api, .. } => {
                    let _ = window.hide();
                    api.prevent_close();
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
