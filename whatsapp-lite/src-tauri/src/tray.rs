use tauri::{AppHandle, Runtime};
use tauri::menu::MenuBuilder;
use tauri::tray::{TrayIconBuilder, TrayIconEvent};

use crate::window;

pub fn setup_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let menu = MenuBuilder::new(app)
        .text("open", "Open")
        .separator()
        .text("quit", "Quit")
        .build()?;

    let app_handle_menu = app.clone();
    let app_handle_tray = app.clone();
    let mut builder = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("WhatsApp Tauri");
    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }

    builder
        .on_menu_event(move |_, event: tauri::menu::MenuEvent| match event.id().as_ref() {
            "open" => window::restore(&app_handle_menu),
            "quit" => std::process::exit(0),
            _ => {}
        })
        .on_tray_icon_event(move |_, event| match event {
            TrayIconEvent::Click { button, .. } => {
                if button == tauri::tray::MouseButton::Left {
                    window::restore(&app_handle_tray)
                }
            }
            TrayIconEvent::DoubleClick { button, .. } => {
                if button == tauri::tray::MouseButton::Left {
                    window::restore(&app_handle_tray)
                }
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}
