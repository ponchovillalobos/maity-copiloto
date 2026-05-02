use crate::database::manager::DatabaseManager;
use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;

pub struct AppState {
    pub db_manager: DatabaseManager,
    /// BUG #16 fix (asamblea 2026-05-02): meeting_id de la grabación activa,
    /// compartido entre TODAS las webviews. Tauri 2 aísla sessionStorage por
    /// origin (main `/` vs floating `/floating`), así que la burbuja flotante
    /// NUNCA puede leer el sessionStorage del main. Esta fuente única evita
    /// el desync. Frontend lo setea al iniciar grabación vía
    /// `set_active_meeting_id`, lo limpia al detener vía
    /// `clear_active_meeting_id`. Burbuja consulta vía `get_active_meeting_id`.
    pub active_meeting_id: Mutex<Option<String>>,

    /// v31.2 (2026-05-02): buffer en memoria de los últimos chunks transcritos.
    /// Cada elemento: (speaker_label, text). TranscriptContext alimenta vía
    /// `coach_push_transcript_chunk` cada vez que llega `transcript-update`.
    /// `coach_simple_tick` (y por extensión `coach_request_simple_tip`) lee
    /// de aquí cuando se invoca sin window explícito — esto permite a la
    /// burbuja flotante (otra webview, sin acceso a transcriptsRef del
    /// CoachContext) pedir tips sin pasar el contexto.
    /// Cap a 60 chunks (≈3-5 min de conversación). Limpieza al cerrar grabación.
    pub live_transcript: Mutex<VecDeque<(String, String)>>,

    /// v31.5 (2026-05-02): lock contra coach_simple_tick concurrente.
    /// El tick automático cada 30s (CoachContext) y el botón manual
    /// (coach_request_simple_tip) pueden invocar simultáneamente. Sin lock,
    /// el sidecar recibe 2 requests al hilo, satura CPU y duplica trabajo.
    /// AtomicBool — `compare_exchange` atómico, sin lifetime issues.
    pub coach_tick_in_flight: AtomicBool,
}
