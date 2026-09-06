use tauri::menu::MenuBuilder;
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Runtime};

use crate::window;

pub fn setup_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let menu = MenuBuilder::new(app)
        .text("open", "Open")
        .separator()
        .text("quit", "Quit")
        .build()?;

    let mut builder = TrayIconBuilder::new().menu(&menu).tooltip("WhatsApp Tauri");

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }

    builder
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "open" => window::show(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button, .. } | TrayIconEvent::DoubleClick { button, .. } =
                event
            {
                if button == tauri::tray::MouseButton::Left {
                    window::show(tray.app_handle());
                }
            }
        })
        .build(app)?;

    Ok(())
}
