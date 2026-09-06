# Changelog

All notable changes to this project will be documented in this file.

## [0.3.0] - 2026-09-06
### Added
- **Download Manager**: Choose a custom download folder via `Settings → Select Custom Download Folder...` or let every file be saved through a native dialog with `Always Ask Where to Save Files`. Settings persist across restarts.
- **Safe Filename Handling**: Sanitizes invalid characters and auto-renames duplicates (`name (1).ext`) so downloads never silently overwrite existing files.
- **Download Notifications**: Native system notification ("Download Concluído") with file name and destination path when a download finishes.
- **Settings Submenu**: Full native menu with toggles for Start with System (autostart), Handle WhatsApp Links (deep-link on/off, persisted, re-registers/unregisters the `whatsapp://` scheme on the fly) and download options, plus the About dialog under Help.
- **Close-to-Tray**: Closing the window hides the app to the system tray instead of quitting (Quit via tray menu or Ctrl+Alt+W to toggle the window).

### Changed
- **Native Menu Reconstruction**: The menu is fully rebuilt after every menu interaction so checkbox states always stay in sync with real app state (avoids Windows checkmark bugs).

## [0.2.0] - 2026-05-25
### Added
- **Deep Link Support (`whatsapp://`)**: Fully implemented support for opening `whatsapp://send` and `whatsapp://chat` links directly from the OS, enabling seamless joining of groups and starting direct chats from the browser.
- **Robust Link Parsing**: Automatically extracts and converts `chat.whatsapp.com` web links into native internal routes when passed to the app.
- **Triple Redundancy Link Handling**: Ensures 100% reliability on Windows by capturing links via Tauri events, Command-Line Arguments (`argv`), and Cold Start verification.
- **Native External Link Interceptor**: External links (e.g., YouTube, GitHub) clicked inside the chat are now securely intercepted by the Rust backend (`on_navigation` hook) and opened in the system's default browser, preventing the app from navigating away from WhatsApp.

### Changed
- **Pure Rust Architecture**: The app is now 100% native Rust. Removed all dependency on local JavaScript/HTML frontend files.
- **Inline CSS Injection**: The UI-cleaning CSS (removing download banners) is now injected natively by the Rust backend on load, eliminating visual tearing and the need for external scripts.
- **Tray & Window Logic**: Refactored the System Tray and Window event handling for cleaner application shutdown and better performance.

### Removed
- **Frontend Dependencies**: Completely removed heavy, unused JavaScript dependencies (`@tauri-apps/api`, `@tauri-apps/plugin-opener`, `sharp`) and the empty `frontend/` folder, drastically reducing the project footprint.
- **External Scripts**: Removed `inject.js` and custom Node.js icon generation scripts (now using the official Tauri CLI).

## [0.1.1] - 2026-05-02
### Added
- Tray icon tooltip displaying "WhatsApp Tauri".
- Smart versioning script `npm run version`.
- GitHub Actions CI/CD pipeline for Windows and Linux.
- Professional "About" dialog with full metadata and icon.

