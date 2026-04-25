# Sistema de Evaluación Post-Reunión

## Descripción General

El sistema de evaluación proporciona análisis profundo de la comunicación después de cerrar una reunión. Genera un reporte visual con radar de 6 dimensiones, medidor global (gauge), gráfico de muletillas y recomendaciones priorizadas basadas en IA.

**Características principales:**
- Análisis automático tras finalizar grabación
- 100% privado — ejecutado localmente vía Ollama
- Modelo por defecto: `gemma3:4b` (~3GB, compatible con laptops 8GB RAM)
- Configurable a `gemma4:e4b` (~7GB) para máquinas potentes
- Visualización interactiva con Recharts 2.15.4

## Arquitectura

### Backend Rust

**Comando Tauri**
```rust
#[tauri::command]
async fn coach_evaluate_post_meeting(
    meeting_id: String,
    transcript_text: String,
    model: Option<String>  // Fallback a DEFAULT_MODEL si no especificado
) -> Result<MeetingEvaluation, String>
```

**Archivos del módulo:**

| Archivo | Líneas | Responsabilidad |
|---------|--------|-----------------|
| `frontend/src-tauri/src/coach/evaluator.rs` | ~280 | Orquestación: invoca LLM, parsea JSON tolerante, persiste en BD |
| `frontend/src-tauri/src/coach/prompts/evaluation_v4.rs` | ~12000 | Prompt system de 12k caracteres (versión `v4-condensado`) |
| `frontend/src-tauri/src/coach/evaluation_types.rs` | ~420 | `MeetingEvaluation` struct con 42 campos, serialización |
| `frontend/src-tauri/src/database/migrations/20260425000000_add_meeting_evaluations.sql` | ~30 | Schema: tabla `meeting_evaluations(meeting_id, evaluation_json, created_at)` |

### Frontend TypeScript

**Componente principal:**
```typescript
// frontend/src/components/MeetingEvaluation/EvaluationPanel.tsx (~380 LOC)
export function EvaluationPanel({ meetingId }: Props) {
  const [evaluation, setEvaluation] = useState<MeetingEvaluation | null>(null);
  const [loading, setLoading] = useState(false);
  
  // Render: Gauge + Radar + BarChart muletillas + Insights + Recomendaciones
}
```

**Tab en detalles de reunión:**
- Ubicado en `frontend/src/components/MeetingDetails/MeetingDetailsPanel.tsx`
- Tab "Evaluación" junto a "Transcripción", "Chat"
- Botón "Generar evaluación" si no existe evaluación previa
- Tiempo estimado: 30-60 segundos (depende del modelo y longitud)

## Flujo de Uso

1. **Usuario cierra reunión** → `stop_recording` invoca `coach_evaluate_post_meeting` fire-and-forget
2. **Evaluación se generara en background** (usuario puede navegar mientras se procesa)
3. **Usuario abre detalles de meeting** → tab "Evaluación"
4. **Si evaluación existe**: renderiza gráficas y recomendaciones
5. **Si no existe**: botón "Generar ahora" para iniciación manual

## Estructura del Resultado JSON

El LLM produce un JSON con campos agrupados:

### Identificación
- `id_evaluacion`: string UUID
- `meeting_id`: referencia a la reunión
- `timestamp`: fecha/hora de generación
- `modelo`: qué modelo LLM se usó

### Historio (Contexto)
- `duracion_minutos`: int (ej: 15)
- `num_turnos`: int (ej: 42)
- `num_hablantes`: int (1 o 2)
- `idioma_detectado`: "es" | "en" | etc.

### Análisis de Meta
- `meta_reunion`: string (qué se pretendía lograr)
- `meta_lograda`: bool
- `justificacion_meta`: string (1-2 líneas)

### Resumen Ejecutivo
- `resumen_3_lineas`: string
- `punto_fuerte_principal`: string
- `area_mejora_critica`: string

### Radiografía Cuantitativa
- `duracion_usuario_minutos`: float
- `duracion_interlocutor_minutos`: float
- `ratio_habla_usuario_pct`: 0-100
- `ratio_escucha_usuario_pct`: 0-100
- `interrupciones`: int
- `silencios_prolongados`: int

