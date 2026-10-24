# perf-log.md — registro crudo de mediciones

Metodología: `perf/measure.ps1` (5 tomas de arranque, mediana; idle = 60 s de muestreo cada 2 s tras 25 s de warmup;
FPS = 5 s de rAF vía DevTools Protocol). Todo sobre build **release**, mismo equipo (8 cores),
sin builds ni clippy en paralelo. Ver `PERF-PLAN.md` para los límites declarados.

## Baseline

| Fecha ISO | Commit | KPI | Valor | Unidad | Notas |
|---|---|---|---|---|---|
| 2026-10-24T09:46Z | 12ae4d2 | kpi1_exe | 12.14 | MiB | `cargo build --release`; 12 729 344 B |
| 2026-10-24T09:46Z | 12ae4d2 | kpi1_installer | 2.70 | MiB | NSIS x64-setup, 2 829 447 B |
| 2026-10-24T09:46Z | 12ae4d2 | assets_icons | 224 046 | B | `src-tauri/icons` (frontend empaquetado = icons) |
| 2026-10-24T09:45Z | 12ae4d2 | kpi3_t_window | 167.2 | ms | mediana de 5 (141.4–267.3) |
| 2026-10-24T09:45Z | 12ae4d2 | kpi3_t_cdp | 688.6 | ms | mediana de 5 (656–938), primer target CDP del webview |
| 2026-10-24T09:45Z | 12ae4d2 | kpi3_fcp | 549.0 | ms | `first-contentful-paint` de la navegación (1 toma) |
| 2026-10-24T09:45Z | 12ae4d2 | kpi3_dom_content | 570.4 | ms | `domContentLoadedEventEnd` |
| 2026-10-24T09:45Z | 12ae4d2 | kpi2_rss_total | 871.3 | MiB | mediana 60 s; árbol completo (app + WebView2) |
| 2026-10-24T09:47Z | 12ae4d2 | kpi2_rss_total | 845.9 | MiB | segunda tanda (SkipStartup), mediana 60 s |
| 2026-10-24T09:47Z | 12ae4d2 | kpi2_rss_main | 32.7 | MiB | **solo proceso nativo Rust** |
| 2026-10-24T09:47Z | 12ae4d2 | kpi2_rss_webview | 813.2 | MiB | árbol `msedgewebview2` (8 procesos) |
| 2026-10-24T09:47Z | 12ae4d2 | kpi2_procs | 8 | procs | mediana del árbol en idle |
| 2026-10-24T09:47Z | 12ae4d2 | kpi4_cpu_median | 0.98 | % | 60 s, normalizado por 8 cores |
| 2026-10-24T09:47Z | 12ae4d2 | kpi4_cpu_max | 2.68 | % | pico de una muestra de 2 s |
| 2026-10-24T09:47Z | 12ae4d2 | kpi5_fps_avg | 75.02 | fps | rAF 5 s en web.whatsapp.com |
| 2026-10-24T09:47Z | 12ae4d2 | kpi5_frame_p95 | 13.37 | ms | 0 frames > 50 ms |
| 2026-10-24T09:47Z | 12ae4d2 | kpi5_js_heap | 147.99 | MB | `JSHeapUsedSize` del webview |
| 2026-10-24T09:46Z | 12ae4d2 | kpi8_build_release | 170.3 | s | `npm run build` (tauri build, deps release recompiladas) |
| 2026-10-24T09:44Z | 12ae4d2 | kpi8_cargo_release | 10.1 | s | `cargo build --release` tras `cargo clean -p whatsapp-tauri` |

### Gate base (antes de tocar nada)

| Fecha ISO | Commit | Gate | Resultado |
|---|---|---|---|
| 2026-10-24T09:30Z | 12ae4d2 | `cargo fmt --check` | verde |
| 2026-10-24T09:31Z | 12ae4d2 | `cargo clippy --all-targets -- -D warnings` | verde |
| 2026-10-24T09:31Z | 12ae4d2 | `cargo test --all-targets` | 9/9 ok |
| 2026-10-24T09:26Z | 12ae4d2 | `npm run build` | verde (NSIS generado) |

### Nota de atribución (importante para el scoring)

El 96,1 % del RSS en idle (813 de 846 MiB) pertenece al árbol **WebView2 + WhatsApp Web**, no al binario.
El proceso nativo ocupa 32,7 MiB. Por tanto:

- El KPI 2 se evalúa sobre `kpi2_rss_total` (lo que siente el usuario) **y** sobre `kpi2_rss_main`
  (lo que controla el código). Ambas filas se registran en cada iteración.
- Ninguna afirmación de "RAM mejorada" vale si solo mueve `rss_main` un 3 % del total sin tocar el árbol.

---

## Iteración 1 — rama `perf/release-profile`

Una sola idea por iteración: **ajustar el perfil de compilación de release**. Se probaron dos variantes
del mismo cambio; la primera se rechazó por regresión de arranque y se descartó sin piedad (§0 del runbook).

### Variante A — `opt-level = "z"` (RECHAZADA → rollback)

Build: `lto="fat"`, `codegen-units=1`, `panic="abort"`, `strip="symbols"`, `opt-level="z"`.

