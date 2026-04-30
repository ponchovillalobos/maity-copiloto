# Form de prueba `/dev` — Importar audio para iterar

## ¿Qué hace?

Permite cargar un archivo de audio (`.mp3`, `.wav`, `.m4a`, `.ogg`, `.flac`,
`.webm`) y dispara el pipeline completo de Maity end-to-end:

1. Decodificación del audio a PCM mono 16 kHz vía FFmpeg
2. Chunking en bloques de 30 s
3. Transcripción de cada chunk con Parakeet ONNX
4. Persistencia como reunión nueva en SQLite
5. Generación automática de evaluación post-meeting con Qwen 1.7B
6. Redirección automática a `/meeting-details?id=...&source=test`

Tiempo total: ~3-5 min para audio de 5-15 min en CPU 4-8 cores.

## ¿Para qué sirve?

Iterar calidad sin tener que grabar reuniones reales. Permite probar:

- Precisión de transcripción español
- Tips relevantes y no repetidos
- Resumen post-meeting bien estructurado
- Evaluación con 15 secciones llenas y scores razonables

## Cómo usarlo

1. Abrir Maity
2. Navegar manualmente a `http://localhost:3118/dev` (la ruta NO aparece en
   navegación principal — se accede solo via URL directa)
3. Click "Click para seleccionar archivo" → elige .mp3/.wav/.m4a
4. Opcional: dar nombre a la reunión
5. Click "Procesar reunión simulada"
6. Esperar ~3-5 min mirando progress bar
7. Al terminar redirige a meeting-details con todo generado

## Implementación

### Backend Rust

**Archivo**: [`frontend/src-tauri/src/audio/import_audio.rs`](../frontend/src-tauri/src/audio/import_audio.rs)

**Comando Tauri**: `dev_import_audio_file`

```rust
#[tauri::command]
pub async fn dev_import_audio_file<R: Runtime>(
    app: AppHandle<R>,
    file_path: String,
    meeting_name: Option<String>,
) -> Result<DevImportResult, String>
```

Reusa:
- `audio::ffmpeg::find_ffmpeg_path` para localizar binario FFmpeg
- `parakeet_engine::PARAKEET_ENGINE` para transcribir cada chunk
- `database::repositories::transcript::TranscriptsRepository::save_transcript`
  para crear meeting + segments en una transacción
- `coach::evaluator::coach_evaluate_post_meeting` para evaluación auto

**Eventos emitidos**: `dev-import-progress` con stages `decoding` →
`transcribing` → `evaluating` → `done`.

### Frontend TS/TSX

**Archivo**: [`frontend/src/app/dev/page.tsx`](../frontend/src/app/dev/page.tsx)

- File picker via `@tauri-apps/plugin-dialog::open`
- Listener de `dev-import-progress` para barra animada
- Invoke vía `safeInvoke` (toast error automático si falla)
- Redirect via `next/navigation::useRouter` a meeting-details

## Restricciones

- Audio sin voz o con volumen muy bajo → `Err("Ningún chunk produjo
  transcripción")`
- Archivo > 30 min → procesa todos los chunks (limitado solo por timeout LLM
  evaluación de 5 min)
- Speaker attribution: TODOS los segmentos quedan como `interlocutor` (no hay
  separación L/R porque el audio es mono single-source)

## Riesgos / observaciones

- La ruta `/dev` queda **accesible en producción** vía URL directa. Para
  ocultarla completamente se puede agregar feature flag `MAITY_DEV_TOOLS=1` y
  condicional render. Por ahora se mantiene visible solo para usuarios que
  conozcan la ruta.
- FFmpeg debe estar disponible (búsqueda en PATH + sidecar dir + ruta del
  ejecutable). En instaladores MSI/NSIS viene empaquetado como sidecar.
