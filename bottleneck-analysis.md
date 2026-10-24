# Análisis forense de cuellos de botella — whatsapp-tauri

Fecha: 2026-10-24T10:05Z · Commit base: `12ae4d2` · Build medida: release (`npm run build`)
Instrumentación: `perf/measure.ps1`, `perf/cdp-probe.mjs`, `cargo bloat 0.12.1`.
Todo número de este documento está en `perf-log.md` o en los archivos citados. Nada estimado se presenta como medido.

## Resumen

La app es un **shell nativo de ~32 MiB de RSS** que aloja WhatsApp Web en WebView2. Eso ordena el trabajo:
el 96 % del consumo de memoria no está en el código propio, y las tres palancas reales son (1) el peso del
binario, (2) la configuración del motor WebView2, (3) la secuencia de arranque. Nada de eso exige tocar
features ni esquemas de datos.

## Cuello 1 — Binario sobredimensionado (KPI 1, peso 20)

**Medido:** exe 12.14 MiB (12 729 344 B) tras `npm run build`; instalador NSIS 2.70 MiB.
`cargo bloat`: sección `.text` = **8.3 MiB** (69.4 % del archivo); el resto son recursos, iconos y cabeceras PE.

Reparto por crate (`perf/bloat-crates.txt`):

| Crate | .text | Comentario |
|---|---|---|
| `tauri` | 2.2 MiB | núcleo + plugins internos (menú, tray, window) |
| `std` | 1.4 MiB | irreducible sin `build-std` |
| `whatsapp_tauri` (código propio) | 1.1 MiB | **una sola función concentra 1.0 MiB** |
| `tokio` | 790.7 KiB | runtime async completo |
| `regex_automata` + `regex_syntax` + `aho_corasick` | 739.2 KiB | entran por features de `tauri-utils`/`glob`, no por código propio |
| `serde_json` | 251.6 KiB | |
| `tauri_utils` | 241.7 KiB | |
| `urlpattern` | 172.3 KiB | matcher de `remote.urls` de capabilities |
| `infer` + `rfd` + `softbuffer` | 108.4 KiB | `infer` es detección MIME de drag&drop, que la app **desactiva** |

**Causa raíz:** `whatsapp-lite/src-tauri/Cargo.toml` **no declara `[profile.release]`** (grep `\[profile` → 0 resultados).
Se compila con los defaults de Cargo: `opt-level = 3`, `lto = false`, `codegen-units = 16`, `panic = "unwind"`,
`strip = "none"`. Es decir: sin LTO entre crates, sin eliminar símbolos, con tablas de pánico completas.

**Evidencia secundaria:** la función más grande del binario es
`whatsapp_tauri::main::inner::closure$0<...>` = **1.0 MiB (12.2 % de .text)**: el closure de `setup`
contiene menú, tray, construcción de la ventana y cuatro handlers `move` inline (`main.rs:379-539`).
Un closure monstruoso impide que el optimizador descarte ramas por partes; es el candidato natural a
trocear en funciones `#[inline(never)]` nombradas.

## Cuello 2 — 813 MiB de WebView2 sin configurar (KPI 2, peso 20)

**Medido** (`baseline-01-idle.json`): RSS del árbol 845.9 MiB en mediana de 60 s.

| Componente | RSS mediana |
|---|---|
| `whatsapp-tauri.exe` (código propio) | **32.7 MiB** |
| árbol `msedgewebview2` (8 procesos) | **813.2 MiB** |

**Causa raíz:** el proyecto **nunca pasa `additional_browser_args`** al webview (grep → 0 resultados) y
`tauri.conf.json` no fija nada del motor. WebView2 arranca con los defaults de Chromium: varios renderers,
GPU process, cache de disco por defecto y heap JS sin techo práctico (medido `JSHeapUsedSize` = 148 MB del
propio WhatsApp Web, con `JSHeapTotalSize` 172 MB).

**Hipótesis de trabajo**, en orden de relación beneficio/riesgo:
1. `--renderer-process-limit=1` y `--process-per-site`: menos procesos → menos overhead por proceso.
2. `--js-flags=--max-old-space-size=...`: techo de heap; riesgo de GC más agresivo u OOM.
3. `--disk-cache-size`: cache en disco, no RSS, pero reduce I/O del perfil.
4. `--disable-features=...` (p. ej. `CalculateNativeWinOcclusion`, `BackForwardCache`): a medir, no a fe.