| Fecha ISO | KPI | Antes (12ae4d2) | Después (variante A) | Δ | Veredicto |
|---|---|---|---|---|---|
| 2026-10-24T10:01Z | kpi1_exe | 12.14 MiB | 4.22 MiB (4 427 776 B) | −65.2 % | mejora |
| 2026-10-24T10:01Z | kpi1_installer | 2.70 MiB | 1.56 MiB | −42.2 % | mejora |
| 2026-10-24T10:02Z | kpi3_t_window | 167.2 ms | **274.1 ms** | **+64.0 %** | **regresión** |
| 2026-10-24T10:02Z | kpi3_t_cdp | 688.6 ms | 784.8 ms | +13.9 % | regresión |
| 2026-10-24T10:02Z | kpi5_fps_avg | 75.02 fps | 75.05 fps | +0.0 % | neutro |
| 2026-10-24T10:02Z | kpi2_rss_total | 845.9 MiB | 883.4 MiB | +4.4 % | neutro |
| 2026-10-24T10:01Z | kpi8_build_release | 170.3 s | 313.8 s | +84.3 % | coste |

**Motivo del rollback:** `t_window` +64 % (KPI crítico, §6: >10 % ⇒ descartar). `opt-level="z"` prioriza tamaño
sobre velocidad y el coste de arranque lo paga el usuario. La variante se sustituyó, no se "matizó".

### Variante B — `opt-level = 3` (ACEPTADA)

Mismo bloque cambiando solo `opt-level` a `3`.

| Fecha ISO | KPI | Antes (12ae4d2) | Después (B) | Δ | Veredicto |
|---|---|---|---|---|---|
| 2026-10-24T10:15Z | kpi1_exe | 12 729 344 B (12.14 MiB) | 6 555 648 B (6.25 MiB) | **−48.5 %** | mejora |
| 2026-10-24T10:15Z | kpi1_installer | 2 829 447 B (2.70 MiB) | 2 096 343 B (2.00 MiB) | **−25.9 %** | mejora |
| 2026-10-24T10:20Z | kpi3_t_window | 167.2 ms | **175.2 ms** (10 tomas: 139.9–222.2) | +4.8 % | dentro de incertidumbre |
| 2026-10-24T10:20Z | kpi3_t_cdp | 688.6 ms | 691.6 ms (657–913) | +0.4 % | neutro |
| 2026-10-24T10:31Z | kpi2_rss_total | 845.9 MiB | 844.7 MiB | −0.1 % | neutro |
| 2026-10-24T10:31Z | kpi2_rss_main | 32.7 MiB | 31.2 MiB | −4.6 % | neutro |
| 2026-10-24T10:31Z | kpi2_rss_webview | 813.2 MiB | 813.5 MiB | +0.0 % | neutro |
| 2026-10-24T10:31Z | kpi4_cpu_median | 0.98 % | 1.10 % | +0.12 pp | dentro de incertidumbre |
| 2026-10-24T10:31Z | kpi4_cpu_max | 2.68 % | 5.86 % | pico de sincronización | no atribuible al binario |
| 2026-10-24T10:31Z | kpi5_fps_avg | 75.02 fps | 74.97 fps | −0.1 % | neutro |
| 2026-10-24T10:31Z | kpi5_frame_p95 | 13.37 ms | 13.37 ms | 0.0 % | neutro |
| 2026-10-24T10:15Z | kpi3_fcp | 549.0 ms | 724.9 ms | +32 % | **no concluyente** (ver nota) |
| 2026-10-24T10:15Z | kpi8_build_release | 170.3 s | 366 s | +115 % | coste de CI, no de usuario |

**Nota metodológica sobre `t_window` (L-0008 del workspace).** La primera tanda (5 tomas) daba 201.5 ms y
parecía una regresión de +20 %; la dispersión intra-tanda (151–284 ms, CV ≈ 25 %) era mayor que la diferencia,
así que el veredicto correcto era "no establecido", no "regresión". Se repitió con **10 tomas** → mediana
175.2 ms, es decir +4.8 % sobre el baseline y por debajo del umbral del 10 %. La primera toma de cada tanda
es siempre un valor atípico alto (222–284 ms) por arranque en frío tras matar todos los procesos.

**Nota sobre `kpi3_fcp`.** `first-contentful-paint` lo dicta la red y el servidor de WhatsApp Web, no el binario:
las tres tandas dieron 527 / 549 / 725 ms con el mismo código de aplicación subyacente. No se usa para
aceptar ni rechazar ningún cambio; solo se registra.

### Gates de la iteración 1 (variante B)

| Fecha ISO | Commit | Gate | Resultado |
|---|---|---|---|
| 2026-10-24T10:34Z | `perf/release-profile` (sin commitear aún) | `cargo fmt --check` | verde |
| 2026-10-24T10:35Z | idem | `cargo clippy --all-targets -- -D warnings` | verde |
| 2026-10-24T10:35Z | idem | `cargo test --all-targets` | 9/9 ok |
| 2026-10-24T10:15Z | idem | `npm run build` (tauri build) | verde (NSIS 2 096 343 B) |

**Nota sobre `kpi4_cpu_max`.** El pico (5.86 %) cae en las primeras muestras y decae monótonamente hasta
0.70 %; es la resincronización de WhatsApp Web al abrir la app, presente también en el baseline (2.68 %).
La mediana de 60 s es la cifra que se compara.