### Insights Narrativos
- `insights`: array de strings (5-7 observaciones clave)
- `patron_comunicacion`: string (ej: "Usuario domina conversación, baja empatía reflejada")
- `dinamica_poder`: string (ej: "Usuario alto control, interlocutor pasivo")

### Timeline Simplificado
- `timeline`: array de events `{ minuto: int, evento: string, speaker: "user"|"interlocutor" }`
- Máx 8 eventos clave (cambios de tono, acuerdos, tensiones)

### Dimensiones (Radar 6D)
```typescript
type Dimensiones = {
  claridad: 0-100,              // ¿Se entiende bien lo que se dice?
  proposito: 0-100,            // ¿Hay objetivos claros?
  emociones: 0-100,            // ¿Se gestiona bien el tono?
  estructura: 0-100,           // ¿Hay lógica fluida?
  persuasion: 0-100,           // ¿Se consigue influir?
  muletillas: 0-100            // ¿Baja frecuencia de muletillas?
};
```

### Análisis por Hablante
```typescript
type PorHablante = {
  user: {
    fortalezas: string[],          // 3-5 puntos
    debilidades: string[],         // 3-5 puntos
    tone_dominante: string,        // ej: "Asertivo, a veces impaciente"
    engagement: "alto"|"medio"|"bajo"
  },
  interlocutor: { /* igual */ }
};
```

### Métricas Globales
- `empatia_detectada`: 0-100 (¿reconoce perspectivas contrarias?)
- `calidad_global`: 0-100 (media ponderada de dimensiones)
- `score_confianza`: 0-100 (qué tan seguro está el modelo de sus evaluaciones)

### Recomendaciones Priorizadas
```typescript
type Recomendacion = {
  prioridad: "critica"|"importante"|"mejorable",
  categoria: "claridad"|"escucha"|"estructura"|"empatia"|"foco"|"tecnica",
  accion: string,                           // Recomendación concreta (1-2 líneas)
  aplicable_a: "usuario"|"interlocutor"|"ambos",
  tecnica_sugerida?: string,               // ej: "Técnica SPIN para discovery"
  ejemplo?: string                          // Frase modelo
};
```

### Visualizaciones Sugeridas
```typescript
type Visualizaciones = {
  gauge_tipo: "semicircular_horizontal",      // Gauge semicircular para calidad global
  radar_labels: ["Claridad", "Propósito", "Emociones", "Estructura", "Persuasión", "Muletillas"],
  muletillas_chart_type: "bar",               // BarChart mostrando top 5 muletillas
  timeline_chart_type: "scatter"              // Scatter plot timestamps vs dimensiones
};
```

## Modelos LLM Soportados

### Por Defecto: `gemma3:4b`
- **Tamaño**: ~3GB
- **Requisito RAM**: 8GB (mínimo viable)
- **Latencia**: 40-60s por reunión de 15min
- **Precisión**: 8.5/10 (buena para evaluación comunicación)

### Premium: `gemma4:e4b`
- **Tamaño**: ~7GB
- **Requisito RAM**: 16GB+
- **Latencia**: 50-90s
- **Precisión**: 9.2/10 (más detallado, análisis más fino)

### Alternativas Testeadas
- `llama2:13b` — disponible pero más lento y menos preciso para análisis fino
- `mistral:7b` — rápido pero superficial
- `neural-chat:7b` — bien para español, buen balance

**Configuración**: En `CoachModelSettings.tsx`, usuario puede cambiar modelo globalmente. La evaluación usa el modelo seleccionado (fallback a `gemma3:4b`).

## Calibración de Puntuaciones

Rangos de interpretación para `calidad_global` y dimensiones:

| Rango | Interpretación | Acción Sugerida |
|-------|----------------|----|
| 0-15 | Desastroso | Capacitación intensiva necesaria |
| 16-30 | Deficiente | Múltiples áreas críticas a mejorar |
| 31-45 | Regular | Puede mejorar con práctica dirigida |
| 46-60 | Aceptable | Comunicación funcional, margen de mejora |
| 61-75 | Competente | Buena comunicación, algunos refinamientos posibles |
| 76-90 | Muy bueno | Comunicación efectiva, solo detalles |
| 91-100 | Excelente | Maestría demostrada |

