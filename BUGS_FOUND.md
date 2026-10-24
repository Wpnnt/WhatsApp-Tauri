# BUGS_FOUND.md — defectos preexistentes detectados durante el trabajo de rendimiento

Regla del runbook (§0.4): se registran y **no se arreglan** sin permiso explícito.
Ninguno de estos hallazgos ha sido modificado por las iteraciones de rendimiento.

## B-01 — `std::thread::sleep` dentro de `async_runtime::spawn` (bloqueo de worker)

- **Ubicación:** `whatsapp-lite/src-tauri/src/main.rs:513-524`
- **Qué pasa:** al arrancar con un deep link pendiente (`whatsapp://...`), se hace
  `tauri::async_runtime::spawn(async move { std::thread::sleep(1500ms); handle_link(...) })`.
  `thread::sleep` bloquea el hilo del reactor de tokio durante 1.5 s en lugar de cederlo.
- **Impacto:** con 8 cores, un worker de 8 queda inutilizado 1.5 s justo en el arranque; el
  efecto visible es jank en el primer render y cualquier tarea async concurrente se retrasa.
- **Severidad:** media (correctitud del runtime async).
- **Arreglo propuesto (no aplicado en Fase 1):** `tokio::time::sleep` o `spawn_blocking`.
  Es el candidato `no-blocking-sleep` de Fase 2; se aplicará allí **con medición**, no como "fix suelto".

## B-02 — El menú nativo se reconstruye dos veces por evento

- **Ubicación:** `main.rs:556-582` (caso `MENU_DOWNLOAD_FOLDER_PICK_ID` reconstruye en 565-567 y el
  bloque común de 578-582 reconstruye otra vez).
- **Impacto:** el doble trabajo incluye 3 lecturas de disco síncronas por reconstrucción; en el hilo de UI.
- **Severidad:** baja-media (latencia de UI, no corrupción).
- **Arreglo propuesto:** reconstruir una sola vez al final del handler. Candidato `menu-cache`.

## B-03 — `infer` compilado aunque el drag&drop está desactivado

- **Ubicación:** `main.rs:393` `.disable_drag_drop_handler()` frente a `cargo bloat`: `infer` 33.6 KiB.
- **Impacto:** bytes muertos en el binario.
- **Severidad:** cosmética (tamaño).
- **Nota:** entra por features de `tauri`; no se puede quitar sin tocar features (candidato 6, requiere aprobación).
