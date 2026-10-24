# Source Audit — WhatsApp-Tauri fork (barraco002)

- **Date:** 2026-10-24
- **Base:** upstream [`Wpnnt/WhatsApp-Tauri`](https://github.com/Wpnnt/WhatsApp-Tauri) `v0.3.2` (`main` at clone time)
- **Scope:** full read of `whatsapp-lite/src-tauri` (763 LOC: `main.rs`, `tray.rs`, `window.rs`, `tauri.conf.json`), plus threat-modeling of every untrusted input: URLs navigated from remote content, `whatsapp://` deep links arriving from any local process, and download filenames suggested by the remote server.
- **Tooling:** manual review, `cargo test` (release), `cargo clippy` (release, clean), release build with MSVC 14.44 + Windows SDK 10.0.26100, runtime verification on Windows 10 22H2 (build 19045).

---

## 1. Verdict

The codebase is small, focused, and in good shape for what it is: a hardened wrapper around `https://web.whatsapp.com`. The security-critical primitives (navigation allow-list, deep-link parsing, download destinations) are structurally validated and covered by unit tests. No high-severity findings. Four issues were fixed in this fork (one cosmetic root-caused, three robustness/UX), five advisories are documented and intentionally left as-is.

## 2. Fixed in this fork (v0.3.3)

| ID | Severity | Finding | Fix |
|----|----------|---------|-----|
| F-01 | Cosmetic / UX | The window declared no `theme`, so on Windows with the system in light-app-mode the **native title bar and menu bar rendered white** against the dark WhatsApp UI. | `tauri.conf.json`: window `"theme": "Dark"` + `"backgroundColor": "#111b21"`. Tauri/tao applies `DWMWA_USE_IMMERSIVE_DARK_MODE` to the title bar and muda owner-draws the menu bar dark — independent of the system setting. Verified: `src/platform_impl/windows/dark_mode.rs` of tao 0.34.8 and `MenuTheme` propagation in tauri 2.10.3. |
| F-02 | Low (robustness) | `sanitize_filename` removed invalid characters but allowed **Windows reserved device names** (`CON.txt`, `NUL`, `COM1`…`COM9`, `LPT1`…`LPT9`), control characters, and trailing dots/spaces (which Windows silently strips, so the reserved name did not match the file actually written; device names can make the write fail outright). | Reserved stems are now prefixed (`_CON.txt`), control chars are filtered, trailing dots/spaces are stripped. Unit tests added. |
| F-03 | Low (UX) | `TrayIconBuilder` defaults to `menu_on_left_click(true)`: a left click on the tray icon popped the context menu **and** showed the window at the same time. | `.menu_on_left_click(false)` — menu is right-click only; left click shows the window, as the existing handler intended. |
| F-04 | Info | Failure to register the `Ctrl+Alt+W` global shortcut (e.g., owned by another application) was discarded with `let _ =`, leaving the user without the toggle and without a hint. | Registration errors are now reported on stderr. |

## 3. Advisories (documented, intentionally not changed)

- **A-01 — Outdated user agent.** The window pins `Chrome/120.0.0.0` (Dec 2023). WhatsApp Web may eventually restrict or warn on old UA strings. Changing it can alter WhatsApp's feature negotiation (calls, stickers, banners), so it is kept byte-identical to upstream on purpose. Revisit if WhatsApp starts blocking.
- **A-02 — `"csp": null`.** Tauri's CSP injection only applies to *local* content; this app loads a remote origin that enforces its own CSP. Not exploitable as configured, but any future local content (custom pages, bundles) should not ship under a null CSP.
- **A-03 — Reservation release race.** `release_download_reservation(path=None)` runs `retain(|p| !p.exists())` over the *global* reservation set: if two concurrent downloads share a name and one finishes (file exists on disk), the other's reservation is released early, reopening a small window where two writes can target the same path. Requires two same-name downloads racing in the same session; impact is a clobbered download, not code execution.
- **A-04 — Deep-link startup delay.** Links captured at startup are applied after a fixed 1.5 s sleep to let the webview settle. On very slow machines a cold-start link could be dropped. A "navigate when the webview signals readiness" approach would be strictly better.
- **A-05 — Settings fallback to CWD.** `app_data_dir().unwrap_or_default()` degrades to an empty path, so if the data dir cannot be resolved, `download_settings.json` / `deep_link.enabled` would be read/written relative to the current directory (e.g., `Program Files`). Practically unreachable; worth a hard error instead if it ever matters.

## 4. Positive findings (credit to upstream)

- `is_internal_url` requires HTTPS, no userinfo, no port, and an **exact** host match against four approved hosts — the classic `web.whatsapp.com.evil.com` and `user@host` bypasses are rejected, and both are covered by tests.
- `parse_whatsapp_link` rebuilds the target URL from validated components (digit-only `phone` ≤ 20 chars; `code` limited to `[_a-zA-Z0-9-]` ≤ 256; duplicate parameters rejected; no path/fragment/userinfo/port on the incoming link) instead of splicing attacker text into a URL — no injection surface.
- Downloads: in-process reservation set for same-name collisions, a real write-probe before committing to a custom folder, cancellation on probe failure with a system notification, and destinations always rebuilt from a sanitized basename (no traversal).
- Close-to-tray with a global shortcut escape hatch, single-instance focus handling, and a hidden console subsystem in release — all consistent with the tray-based UX.

## 5. Verification

- `cargo test --release`: **9 passed, 0 failed** (8 upstream + 1 added).
- `cargo clippy --release`: **0 warnings**.
- Release build: MSVC 14.44.35207, Windows SDK 10.0.26100, Tauri 2.10.3 / tao 0.34.8 / muda 0.17.2.
- Runtime: launched on Windows 10 22H2; dark title bar + dark menus confirmed by screenshot; WhatsApp session preserved (same bundle identifier and WebView2 profile as upstream installs).
