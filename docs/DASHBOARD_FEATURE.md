# Dashboard de Control Milimétrico — `/dashboard`

## ¿Qué es?

Vista única de observabilidad local para Maity. Muestra **en tiempo real**
todo lo que pasa en la app: CPU/RAM, modelos activos, latencias por etapa del
pipeline, WER de transcripción, scores de evaluación, eventos Tauri en vivo y
una matriz auditable de cada botón crítico de la UI.

Diseñado para iterar 50× con control total: cada audio que se carga en `/dev`
queda persistido como una fila en `dev_iterations` con todas sus métricas, y se
visualiza con tendencia histórica.

## Acceso

- URL directa: `localhost:3118/dashboard`
- Command palette: `Ctrl+K → "dashboard"`

NO aparece en navegación principal. Es ruta dev/admin.

## Layout

```
┌─ Header (back + título + link a /dev)
├─ Row 1: SystemPanel | ModelPanel | SummaryKPIs
├─ Row 2: PipelineTimingChart (stacked bars por iteración)
├─ Row 3: QualityTrendsChart (line chart WER% + score)
├─ Row 4: IterationsTable (todas las iteraciones, click → modal)
├─ Row 5: ButtonsMatrix (26 botones, status editable)
└─ Row 6: LiveEventsStream | PromptsSummary
```

## Métricas en tiempo real

### CPU/RAM (1Hz)
Backend: `frontend/src-tauri/src/observability/system_monitor.rs` corre como
task tokio infinito que muestrea `sysinfo` cada 1s y emite evento
`system-metrics`. Frontend (`useSystemMetrics`) bufferea los últimos 120 puntos
(2 min) y los grafica en `SystemPanel` con `recharts`.

Payload del evento:
```json
{
  "ts": 1730000000000,
  "cpu_pct": 12.4,
  "ram_used_mb": 8192,
  "ram_total_mb": 16384,
  "process_cpu_pct": 4.7,
  "process_ram_mb": 480,
  "thread_count": 24
}
```

### Estado de modelos
`ModelPanel` polea cada 10s `builtin_ai_is_model_ready` para `qwen3:0.6b` y
`qwen3:1.7b` + `parakeet_is_model_loaded`. Verde = listo, gris = no cargado.

## Iteraciones persistidas

### Tabla `dev_iterations`
Migración: `migrations/20260430000000_create_dashboard_tables.sql`.

Cada vez que se ejecuta `dev_import_audio_file` o `dev_import_two_audios`,
se inserta una fila con:
- Audios (paths)
- Layout (stereo / mono / two-files)
- Timings: decode_ms, transcribe_user_ms, transcribe_interlocutor_ms, evaluation_ms, total_pipeline_ms
- WER global / user / interlocutor (si hubo ground truth)
- evaluation_score (0-10) + sections_filled (0-15)
- Hipótesis Maity completa + ground truth references
- Modelos usados + prompt_version

### Comandos Tauri
- `dashboard_list_iterations(limit)` → últimas N filas
- `dashboard_get_iteration_detail(iteration_id)` → detalle full con refs/hyps
- `dashboard_get_summary()` → KPIs agregados (totals, WER avg 30d, etc.)

## Auditoría de botones

### Tabla `button_audit`
26 botones inventariados (recording, sidebar, command palette, coach,
evaluation, summary, dev). Cada uno tiene:
- `id` único (ej. `rec.start`, `eval.generate`)
- `status` ∈ {ok, broken, warn, untested, deprecated}
- `notes` (free text)
- `last_checked_at` (timestamp última edición)

### Workflow
1. Al startup, `seed_buttons_if_needed` inserta los 26 botones con status
   `untested` (idempotente — no toca los ya existentes).
2. Usuario abre `/dashboard` → `ButtonsMatrix` los lista filtrables por
   categoría o status.
3. Click en dropdown de status → invoca `dashboard_update_button_status`
   con timestamp automático.
4. KPI `broken_button_count` y `untested_button_count` aparecen en
   `SummaryKPIs` para vista rápida.

### Comandos Tauri
- `dashboard_seed_buttons` → idempotent seed
- `dashboard_list_buttons` → lista filtrable
- `dashboard_update_button_status(button_id, status, notes, iteration_id?)`

## Eventos en vivo

`LiveEventsStream` escucha 8 nombres de eventos Tauri y mantiene un buffer de
los últimos 100. Útil para debug en tiempo real:
- `coach-tip-update`, `coach-tips-clear`
- `meeting-metrics`
- `dev-import-progress`
- `transcript-update`
- `recording-started`, `recording-stop-complete`
- `system-metrics`

## Verificación

1. `cargo check --release` exit 0.
2. `corepack pnpm run tauri:build` exit 0 + 3 artefactos.
3. Launch app fresca → migración corre automáticamente, tablas creadas.
4. Navegar a `/dashboard`:
   - SystemPanel actualizando cada 1s
   - ModelPanel con 3 modelos check
   - SummaryKPIs con todos en `–` o 0 (DB vacía)
   - ButtonsMatrix con 26 filas en `untested`
   - IterationsTable vacía
5. Cargar audio en `/dev` modo single → procesar → volver a `/dashboard`:
   - IterationsTable +1 fila con timings
   - QualityTrendsChart muestra el primer punto
6. Cargar audio en `/dev` modo QA con ground truth → procesar:
   - IterationsTable nueva fila con WER user/interlocutor poblado
   - QualityTrendsChart pinta WER% en color
7. Cambiar status de un botón → reload página → status persiste.
8. `Ctrl+K → "dash"` → navega via CommandPalette.

## Out of scope

- Dashboard remoto (solo local SQLite + Tauri events)
- IA generadora de widgets (estilo infinite-monitor)
- Comparación side-by-side de runs (botón "diff iteración A vs B")
- Export del dashboard a PDF/HTML
- Alertas con umbrales (notif si WER > X)