Cualquier cambio aquí se valida con **FPS y funcionamiento de WhatsApp** (login, envío, descargas), no solo con RSS.

## Cuello 3 — Arranque y trabajo bloqueante en `setup` (KPI 3 y 4, peso 25)

**Medido:** `t_window` = 167.2 ms (mediana), `t_cdp` = 688.6 ms, `FCP` = 549.0 ms,
`domContentLoaded` = 570.4 ms. CPU idle 0.98 % (ya cumple el ideal < 1 %).

**Causa raíz (tres hechos de código, no sospechas):**

1. **La ventana se muestra vacía y luego carga.** `tauri.conf.json` no declara `visible: false` y
   `setup` construye la ventana con `WebviewWindowBuilder::from_config` (`main.rs:392`) sin ocultarla.
   El usuario ve fondo `#111b21` durante ~380 ms.
2. **Bloqueo de un worker de tokio.** `main.rs:519`: `std::thread::sleep(Duration::from_millis(1500))`
   **dentro de `tauri::async_runtime::spawn`**. Ese `sleep` secuestra un hilo del runtime durante 1.5 s
   cuando hay un deep link pendiente en el arranque. Es un defecto, no una decisión: lo correcto es
   `tokio::time::sleep` o un `spawn_blocking`.
3. **Trabajo de disco síncrono y repetido en el camino caliente de los menús.** `build_native_menu`
   (`main.rs:288`) hace 3 lecturas de disco (`autolaunch().is_enabled()`, `read_deep_link_enabled`,
   `read_download_settings`). Se invoca: 1 vez en `setup` (`main.rs:526`) y **otra vez al final de
   *cada* evento de menú** (`main.rs:579-581`), además de una segunda reconstrucción dentro del caso
   "elegir carpeta" (`main.rs:565-567`). Un clic en "Start with System" reconstruye el menú dos veces.

**No es cuello:** los 7 plugins se registran en `main` antes de `setup`, pero su coste medido está dentro de
los 167 ms hasta tener ventana; no hay evidencia de que dominen. No se toca lo que no se ha medido.

## Candidatos priorizados (para Fase 2)

| # | Slug | Cambio | KPI objetivo | Estimación | Riesgo | Archivos |
|---|---|---|---|---|---|---|
| 1 | `release-profile` | `[profile.release]`: `lto="fat"`, `codegen-units=1`, `panic="abort"`, `strip="symbols"`, `opt-level="z"` | KPI 1 | −35/−50 % | bajo | 1 |
| 2 | `webview2-flags` | `additional_browser_args` en el builder del webview | KPI 2 | −10/−25 % del árbol | medio (puede degradar FPS) | 1 |
| 3 | `hidden-until-ready` | `visible:false` + `show()` al primer paint del webview | KPI 3 (percepción) | −300 ms percibidos | bajo | 2 |
| 4 | `no-blocking-sleep` | `std::thread::sleep` → `tokio::time::sleep` | KPI 4 (jank) | elimina 1 worker bloqueado 1.5 s | bajo | 1 |
| 5 | `menu-cache` | reconstruir el menú solo si cambió el estado; 1 vez por evento | KPI 6-sustituto | −2/3 lecturas por clic | bajo | 1 |
| 6 | `trim-unused-features` | revisar `features` de `tauri` (`unstable`) y deps con `infer` | KPI 1 | −0.1/−0.7 MiB | **alto** (puede romper API) | 1-2 |
| 7 | `split-setup-closure` | trocear `setup` en funciones nombradas `#[inline(never)]` | KPI 1 | desconocido (a medir) | bajo | 1 |

Orden de ejecución propuesto: **1 → 3 → 4 → 5 → 2 → 7** (de menor riesgo y mayor rendimiento esperado a mayor).
Los candidatos 6 queda **condicionado a aprobación** por tocar features de compilación.

## Bugs preexistentes detectados (registrados, NO arreglados)

Ver `BUGS_FOUND.md`.
