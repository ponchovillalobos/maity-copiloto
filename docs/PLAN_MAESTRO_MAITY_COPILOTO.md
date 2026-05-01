# Plan Maestro — Maity Copiloto (Consejo del 2026-04-30)

## Veredicto del consejo

3 agentes consultados (Arquitecto + Security-Sentinel + Performance-Oracle).
Hallazgos: **23 fixes priorizados** en 3 dimensiones.

**Estado actual**: app fragmentada (Meetily + Maity), backend Python coexistiendo
con Rust local, UI con providers cloud, 4 pollings críticos en frontend, secrets
embebidos, fs:all permisos, lib.rs monolítico (768 LOC), unsafe sin docs.

**Objetivo**: ÚNICA app **Maity Copiloto**, 100% local, sin Python, sin marca
vieja, CPU idle <3%, zero crashes en edge cases.

---

## A — Arquitectura (8 fases secuenciales)

### Fase A1 — Rebrand Meetily → Maity Copiloto
**Archivos**: `Cargo.toml:7`, `package.json`, `tauri.conf.json`, `.github/workflows/build.yml:22`, `README.md:3`, `frontend/src/components/AnalyticsConsentSwitch.tsx:150`
**Riesgo**: Bajo · **Tiempo**: 1-2h

### Fase A2 — Eliminar dependencia de backend Python
**Archivos**: `frontend/src-tauri/src/api/api.rs:20` (kill `APP_SERVER_URL`), `backend/` (move to `archive/`), `docker-compose.yml`
**Riesgo**: Alto · **Tiempo**: 4-6h
**Verificación**: app funciona sin Python corriendo, cero `connect ECONNREFUSED localhost:5167`

### Fase A3 — Split `lib.rs` 768 LOC
**Archivos**: nuevo `init/`, `bootstrap/`, `commands/` modules
**Riesgo**: Medio · **Tiempo**: 3-4h
**Verificación**: cargo check + integrated build

### Fase A4 — Limpieza UI proveedores cloud
**Archivos**: dropdowns de provider en SettingsModal/ConfigContext (Groq/Claude/OpenAI/OpenRouter)
**Riesgo**: Medio · **Tiempo**: 2-3h

### Fase A5-A8 — Docs, Python definitivo, DB migration, QA final
**Tiempo agregado**: 5-7h

---

## B — Seguridad / Privacidad (5 fixes — security-sentinel)

| # | Archivo | Problema | Fix |
|---|---|---|---|
| B1 | `analytics/commands.rs:17` | PostHog key embebida (`phc_cohh...`) | `std::env::var("POSTHOG_API_KEY").ok()` sin fallback |
| B2 | `tauri.conf.json:69-70` | `fs:read-all` + `fs:write-all` globales | Limitar a `fs:scope-app-data` + `fs:scope-download` |
| B3 | `api/api.rs:1143` | `open_external_url` sin allowlist | Allowlist hardcoded: github.com, ollama.com |
| B4 | `lib/analytics.ts:170` | `user_id` persistente sin consent previo | Vincular a `AnalyticsConsentSwitch` antes de gen ID |
| B5 | `lib.rs:42` | Path traversal — bloquea `/`,`\` pero no canonicaliza | `Path::canonicalize()` + verificar boundary |

**Tiempo total**: ~1h. **Riesgo**: Bajo.

---

## C — Performance / Robustez (10 fixes — performance-oracle)

### Pollings críticos a eliminar (events-driven)
| # | Archivo:línea | Polling actual | Fix |
|---|---|---|---|
| C1 | `RecordingStateContext.tsx:117` | 500ms `syncWithBackend` | Listen `recording-state-updated` |
| C2 | `CoachContext.tsx:729` | 30s status check | Listen `ollama-status-changed` |
| C3 | `CoachContext.tsx:1135` | 3s metrics compute | Trigger en `transcript-update` + speaker change |
| C4 | `CoachContext.tsx:1149` | 10s nudge eval | Trigger en delta de connectionScore |
| C5 | `ConfigContext.tsx:158` | fetch Ollama on mount | User-triggered only |

**Impacto**: -25-35% CPU, -60-80% network requests.

### Robustez Rust
| # | Archivo:línea | Problema | Fix |
|---|---|---|---|
| C6 | `tray.rs:27` | `.unwrap()` icono UI | `.ok_or_else()` + handle |
| C7 | `database/manager.rs:57` | `.expect()` DB init | Propagar error con `?` |
| C8 | `recording_preferences.rs:248` | `select_recording_folder` retorna `Ok(None)` siempre | Implementar con `FileDialogBuilder` |
| C9 | `recording_manager.rs:35` | `unsafe impl Send` sin docs | Documentar SAFETY |
| C10 | `stream.rs:29,38,363` | `unsafe impl Send` sin docs | Documentar SAFETY |

**Tiempo total**: ~3-4h. **Riesgo**: Medio.

---

## Priorización ejecución (orden propuesto)

1. **B1-B5** (Security): rápido + alto impacto, sin riesgo de regresión.
2. **A1** (Rebrand): cosmético, pero limpia identidad.
3. **C6, C7, C8** (Robustez Rust): elimina crashes inmediatos.
4. **C1-C5** (Pollings): mejora masiva idle CPU.
5. **A2** (Kill Python): punto de no retorno, requiere QA exhaustivo.
6. **A3** (Split lib.rs): refactor sin cambio funcional.
7. **A4, A5, A6, A7, A8**: limpieza UI + docs + remove Python + migrations + QA.
8. **C9, C10**: documentar unsafe (post-funcional).

**Tiempo total estimado**: ~25-30h trabajo (3-4 días intenso, 1 commit por fase).

---

## Disciplina de iteración por fase

Cada fase ejecuta **el ciclo del usuario**:
1. **Backup branch** (`backup/YYYY-MM-DD-fase-XX`)
2. Fix código
3. `cargo check --release` exit 0
4. `corepack pnpm run tauri:build` exit 0 (3 artefactos)
5. Lanzar app con stdout → `/tmp/maity-app.log`
6. Re-correr batch de 25 audios scenarios (`/dev?mode=batch&autorun=1`)
7. Comparar dashboard antes/después: WER, eval_score, CPU avg, errores
8. Si métricas ≥ baseline → commit + push + IMPROVEMENT_LOG entry
9. Si métricas < baseline → revertir + investigar

**NO se sale del ciclo hasta**:
- Fase actual: zero ERROR/FATAL en logs durante batch completo
- Cumulativo (post-fase-final): 100 iter consecutivas sin error nuevo

---

## Antes de empezar — pendientes inmediatos visibles

1. **Dashboard web** ya muestra logs en vivo con resaltado ERROR/WARN/INFO en `http://localhost:3119` (refresh 3s).
2. **Onboarding bloqueado**: la app abierta está en wizard de bienvenida, Ctrl+K no funciona ahí. Necesito autorización para escribir `onboarding-status.json` o que termines el wizard manual.
3. **Audios listos**: 25 scenarios procesables en `D:\Poncho\Videos\Edicion-Claude\output`.

---

## Decisión esperada del usuario

**Opciones**:
- **A**: Aprobar plan completo en orden propuesto. Empiezo por B1-B5 (Security, ~1h, low risk).
- **B**: Cambiar prioridad — empezar por algo específico (ej. A2 kill Python).
- **C**: Rebrand primero (A1), luego batch de iteraciones para baseline, luego fix loop.
- **D**: Otra ruta.

NO toco código hasta tu OK. Dashboard listo para que veas todo.
