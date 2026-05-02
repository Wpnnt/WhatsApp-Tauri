<img src="https://github.com/Wpnnt/WhatsApp-Tauri/blob/main/assets/whatsapp-tauri.svg" alt="Whatsapp Tauri Logo" align="left" width="65" height="65">

# WhatsApp Tauri

A native WhatsApp desktop client built with Tauri v2 and Rust.

I originally made this project because most WhatsApp desktop wrappers use Electron, which can feel unnecessarily heavy for what is basically a web app. This version uses the system WebView instead, so it starts faster and generally uses less memory.

The goal of this project is to keep things simple, lightweight, and close to the native desktop experience.

Supports Windows and Linux.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Tauri](https://img.shields.io/badge/Framework-Tauri_v2-red.svg)](https://tauri.app/)
[![Rust](https://img.shields.io/badge/Language-Rust-orange.svg)](https://www.rust-lang.org/)
[![Platform](https://img.shields.io/badge/Platform-Windows_%7C_Linux-blue.svg)](#)

---

## Features

- Uses the native system WebView instead of bundling Chromium
- Tray support for background notifications
- Global shortcut (`Ctrl + Alt + W`) to show or hide the window
- Optional auto-start on Windows
- Custom CSS tweaks to remove some unnecessary UI elements
- Linux support with `.deb` and `.AppImage` builds
- Built with Rust and Tauri

---

## Why Tauri?

Most desktop web apps today ship with a full Chromium instance through Electron. While that makes development easier, it also increases RAM usage and app size.

Tauri takes a different approach by using the operating system's native WebView. For a project like this, that makes more sense and keeps the app much lighter.

---

## Installation

### Windows

1. Go to the [Releases](https://github.com/Wpnnt/whatsapp-tauri/releases) page
2. Download the latest Windows installer
3. Run `whatsapp-tauri_x64-setup.exe`

### Linux

1. Download either the `.deb` or `.AppImage` release
2. If using AppImage, give it execution permissions:

```bash
chmod +x WhatsApp_Tauri.AppImage
```

3. Run the application

---

## Building from source

If you want to build the project yourself:

### Requirements

- [Rust & Cargo](https://www.rust-lang.org/tools/install)
- [Node.js](https://nodejs.org/)
- [NSIS](https://nsis.sourceforge.io/Download) (Windows only)
- [WebKit2Gtk](https://tauri.app/v2/guides/getting-started/prerequisites/#linux) (Linux only)

---

### Clone the repository

```bash
git clone https://github.com/Wpnnt/whatsapp-tauri.git
```

### Enter the project folder

```bash
cd whatsapp-tauri
```

### Install dependencies

```bash
npm install
```

### Run in development mode

```bash
npm run tauri dev
```

### Build production binaries

```bash
npm run tauri build
```

---

## Notes

- This is an unofficial client and is not affiliated with WhatsApp or Meta.
- The application depends on the system WebView being available and updated.
- Linux builds may look slightly different depending on the desktop environment.

---

## Contributing

Pull requests, suggestions, and bug reports are welcome.

If you find a bug or have an idea that could improve the project, feel free to open an issue.

---

## Author

Made by [Wpnnt](https://github.com/Wpnnt)

If you like the project, consider giving it a star.

---

## License

This project is licensed under the MIT License.

See the [LICENSE](LICENSE) file for more information.
