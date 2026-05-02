use tauri::{AppHandle, Manager, Runtime};

pub fn show<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        window.show().ok();
        window.set_focus().ok();
    }
}

pub fn restore<R: Runtime>(app: &AppHandle<R>) {
    show(app);
}