## Privacidad y Seguridad

- **Cero datos a la nube**: El modelo LLM se ejecuta localmente vía Ollama
- **Almacenamiento local**: Evaluación guardada en SQLite como JSON cifrable (future)
- **Modelo descargas opcionales**: Usuario controla qué modelos descargar
- **Sin rastreo**: No hay envío de contenido de reuniones a servidores externos

## API Tauri

### Comando Principal
```rust
#[tauri::command]
async fn coach_evaluate_post_meeting(
    meeting_id: String,
    transcript_text: String,
    model: Option<String>
) -> Result<MeetingEvaluation, String>
```

**Parámetros:**
- `meeting_id`: UUID de la reunión a evaluar
- `transcript_text`: Texto completo de la transcripción
- `model`: Nombre de modelo Ollama (opcional; fallback a `gemma3:4b`)

**Retorno:**
- Success: `MeetingEvaluation` serializado a JSON
- Error: string con descripción (ej: "Modelo no encontrado en Ollama", "LLM timeout")

**Eventos emitidos:**
- `evaluation-started` - Inicio del procesamiento
- `evaluation-progress` - Actualización (ej: LLM procesando)
- `evaluation-complete` - Resultado disponible
- `evaluation-error` - Falló (incluye detalle)

### Comandos Auxiliares
```rust
#[tauri::command]
async fn get_meeting_evaluation(meeting_id: String) -> Result<MeetingEvaluation, String>
```
Lee evaluación existente de BD.

```rust
#[tauri::command]
async fn delete_meeting_evaluation(meeting_id: String) -> Result<(), String>
```
Elimina evaluación (permite regenerar con modelo distinto).

## Persistencia en BD

**Tabla:**
```sql
CREATE TABLE meeting_evaluations (
    id TEXT PRIMARY KEY DEFAULT (uuid()),
    meeting_id TEXT NOT NULL UNIQUE,
    evaluation_json TEXT NOT NULL,          -- JSON completo serializado
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);
```

**Consultas frecuentes:**
- Obtener evaluación: `SELECT evaluation_json FROM meeting_evaluations WHERE meeting_id = ?`
- Listar evaluaciones generadas: `SELECT meeting_id, created_at FROM meeting_evaluations ORDER BY created_at DESC`
- Eliminar: `DELETE FROM meeting_evaluations WHERE meeting_id = ?`

## Testing

**Tests Rust** (`cargo test --lib coach::evaluator`):
- Parser tolerante (JSON con campos faltantes)
- Evaluación completa (todos los campos presentes)
- Evaluación mínima (solo campos obligatorios)
- Timeout LLM (manejo de errores)
- Persistencia (lectura/escritura BD)

**Test Manual:**
1. Grabar reunión de 5-10 minutos
2. Detener grabación
3. Navegar a detalles → tab "Evaluación"
4. Hacer clic "Generar evaluación"
5. Esperar 30-60s
6. Validar que radar, gauge y recomendaciones se renderizan correctamente

## Limitaciones Conocidas

- **Lenguaje**: Prompt optimizado para español e inglés. Otros idiomas pueden producir evaluaciones de menor calidad
- **Duración mínima**: Reuniones <2 minutos pueden producir evaluaciones superficiales
- **Contexto**: Solo analiza transcripción (no captura lenguaje corporal, expresiones faciales, tono de voz)
- **Modelos grandes**: Ollama puede necesitar >15GB RAM para `gemma4:e4b`; caídas posibles con múltiples instancias paralelas

## Mejoras Futuras

- Comparación período a período (evaluación anterior vs actual)
- Tendencias de mejora (radar histórico superpuesto)
- Exportación a PDF con gráficos
- Integración con calendar (vincular evaluación a evento de outlook/google)
- Fine-tuning del modelo con datos reales de reuniones
