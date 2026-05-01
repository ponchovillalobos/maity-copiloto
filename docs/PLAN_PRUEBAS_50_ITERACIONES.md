# Plan de Pruebas — 50 Iteraciones Maity

## Carpeta fuente
`D:\Poncho\Videos\Edicion-Claude\output` — 27 subcarpetas-scenarios reales,
cada una con `*-valentina.mp3` (user) + `*-cliente.mp3` (interlocutor) +
`*.txt` (ground truth con líneas `[user]` / `[interlocutor]`).

## Categorías de scenarios (5 grupos)

### Atención al cliente (6)
- `atencion_aerolinea_vuelo_perdido` — queja vuelo
- `atencion_banco_fraude` — disputa cargo
- `atencion_ecommerce_devolucion` — RMA
- `atencion_gobierno_tramite` — burocracia
- `atencion_hospital_queja` — queja servicio
- `atencion_telefonia_cancelacion` — retención

### Ventas (10)
- `venta_autos_excelente` — cierre exitoso
- `venta_consultoria_cold_call` — frío
- `venta_ibm_cto_junior` — junior intentando vender enterprise
- `venta_inmobiliaria_mediocre` — sin objeciones bien manejadas
- `venta_publicidad_startup`
- `venta_seguros_senior_indeciso`
- `venta_software_junior_dificil`
- `venta_software_senior_facil`
- `reunion_venta_ibm`
- `videollamada_cierre_perdido` — perdido

### Reuniones internas (5)
- `reunion_1a1_feedback_dificil` — feedback duro
- `reunion_estrategia_ceo_cfo` — estratégica
- `reunion_onboarding_nuevo` — nuevo empleado
- `reunion_standup_equipo` — daily
- `coaching_comunicacion_ejecutiva` — coaching

### Servicio / soporte (3)
- `llamada_servicio_cliente` — soporte
- `mentoria_emprendedor_investor` — pitch a investor
- `feedback_destructivo` — comunicación tóxica

### Sample / dev (3)
- `dialogo-aeromexico` — sin .txt (skip ground truth)
- `junta_equipo_productiva` — vacía (skip)
- `sample_standup` — referencia

**Total scenarios procesables: ~25** (descartando vacíos y sin gt).

## Métricas a capturar (automático por scenario via `dev_iterations`)

| Métrica | Bueno | Aceptable | Malo |
|---|---|---|---|
| WER user | < 10% | < 20% | > 30% |
| WER interlocutor | < 10% | < 20% | > 30% |
| WER global | < 10% | < 20% | > 30% |
| Pipeline total | < 60s | < 180s | > 300s |
| Eval score (0-10) | > 7 | > 5 | < 5 |
| Sections filled (15) | ≥ 12 | ≥ 8 | < 8 |

## Workflow automático

1. Build con autorun completa.
2. Maity arranca con URL `/dev?mode=batch&autorun=1&folder=...`.
3. Auto-scan detecta los 25 scenarios.
4. Procesa secuencial — para cada uno:
   - Decode mp3 ffmpeg → PCM 16kHz (~2s)
   - Transcribe user mp3 con Parakeet (~30-60s)
   - Transcribe interlocutor mp3 (~30-60s)
   - Insert TranscriptSegments + meeting en DB
   - LLM evaluation con qwen3:1.7b (~30-60s)
   - Compute WER user / interlocutor / global vs ground truth del .txt
   - INSERT row a `dev_iterations`
5. Dashboard web (`http://localhost:3119`) refresh 3s → ves cada run aparecer.

## Tiempo estimado
- Por scenario: 90-180s
- Total 25 scenarios: **45-75 minutos**

## Análisis post-run

1. Abrir `http://localhost:3119` → ver KPIs:
   - WER promedio user/interlocutor
   - Eval score promedio
   - Tendencia (mejora/regresa por iteración)
2. Click row → modal con ref vs hyp side-by-side
3. Identificar scenarios outliers (WER > 30%) → revisar audio (¿ruido? ¿overlap?)
4. Categorías con peor WER → investigar prompt/modelo

## Iteraciones siguientes (post-50)

Si WER user > 15% promedio:
- Revisar VAD configuración (silence threshold)
- Probar Canary 1B (mejor WER español per memoria)
- Aumentar audio_ctx en parakeet config

Si eval_score < 6 promedio:
- Revisar prompt v4 (longitud, estructura)
- Probar modelo más grande (Qwen3 4B)
- Agregar few-shot examples
