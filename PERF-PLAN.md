# PERF-PLAN — configuración verificada (§1) y KPIs adaptados (§2.1)

Fecha de reconocimiento: 2026-10-24 (America/Lima)
Repo auditado: `D:\dev\whatsapp-tauri` (rama `main`, HEAD `12ae4d2 chore(release): v0.3.3`)
Sin tocar código de la app: esta fase solo añade `perf/` (instrumentación) en la rama `perf/baseline`.

## §1 Contexto del proyecto — rellenado con evidencia, sin inventar

| Campo | Valor real | Evidencia |
|---|---|---|
| Ruta absoluta | `D:\dev\whatsapp-tauri` (proyecto en `whatsapp-lite/`) | `git rev-parse`, listado de árbol |
| Versión Tauri | **v2** | `src-tauri/Cargo.toml`: `tauri = "2"`, plugins `*-plugin-*` v2; `tauri.conf.json` con `$schema ... config/2` |
| Frontend framework | **ninguno** (vanilla: la app carga `https://web.whatsapp.com` remoto) | `tauri.conf.json`: `devUrl = https://web.whatsapp.com`, `frontendDist = "icons"`; `package.json` sin dependencias de UI |
| Bundler | **ninguno** | no hay Vite/Webpack/esbuild; `npm run build` = `tauri build` |
| Package manager | **npm** | `package-lock.json` presente, `npm ci` usado por CI |
| SO objetivo | **Windows 10 Pro 10.0.19045** (host y target principal) | `Get-CimInstance Win32_OperatingSystem` |
| Comando dev | `npm run dev` (`tauri dev`) | `package.json` |
| Comando build release | `npm run build` (`tauri build`) | `package.json`; CI usa `tauri-apps/tauri-action@v0` |
| Comando tests | **nadie declaró script npm**; el gate real es Rust: `cargo test --all-targets` (8 tests en `main.rs`) | `.github/workflows/release.yml` líneas 34-44 |
| CI | **Sí**: `Release` → `quality-gate` en ubuntu-22.04 con `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets`; release multiplataforma por tag `v*` | `.github/workflows/release.yml` |
| Features críticas | sesión persistente de WhatsApp Web, bandeja + menú nativo, atajos globales, deep links `whatsapp://`, descargas con saneado de nombre y carpeta configurable, autostart | `src/main.rs`, `src/tray.rs`, `capabilities/default.json` |
| Plugins Tauri | `dialog`, `notification`, `opener`, `autostart`, `single-instance`, `deep-link`, `global-shortcut` (+ `tray-icon`, feature `unstable`) | `Cargo.toml`, `main.rs` líneas 344-378 |
| Deps Rust pesadas | `tauri` (con `tray-icon` + `unstable`), `serde`/`serde_json`, 7 plugins; `webview2-com`/`windows` transitivas | `Cargo.lock` (143 KB), `cargo bloat` en Fase 1 |
| Deps JS pesadas | ninguna en runtime; solo `@tauri-apps/cli` (dev) | `package.json` |
| Presupuesto de tiempo | **sin límite declarado** → se adopta 8 h de trabajo efectivo y parada por §7 | este documento |
| Rama base | `main` | `git branch --show-current` |
| Remotos | `origin` = barraco002/WhatsApp-Tauri, `upstream` = Wpnnt/WhatsApp-Tauri | `git remote -v` |

### Entorno de compilación (hallazgo bloqueante de Fase 0)

El host **no tiene Visual Studio**: MSVC y el Windows SDK están reubicados en `D:\dev` y no están en el PATH.
El repo lo sabe a medias: `whatsapp-lite/src-tauri/.cargo/config.toml` (untracked) fija solo el linker.
Sin `LIB`/`INCLUDE`/`RC`, los gates fallan con `LNK1181: cannot open input file 'OleAut32.lib'` y
`RC.EXE not found`. Se aporta `perf/env-msvc.ps1` (dot-source) que reconstruye el entorno:

- PATH: `D:\dev\cargo\bin`, `MSVC\14.44.35207\bin\Hostx64\x64`, `Windows Kits\10\bin\10.0.26100.0\x64`
- LIB: `MSVC\lib\x64`, `Kits\10\Lib\10.0.26100.0\{um,ucrt}\x64`
- INCLUDE: MSVC + `ucrt`, `um`, `shared`, `winrt`
- `RC` apuntando a `rc.exe` del SDK (lo exige `tauri-winres`)

Toolchain: `rustc 1.99.1`, `node v24.20.0`, `npm 11.19.0`, `git 2.55.0`.

## §2.1 KPIs — tabla adaptada a la realidad del proyecto

Dos KPIs del enunciado **no son medibles tal cual** en un wrapper de web remoto. Se sustituyen por
equivalentes con la misma intención (no se eliminan, no se dejan en `?`):

| # | KPI | Herramienta | Baseline | Mínimo | Ideal | Peso |
|---|---|---|---|---|---|---|
| 1 | Tamaño del `.exe` release | `Get-Item .Length` | *(medido en esta fase)* | -20% | -40% | 20 |
| 2 | RAM RSS en idle (5 min, árbol completo) | `perf/measure.ps1` (WorkingSet64 sumado sobre app + WebView2) | *(medido)* | -30% | -50% | 20 |
| 3 | Arranque en frío: `t_window` y `t_fcp` | `perf/measure.ps1` ×5, mediana | *(medido)* | -25% | -50% | 15 |
| 4 | CPU en idle (%, 60 s) | `perf/measure.ps1` (delta `CPU` / tiempo / cores) | *(medido)* | < 2% | < 1% | 10 |
| 5 | **FPS del webview** en pantalla de arranque | `perf/cdp-probe.mjs` (rAF 5 s vía CDP) | *(medido)* | ≥ 55 | 60 sin frames > 50 ms | 10 |
| 6 | **Latencia de IPC nativo** | `invoke` no existe: no hay `#[tauri::command]` ni `invoke_handler` en la app | **N/A** → se sustituye por *latencia de operaciones nativas* (toggle de menú → efecto) y *entradas de menú reconstruidas por evento* (contador) | -20% | -40% | 10 |
| 7 | **Peso del frontend empaquetado** | no hay bundle JS: `frontendDist = icons` | **N/A** → se sustituye por *bytes de assets empaquetados* y *chunks de inicialización* (script inline, `initialization_script`) | -25% | -45% | 10 |
| 8 | Tiempo de build release | `Measure-Command { npm run build }` | *(medido)* | -15% | -30% | 5 |

Peso total = 100. KPI 6 y 7 conservan su peso pero miden el sustituto declarado; cualquier cambio
de KPI se anota en `perf-log.md`.

### Metodología y sus límites (declarados, no escondidos)

1. Tres tomas mínimo, mediana. `measure.ps1` hace 5 por defecto.
2. Misma build release y mismo equipo en todas las tomas; sin `clippy`/`cargo` corriendo en paralelo.
3. **Límite**: el "arranque en frío" no puede vaciar la caché de WebView2 sin destruir la sesión de
   WhatsApp del usuario (§3.1.3 persistencia intacta). Se fuerza el cierre del árbol de procesos
   WebView2 antes de cada toma (proceso frío, caché HTTP caliente). Se declara así en el reporte.
4. **Límite**: la pantalla medida es la de arranque (QR / carga), porque la app exige sesión de
   WhatsApp para mostrar chats. El coste de renderizar conversaciones pertenece a web.whatsapp.com,
   no al binario: fuera de alcance.
5. Medición de FPS y de paint vía DevTools Protocol con
   `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222`; es la misma build, solo
   añade un puerto de depuración durante la medición.
