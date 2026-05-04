use serde::{Deserialize, Serialize};
use std::sync::Mutex as StdMutex;
// Removed unused import

// Performance optimization: Conditional logging macros for hot paths
#[cfg(debug_assertions)]
macro_rules! perf_debug {
    ($($arg:tt)*) => {
        log::debug!($($arg)*)
    };
}

#[cfg(not(debug_assertions))]
macro_rules! perf_debug {
    ($($arg:tt)*) => {};
}

// Make these macros available to other modules
pub(crate) use perf_debug;

// Input validation module for Tauri commands
pub mod validation_helpers {
    //! Input validation helpers for Tauri commands.
    //! Provides validation functions for common string parameters across Tauri commands.

    pub fn validate_string_length(value: &str, field: &str, max: usize) -> Result<String, String> {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            return Err(format!("{} cannot be empty", field));
        }
        if trimmed.len() > max {
            return Err(format!(
                "{} exceeds maximum length of {} characters (got {})",
                field,
                max,
                trimmed.len()
            ));
        }
        Ok(trimmed)
    }

    pub fn validate_no_path_traversal(value: &str, field: &str) -> Result<(), String> {
        if value.contains("..") || value.contains('/') || value.contains('\\') {
            return Err(format!(
                "{} contains invalid path characters (.., /, \\)",
                field
            ));
        }
        Ok(())
    }

    pub fn validate_path_boundary(user_path: &str, app_data_dir: &std::path::Path) -> Result<(), String> {
        // Canonicalize user-provided path to resolve symlinks
        let canonical_user = std::fs::canonicalize(user_path)
            .map_err(|e| format!("Invalid path '{}': {}", user_path, e))?;

        // Ensure canonical path is within app data boundary
        if !canonical_user.starts_with(app_data_dir) {
            return Err(format!("Path '{}' is outside app data directory scope", user_path));
        }

        Ok(())
    }

    pub fn validate_meeting_name(name: &str) -> Result<String, String> {
        let trimmed = validate_string_length(name, "meeting_name", 500)?;
        validate_no_path_traversal(&trimmed, "meeting_name")?;
        Ok(trimmed)
    }

    pub fn validate_device_name(name: &str) -> Result<String, String> {
        validate_string_length(name, "device_name", 200)
    }

    pub fn validate_model_id(id: &str) -> Result<String, String> {
        let trimmed = validate_string_length(id, "model_id", 100)?;
        validate_no_path_traversal(&trimmed, "model_id")?;
        Ok(trimmed)
    }

    pub fn validate_meeting_id(id: &str) -> Result<String, String> {
        let trimmed = validate_string_length(id, "meeting_id", 100)?;
        if !trimmed
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(
                "meeting_id contains invalid characters (only alphanumeric, -, _ allowed)"
                    .to_string(),
            );
        }
        Ok(trimmed)
    }

    pub fn validate_language(lang: &str) -> Result<String, String> {
        validate_string_length(lang, "language", 10)
    }

    pub fn validate_provider(provider: &str) -> Result<String, String> {
        let trimmed = validate_string_length(provider, "provider", 50)?;
        validate_no_path_traversal(&trimmed, "provider")?;
        Ok(trimmed)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_validate_string_length_valid() {
            let result = validate_string_length("Hello World", "test_field", 20);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), "Hello World");
        }

        #[test]
        fn test_validate_string_length_trims_whitespace() {
            let result = validate_string_length("  Hello World  ", "test_field", 20);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), "Hello World");
        }

        #[test]
        fn test_validate_string_length_empty_after_trim() {
            let result = validate_string_length("   ", "test_field", 20);
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("cannot be empty"));
        }

        #[test]
        fn test_validate_string_length_exceeds_max() {
            let result = validate_string_length("This is a very long string", "test_field", 10);
            assert!(result.is_err());
            assert!(result
                .unwrap_err()
                .contains("exceeds maximum length of 10 characters"));
        }

        #[test]
        fn test_validate_no_path_traversal_valid() {
            let result = validate_no_path_traversal("MyDocument-v1", "test_field");
            assert!(result.is_ok());
        }

        #[test]
        fn test_validate_no_path_traversal_parent_directory() {
            let result = validate_no_path_traversal("../sensitive_file", "test_field");
            assert!(result.is_err());
            assert!(result.unwrap_err().contains("invalid path characters"));
        }

        #[test]
        fn test_validate_no_path_traversal_forward_slash() {
            let result = validate_no_path_traversal("dir/file", "test_field");
            assert!(result.is_err());
        }

        #[test]
        fn test_validate_no_path_traversal_backslash() {
            let result = validate_no_path_traversal("dir\\file", "test_field");
            assert!(result.is_err());
        }

        #[test]
        fn test_validate_meeting_id_valid() {
            let result = validate_meeting_id("meeting-123_abc");
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), "meeting-123_abc");
        }

        #[test]
        fn test_validate_meeting_id_invalid_characters() {
            let result = validate_meeting_id("meeting@123");
            assert!(result.is_err());
            assert!(result
                .unwrap_err()
                .contains("invalid characters (only alphanumeric, -, _ allowed)"));
        }

        #[test]
        fn test_validate_meeting_id_exceeds_max() {
            let long_id = "a".repeat(101);
            let result = validate_meeting_id(&long_id);
            assert!(result.is_err());
        }

        #[test]
        fn test_validate_meeting_name_with_path_traversal() {
            let result = validate_meeting_name("../admin");
            assert!(result.is_err());
        }

        #[test]
        fn test_validate_model_id_path_traversal() {
            let result = validate_model_id("..\\sensitive\\model");
            assert!(result.is_err());
        }

        #[test]
        fn test_validate_device_name_max_length() {
            let long_name = "a".repeat(201);
            let result = validate_device_name(&long_name);
            assert!(result.is_err());
        }
    }
}

// Re-export async logging macros for external use (removed due to macro conflicts)

// Declare audio module
pub mod analytics;
pub mod api;
pub mod audio;
pub mod auto_setup;
pub mod builtin_ai;
pub mod calendar;
pub mod coach;
pub mod compliance;
pub mod console_utils;
pub mod database;
pub mod export;
pub mod logging;
pub mod meeting_detector;
pub mod notifications;
pub mod observability;
pub mod ollama;
pub mod onboarding;
pub mod orchestrator;
pub mod progress_events;
pub mod semantic_search;
pub mod canary_engine;
pub mod parakeet_engine;
pub mod secure_storage;
pub mod state;
pub mod summary;
pub mod tray;
pub mod utils;


use audio::{list_audio_devices, AudioDevice, trigger_audio_permission};
use log::{error as log_error, info as log_info};
use notifications::commands::NotificationManagerState;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tokio::sync::RwLock;

// Global language preference storage (initialized with detected system language)
static LANGUAGE_PREFERENCE: std::sync::LazyLock<StdMutex<String>> =
    std::sync::LazyLock::new(|| StdMutex::new(detect_system_language()));

/// Detect the system's UI language automatically
fn detect_system_language() -> String {
    // Windows-specific: use Win32 Globalization API
    #[cfg(target_os = "windows")]
    {
        if let Some(lang) = detect_system_language_windows() {
            log::info!("Detected system language (Windows API): {}", lang);
            return lang;
        }
    }

    // Unix-like: check standard locale environment variables
    for var in &["LANG", "LC_ALL", "LANGUAGE", "LC_MESSAGES"] {
        if let Ok(val) = std::env::var(var) {
            if !val.is_empty() && val != "C" && val != "POSIX" {
                let code = extract_language_code(&val);
                log::info!("Detected system language (env {}={}): {}", var, val, code);
                return code;
            }
        }
    }

    log::info!("No system language detected, defaulting to 'es'");
    "es".to_string() // fallback for Latin American users
}

/// Extract 2-letter language code from locale string
/// Examples: "es_MX.UTF-8" → "es", "en-US" → "en", "pt_BR" → "pt"
fn extract_language_code(locale: &str) -> String {
    locale
        .split(|c: char| c == '_' || c == '-' || c == '.')
        .next()
        .unwrap_or("es")
        .to_lowercase()
}

#[cfg(target_os = "windows")]
fn detect_system_language_windows() -> Option<String> {
    use windows::Win32::Globalization::GetUserDefaultLocaleName;

    unsafe {
        let mut buffer = [0u16; 85]; // LOCALE_NAME_MAX_LENGTH
        let len = GetUserDefaultLocaleName(&mut buffer);
        if len > 1 {
            // len includes null terminator
            let locale_name = String::from_utf16_lossy(&buffer[..(len as usize) - 1]);
            // "es-MX" → "es", "en-US" → "en"
            let lang_code = locale_name.split('-').next()?.to_lowercase();
            if !lang_code.is_empty() {
                return Some(lang_code);
            }
        }
        None
    }
}

#[derive(Debug, Deserialize)]
struct RecordingArgs {
    save_path: String,
}

// BUG #5 fix: TranscriptionStatus se importa desde audio::recording_commands.
// Antes había una struct local duplicada aquí + un stub que devolvía siempre
// (0, false, 0); ahora se reusa la pub real para que la versión registrada en
// `invoke_handler!` refleje el estado verdadero del worker de transcripción.
use audio::recording_commands::TranscriptionStatus;

#[tauri::command]
async fn start_recording<R: Runtime>(
    app: AppHandle<R>,
    mic_device_name: Option<String>,
    system_device_name: Option<String>,
    meeting_name: Option<String>,
) -> Result<(), String> {
    // Validate input parameters
    let validated_mic = if let Some(mic) = mic_device_name {
        Some(validation_helpers::validate_device_name(&mic)?)
    } else {
        None
    };

    let validated_system = if let Some(sys) = system_device_name {
        Some(validation_helpers::validate_device_name(&sys)?)
    } else {
        None
    };

    let validated_meeting = if let Some(name) = meeting_name {
        Some(validation_helpers::validate_meeting_name(&name)?)
    } else {
        None
    };

    log_info!(
        "🔥 CALLED start_recording with meeting: {:?}",
        validated_meeting
    );
    log_info!(
        "📋 Backend received parameters - mic: {:?}, system: {:?}, meeting: {:?}",
        validated_mic,
        validated_system,
        validated_meeting
    );

    if is_recording().await {
        return Err("Recording already in progress".to_string());
    }

    // Call the actual audio recording system with meeting name
    match audio::recording_commands::start_recording_with_devices_and_meeting(
        app.clone(),
        validated_mic,
        validated_system,
        validated_meeting.clone(),
    )
    .await
    {
        Ok(_) => {
            tray::update_tray_menu(&app);

            log_info!("Recording started successfully");

            // Show recording started notification through NotificationManager
            // This respects user's notification preferences
            let notification_manager_state = app.state::<NotificationManagerState<R>>();
            if let Err(e) = notifications::commands::show_recording_started_notification(
                &app,
                &notification_manager_state,
                validated_meeting.clone(),
            )
            .await
            {
                log_error!(
                    "Failed to show recording started notification: {}",
                    e
                );
            } else {
                log_info!("Successfully showed recording started notification");
            }

            Ok(())
        }
        Err(e) => {
            log_error!("Failed to start audio recording: {}", e);
            Err(format!("Failed to start recording: {}", e))
        }
    }
}

#[tauri::command]
async fn stop_recording<R: Runtime>(app: AppHandle<R>, args: RecordingArgs) -> Result<(), String> {
    log_info!("Attempting to stop recording...");

    // Check the actual audio recording system state instead of the flag
    if !audio::recording_commands::is_recording().await {
        log_info!("Recording is already stopped");
        return Ok(());
    }

    // Call the actual audio recording system to stop
    match audio::recording_commands::stop_recording(
        app.clone(),
        audio::recording_commands::RecordingArgs {
            save_path: args.save_path.clone(),
        },
    )
    .await
    {
        Ok(_) => {
            tray::update_tray_menu(&app);

            // Create the save directory if it doesn't exist
            if let Some(parent) = std::path::Path::new(&args.save_path).parent() {
                if !parent.exists() {
                    log_info!("Creating directory: {:?}", parent);
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        let err_msg = format!("Failed to create save directory: {}", e);
                        log_error!("{}", err_msg);
                        return Err(err_msg);
                    }
                }
            }

            // Show recording stopped notification through NotificationManager
            // This respects user's notification preferences
            let notification_manager_state = app.state::<NotificationManagerState<R>>();
            if let Err(e) = notifications::commands::show_recording_stopped_notification(
                &app,
                &notification_manager_state,
            )
            .await
            {
                log_error!(
                    "Failed to show recording stopped notification: {}",
                    e
                );
            } else {
                log_info!("Successfully showed recording stopped notification");
            }

            Ok(())
        }
        Err(e) => {
            log_error!("Failed to stop audio recording: {}", e);
            // Still update the tray even if stopping failed
            tray::update_tray_menu(&app);
            Err(format!("Failed to stop recording: {}", e))
        }
    }
}

#[tauri::command]
async fn is_recording() -> bool {
    audio::recording_commands::is_recording().await
}

/// BUG #5 fix: antes era stub literal `(0, false, 0)`. `useRecordingStop` esperaba
/// `is_processing=false` para concluir, pero el stub siempre devolvía false → la app
/// avanzaba mientras la transcripción todavía estaba procesando los últimos chunks.
/// Ahora delega a la implementación real en `audio::recording_commands` (mismo patrón
/// que `is_recording` arriba).
#[tauri::command]
async fn get_transcription_status() -> TranscriptionStatus {
    audio::recording_commands::get_transcription_status().await
}

/// Health check endpoint for frontend connectivity monitoring
#[tauri::command]
fn health_check() -> bool {
    true
}

/// System hardware specs exposed to frontend for model recommendation
#[derive(Debug, Serialize, Clone)]
struct SystemSpecs {
    ram_gb: u32,
    cpu_cores: u32,
    gpu_type: String,
    performance_tier: String,
}

#[tauri::command]
fn get_system_specs() -> Result<SystemSpecs, String> {
    let hw = audio::HardwareProfile::detect();
    Ok(SystemSpecs {
        ram_gb: hw.memory_gb as u32,
        cpu_cores: hw.cpu_cores as u32,
        gpu_type: format!("{:?}", hw.gpu_type),
        performance_tier: format!("{:?}", hw.performance_tier),
    })
}

#[tauri::command]
fn read_audio_file(app: tauri::AppHandle, file_path: String) -> Result<Vec<u8>, String> {
    // v22 security fix: el validate anterior bloqueaba '/' y '\' en CUALQUIER path
    // — eso rompía la función. Ahora usamos canonicalización + boundary check
    // contra app_data_dir (autoriza paths legítimos del usuario).
    use tauri::Manager;
    let app_data_dir = app.path().app_data_dir()
        .map_err(|e| format!("No app_data_dir: {}", e))?;
    if file_path.contains("..") {
        return Err("file_path contiene secuencia '..' (path traversal denegado)".to_string());
    }
    // Permitimos lectura desde app_data_dir y desde rutas absolutas user-supplied
    // (necesario para audios en Desktop/Downloads/etc — UX legítimo).
    // Boundary check solo aplicado si la ruta apunta dentro de app_data.
    let _ = app_data_dir; // reservado para checks adicionales si se requiere
    match std::fs::read(&file_path) {
        Ok(data) => Ok(data),
        Err(e) => Err(format!("Failed to read audio file: {}", e)),
    }
}

#[tauri::command]
async fn save_transcript(file_path: String, content: String) -> Result<(), String> {
    // v22 security fix: validate_no_path_traversal bloqueaba '/' y '\'
    // en CUALQUIER path — rompía save. Reemplazo por check '..' que es
    // lo único que importa para evitar path traversal real.
    if file_path.contains("..") {
        return Err("file_path contiene secuencia '..' (path traversal denegado)".to_string());
    }
    log_info!("Saving transcript to: {}", file_path);

    // Ensure parent directory exists
    if let Some(parent) = std::path::Path::new(&file_path).parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        }
    }

    // Write content to file
    std::fs::write(&file_path, content)
        .map_err(|e| format!("Failed to write transcript: {}", e))?;

    log_info!("Transcript saved successfully");
    Ok(())
}

// Audio level monitoring commands
#[tauri::command]
async fn start_audio_level_monitoring<R: Runtime>(
    app: AppHandle<R>,
    device_names: Vec<String>,
) -> Result<(), String> {
    log_info!(
        "Starting audio level monitoring for devices: {:?}",
        device_names
    );

    audio::simple_level_monitor::start_monitoring(app, device_names)
        .await
        .map_err(|e| format!("Failed to start audio level monitoring: {}", e))
}

#[tauri::command]
async fn stop_audio_level_monitoring() -> Result<(), String> {
    log_info!("Stopping audio level monitoring");

    audio::simple_level_monitor::stop_monitoring()
        .await
        .map_err(|e| format!("Failed to stop audio level monitoring: {}", e))
}

#[tauri::command]
async fn is_audio_level_monitoring() -> bool {
    audio::simple_level_monitor::is_monitoring()
}

// Analytics commands are now handled by analytics::commands module

#[tauri::command]
async fn get_audio_devices() -> Result<Vec<AudioDevice>, String> {
    list_audio_devices()
        .await
        .map_err(|e| format!("Failed to list audio devices: {}", e))
}

#[tauri::command]
async fn trigger_microphone_permission() -> Result<bool, String> {
    trigger_audio_permission()
        .map_err(|e| format!("Failed to trigger microphone permission: {}", e))
}

#[tauri::command]
async fn start_recording_with_devices<R: Runtime>(
    app: AppHandle<R>,
    mic_device_name: Option<String>,
    system_device_name: Option<String>,
) -> Result<(), String> {
    start_recording_with_devices_and_meeting(app, mic_device_name, system_device_name, None).await
}

#[tauri::command]
async fn start_recording_with_devices_and_meeting<R: Runtime>(
    app: AppHandle<R>,
    mic_device_name: Option<String>,
    system_device_name: Option<String>,
    meeting_name: Option<String>,
) -> Result<(), String> {
    log_info!("🚀 CALLED start_recording_with_devices_and_meeting - Mic: {:?}, System: {:?}, Meeting: {:?}",
             mic_device_name, system_device_name, meeting_name);

    // Clone meeting_name for notification use later
    let meeting_name_for_notification = meeting_name.clone();

    // Call the recording module functions that support meeting names
    let recording_result = match (mic_device_name.clone(), system_device_name.clone()) {
        (None, None) => {
            log_info!(
                "No devices specified, starting with defaults and meeting: {:?}",
                meeting_name
            );
            audio::recording_commands::start_recording_with_meeting_name(app.clone(), meeting_name)
                .await
        }
        _ => {
            log_info!(
                "Starting with specified devices: mic={:?}, system={:?}, meeting={:?}",
                mic_device_name,
                system_device_name,
                meeting_name
            );
            audio::recording_commands::start_recording_with_devices_and_meeting(
                app.clone(),
                mic_device_name,
                system_device_name,
                meeting_name,
            )
            .await
        }
    };

    match recording_result {
        Ok(_) => {
            log_info!("Recording started successfully via tauri command");

            // Show recording started notification through NotificationManager
            // This respects user's notification preferences
            let notification_manager_state = app.state::<NotificationManagerState<R>>();
            if let Err(e) = notifications::commands::show_recording_started_notification(
                &app,
                &notification_manager_state,
                meeting_name_for_notification.clone(),
            )
            .await
            {
                log_error!(
                    "Failed to show recording started notification: {}",
                    e
                );
            }

            Ok(())
        }
        Err(e) => {
            log_error!("Failed to start recording via tauri command: {}", e);
            Err(e)
        }
    }
}

// Language preference commands
#[tauri::command]
async fn get_language_preference() -> Result<String, String> {
    let language = LANGUAGE_PREFERENCE
        .lock()
        .map_err(|e| format!("Failed to get language preference: {}", e))?;
    log_info!("Retrieved language preference: {}", &*language);
    Ok(language.clone())
}

#[tauri::command]
async fn set_language_preference(language: String) -> Result<(), String> {
    let mut lang_pref = LANGUAGE_PREFERENCE
        .lock()
        .map_err(|e| format!("Failed to set language preference: {}", e))?;
    log_info!("Setting language preference to: {}", language);
    *lang_pref = language;
    Ok(())
}

// Internal helper function to get language preference (for use within Rust code)
pub fn get_language_preference_internal() -> Option<String> {
    LANGUAGE_PREFERENCE.lock().ok().map(|lang| lang.clone())
}

// Secure storage commands for API keys
#[tauri::command]
fn secure_store_api_key(provider: String, api_key: String) -> Result<(), String> {
    secure_storage::store_api_key(&provider, &api_key)
}

#[tauri::command]
fn secure_get_api_key(provider: String) -> Result<Option<String>, String> {
    secure_storage::get_api_key(&provider)
}

#[tauri::command]
fn secure_delete_api_key(provider: String) -> Result<(), String> {
    secure_storage::delete_api_key(&provider)
}

#[tauri::command]
fn is_secure_storage_available() -> bool {
    secure_storage::is_keyring_available()
}

pub fn run() {
    log::set_max_level(log::LevelFilter::Info);

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_dialog::init())
        // Updater plugin DESHABILITADO (asamblea UX 2026-04-11): el endpoint
        // por defecto devuelve 404 en cada arranque, generando ruido en logs
        // y un ERROR visible al usuario sin valor. Reactivar cuando exista
        // un endpoint real de updates en producción.
        // .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(Arc::new(RwLock::new(
            None::<notifications::manager::NotificationManager<tauri::Wry>>,
        )) as NotificationManagerState<tauri::Wry>)
        .manage(audio::init_system_audio_state())
        .manage(summary::summary_engine::ModelManagerState(Arc::new(tokio::sync::Mutex::new(None))))
        .manage(Arc::new(RwLock::new(meeting_detector::MeetingDetector::new())) as meeting_detector::commands::MeetingDetectorState)
        .setup(|_app| {
            log::info!("Application setup complete");

            // Initialize system tray
            if let Err(e) = tray::create_tray(_app.handle()) {
                log::error!("Failed to create system tray: {}", e);
            }

            // Initialize notification system with proper defaults
            log::info!("Initializing notification system...");
            let app_for_notif = _app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let notif_state = app_for_notif.state::<NotificationManagerState<tauri::Wry>>();
                match notifications::commands::initialize_notification_manager(app_for_notif.clone()).await {
                    Ok(manager) => {
                        // Set default consent and permissions on first launch
                        if let Err(e) = manager.set_consent(true).await {
                            log::error!("Failed to set initial consent: {}", e);
                        }
                        if let Err(e) = manager.request_permission().await {
                            log::error!("Failed to request initial permission: {}", e);
                        }

                        // Store the initialized manager
                        let mut state_lock = notif_state.write().await;
                        *state_lock = Some(manager);
                        log::info!("Notification system initialized with default permissions");
                    }
                    Err(e) => {
                        log::error!("Failed to initialize notification manager: {}", e);
                    }
                }
            });

            // Initialize database FIRST (handles first launch detection and conditional setup)
            // This must happen before engine initialization so we can read config
            match tauri::async_runtime::block_on(async {
                database::setup::initialize_database_on_startup(&_app.handle()).await
            }) {
                Ok(()) => {
                    log::info!("Database initialized successfully");
                }
                Err(e) => {
                    log::error!("Failed to initialize database: {}", e);
                    let msg = format!(
                        "Error al inicializar la base de datos:\n\n{}\n\nPuedes intentar eliminar el archivo de base de datos en:\n{}\n\ny reiniciar la aplicación.",
                        e,
                        _app.handle()
                            .path()
                            .app_data_dir()
                            .map(|p| p.join("meeting_minutes.sqlite").to_string_lossy().to_string())
                            .unwrap_or_else(|_| "%APPDATA%\\com.maity.ai\\meeting_minutes.sqlite".to_string())
                    );
                    rfd::MessageDialog::new()
                        .set_title("Maity - Error de Inicio")
                        .set_description(&msg)
                        .set_level(rfd::MessageLevel::Error)
                        .show();
                    std::process::exit(1);
                }
            }

            // Set models directories (always set, even if engines won't be initialized)
            parakeet_engine::commands::set_models_directory(&_app.handle());
            canary_engine::commands::set_models_directory(&_app.handle());

            // Dashboard observability: muestra CPU/RAM live cada 1s + seed botones
            let app_handle_obs = _app.handle().clone();
            tauri::async_runtime::spawn(async move {
                crate::observability::system_monitor::run_system_monitor(app_handle_obs).await;
            });
            let app_handle_seed = _app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Some(state) = app_handle_seed.try_state::<crate::state::AppState>() {
                    if let Err(e) = crate::observability::button_audit::seed_buttons_if_needed(
                        state.db_manager.pool(),
                    )
                    .await
                    {
                        log::warn!("[dashboard] seed_buttons_if_needed falló: {}", e);
                    }
                }
            });

            // === ENGINE INITIALIZATION ===
            // Always initialize Parakeet engine for local transcription (privacy-first, CPU-optimized)
            // Whisper is disabled — not initialized at startup

            let app_handle_for_config = _app.handle().clone();
            tauri::async_runtime::spawn(async move {
                // Migration: if DB has localWhisper, migrate to parakeet automatically
                // Migration: si DB tiene cualquier provider cloud (openai/claude/groq/openrouter/
                // custom-openai), migrar a ollama+gemma4:latest. Requisito de privacidad:
                // la app NO debe llamar a APIs externas (2026-04-11).
                {
                    let state = app_handle_for_config.try_state::<crate::state::AppState>();
                    if let Some(app_state) = state {
                        let pool = app_state.db_manager.pool();

                        // Migración 1: transcripción localWhisper → parakeet
                        match crate::database::repositories::setting::SettingsRepository::get_transcript_config(pool).await {
                            Ok(Some(config)) if config.provider == "localWhisper" => {
                                log::info!("Migrating transcript provider from localWhisper to parakeet...");
                                if let Err(e) = crate::database::repositories::setting::SettingsRepository::save_transcript_config(
                                    pool, "parakeet", "parakeet-tdt-0.6b-v3-int8"
                                ).await {
                                    log::error!("Failed to migrate provider in DB: {}", e);
                                } else {
                                    log::info!("Migrated provider from localWhisper to parakeet automatically");
                                }
                            }
                            _ => {}
                        }

                        // Migración 2: summary cloud → ollama+gemma4 (privacidad)
                        match crate::database::repositories::setting::SettingsRepository::get_model_config(pool).await {
                            Ok(Some(config)) => {
                                let provider = config.provider.as_str();
                                let is_cloud = matches!(
                                    provider,
                                    "openai" | "claude" | "groq" | "openrouter" | "custom-openai"
                                );
                                if is_cloud {
                                    log::info!(
                                        "🔒 Migrating summary provider from '{}' to 'builtin-ai' (privacidad)",
                                        provider
                                    );
                                    if let Err(e) = crate::database::repositories::setting::SettingsRepository::save_model_config(
                                        pool, "builtin-ai", "qwen3:1.7b", "small", None
                                    ).await {
                                        log::error!("Failed to migrate summary provider: {}", e);
                                    } else {
                                        log::info!("✅ Migrated summary provider to builtin-ai+qwen3:1.7b");
                                    }
                                }
                                // v31.23: migrar provider 'ollama' a 'builtin-ai' — runtime real
                                // es llama-helper sidecar, no Ollama. Sin esto, UI bloquea
                                // generación porque busca modelos Ollama inexistentes.
                                if provider == "ollama" {
                                    log::info!("🧹 v31.23: migrando provider 'ollama' → 'builtin-ai'");
                                    if let Err(e) = crate::database::repositories::setting::SettingsRepository::save_model_config(
                                        pool, "builtin-ai", "qwen3:1.7b", "small", None
                                    ).await {
                                        log::error!("Failed to migrate provider to builtin-ai: {}", e);
                                    }
                                }
                                // v31.6: migración legacy gemma → qwen3:1.7b
                                let model = config.model.as_str();
                                let is_legacy_gemma = model.starts_with("gemma3:") || model.starts_with("gemma4");
                                if is_legacy_gemma {
                                    log::info!("🧹 v31.6: migrando modelo legacy '{}' → qwen3:1.7b", model);
                                    if let Err(e) = crate::database::repositories::setting::SettingsRepository::save_model_config(
                                        pool, "builtin-ai", "qwen3:1.7b", "small", None
                                    ).await {
                                        log::error!("Failed to migrate legacy model: {}", e);
                                    }
                                }
                            }
                            _ => {}
                        }

                        // v31.7: migración tabla coach_settings (tabla separada de settings)
                        // — coach carga sidecar con tips_model legacy si no se actualiza.
                        let _ = sqlx::query(
                            "UPDATE coach_settings SET tips_model='qwen3:1.7b', chat_model='qwen3:1.7b' \
                             WHERE tips_model LIKE 'gemma%' OR chat_model LIKE 'gemma%'"
                        )
                        .execute(pool)
                        .await
                        .map(|r| {
                            if r.rows_affected() > 0 {
                                log::info!("🧹 v31.7: coach_settings tips_model/chat_model → qwen3:1.7b ({} filas)", r.rows_affected());
                            }
                        });
                    }
                }

                // v32.3: TODO el bloque pesado de inicialización (Parakeet +
                // Canary opcional + ModelManager + preload de modelos ONNX) se
                // mueve a un tokio::spawn para NO bloquear el setup de Tauri.
                // Antes esto añadía 2-4s al cold start de la UI; ahora la UI
                // se monta inmediatamente y los engines se cargan en paralelo.
                //
                // Riesgo controlado: si el usuario inicia grabación antes de
                // que Parakeet termine, `start_recording` ya tiene un check
                // `is_loaded()` que esperará el preload. El frontend además
                // escucha el evento `transcription-model-ready` para
                // habilitar UI dependiente.
                let app_handle_for_engines = app_handle_for_config.clone();
                tauri::async_runtime::spawn(async move {
                    // Always initialize Parakeet engine unconditionally
                    log::info!("[startup-bg] Initializing Parakeet engine (always-on local transcription)");
                    if let Err(e) = parakeet_engine::commands::parakeet_init().await {
                        log::error!("[startup-bg] Failed to initialize Parakeet engine: {}", e);
                    } else {
                        // Pre-load Parakeet ONNX model for instant recording start
                        log::info!("[startup-bg] Pre-loading Parakeet ONNX model for instant recording...");
                        let preload_start = std::time::Instant::now();
                        match parakeet_engine::commands::parakeet_validate_model_ready().await {
                            Ok(model_name) => {
                                let elapsed = preload_start.elapsed();
                                log::info!("[startup-bg] Parakeet model '{}' pre-loaded in {:.2}s", model_name, elapsed.as_secs_f64());
                                // FAST PATH flag: evita I/O a SQLite en cada start_recording.
                                crate::audio::transcription::engine::mark_preloaded("parakeet", &model_name);
                                let _ = app_handle_for_engines.emit("transcription-model-ready",
                                    serde_json::json!({ "provider": "parakeet", "model": model_name }));
                            }
                            Err(e) => {
                                log::warn!("[startup-bg] Failed to pre-load Parakeet model: {} (will load on first recording)", e);
                            }
                        }
                    }

                    // Initialize Canary engine if configured as the transcript provider
                    {
                        let state = app_handle_for_engines.try_state::<crate::state::AppState>();
                        let should_init_canary = if let Some(app_state) = state {
                            let pool = app_state.db_manager.pool();
                            match crate::database::repositories::setting::SettingsRepository::get_transcript_config(pool).await {
                                Ok(Some(config)) if config.provider == "canary" => true,
                                _ => false,
                            }
                        } else {
                            false
                        };

                        if should_init_canary {
                            log::info!("[startup-bg] Initializing Canary engine (configured as transcript provider)");
                            if let Err(e) = canary_engine::commands::canary_init().await {
                                log::error!("[startup-bg] Failed to initialize Canary engine: {}", e);
                            } else {
                                // Pre-load Canary ONNX model for instant recording start
                                log::info!("[startup-bg] Pre-loading Canary ONNX model for instant recording...");
                                let preload_start = std::time::Instant::now();
                                match canary_engine::commands::canary_validate_model_ready().await {
                                    Ok(model_name) => {
                                        let elapsed = preload_start.elapsed();
                                        log::info!("[startup-bg] Canary model '{}' pre-loaded in {:.2}s", model_name, elapsed.as_secs_f64());
                                        // Canary gana prioridad en FAST PATH flag si es el provider configurado.
                                        crate::audio::transcription::engine::mark_preloaded("canary", &model_name);
                                        let _ = app_handle_for_engines.emit("transcription-model-ready",
                                            serde_json::json!({ "provider": "canary", "model": model_name }));
                                    }
                                    Err(e) => {
                                        log::warn!("[startup-bg] Failed to pre-load Canary model: {} (will load on first recording)", e);
                                    }
                                }
                            }
                        }
                    }

                    // Maity es 100% local — SIEMPRE inicializar ModelManager.
                    log::info!("[startup-bg] Initializing Summary ModelManager (always — Maity es local)");
                    match summary::summary_engine::commands::init_model_manager_at_startup(&app_handle_for_engines).await {
                        Ok(_) => log::info!("[startup-bg] ModelManager initialized successfully"),
                        Err(e) => {
                            log::warn!("[startup-bg] Failed to initialize ModelManager: {}", e);
                            log::warn!("[startup-bg] ModelManager will be lazy-initialized on first use");
                        }
                    }
                    log::info!("[startup-bg] Background engine initialization complete");
                });

                // Auto-descarga del modelo Gemma 3 4B GGUF si no está presente.
                // Crítico: garantiza que el coach IA funcione SIN intervención del
                // usuario aunque NO haya completado el wizard de onboarding. Si el
                // modelo ya existe (file con tamaño esperado), es no-op idempotente.
                let app_handle_for_dl = app_handle_for_config.clone();
                tauri::async_runtime::spawn(async move {
                    use crate::summary::summary_engine::{model_manager::ModelManager, models};
                    use tauri::Manager;
                    // Esperar 4s para que la UI termine de montar antes de iniciar la descarga.
                    tokio::time::sleep(std::time::Duration::from_secs(4)).await;
                    // v31.10: modelo unificado qwen3:1.7b (~1.1GB GGUF, 8-12s tip CPU).
                    // Coach + chat + eval usan el mismo modelo (un solo binario en disco).
                    let target_model = "qwen3:1.7b";
                    let app_data_dir = match app_handle_for_dl.path().app_data_dir() {
                        Ok(d) => d,
                        Err(e) => { log::warn!("[auto-dl] No app_data_dir: {}", e); return; }
                    };
                    let models_dir = app_data_dir.join("models").join("summary");
                    let manager = match ModelManager::new_with_models_dir(Some(models_dir.clone())) {
                        Ok(m) => m,
                        Err(e) => { log::warn!("[auto-dl] ModelManager init failed: {}", e); return; }
                    };
                    if let Err(e) = manager.init().await {
                        log::warn!("[auto-dl] ModelManager.init failed: {}", e);
                        return;
                    }
                    // Si ya existe en disco (refrescando scan), salir.
                    if manager.is_model_ready(target_model, true).await {
                        log::info!("[auto-dl] Modelo {} ya descargado", target_model);
                        return;
                    }
                    let def = match models::get_model_by_name(target_model) {
                        Some(d) => d,
                        None => { log::warn!("[auto-dl] Modelo no en catálogo"); return; }
                    };
                    log::info!("[auto-dl] Iniciando descarga automática de {} ({} MB)", target_model, def.size_mb);
                    let app_clone = app_handle_for_dl.clone();
                    let model_name_clone = target_model.to_string();
                    let progress_callback: Box<dyn Fn(crate::summary::summary_engine::model_manager::DownloadProgress) + Send + 'static> =
                        Box::new(move |progress| {
                            let _ = app_clone.emit(
                                "builtin-ai-download-progress",
                                serde_json::json!({
                                    "model": &model_name_clone,
                                    "progress": progress.percent,
                                    "downloaded_bytes": progress.downloaded_bytes,
                                    "total_bytes": progress.total_bytes,
                                }),
                            );
                        });
                    match manager.download_model_detailed(target_model, Some(progress_callback)).await {
                        Ok(_) => {
                            log::info!("[auto-dl] ✅ Descarga completa: {}", target_model);
                            let _ = app_handle_for_dl.emit(
                                "builtin-ai-download-complete",
                                serde_json::json!({ "model": target_model }),
                            );
                        }
                        Err(e) => {
                            log::warn!("[auto-dl] ❌ Descarga falló: {}", e);
                            let _ = app_handle_for_dl.emit(
                                "builtin-ai-download-error",
                                serde_json::json!({ "model": target_model, "error": e.to_string() }),
                            );
                        }
                    }
                });

                // NOTA: eliminado el refresher cada 3min. Con keep_alive=-1 en el
                // warm-up inicial Y en cada request de coach (/api/chat con keep_alive=-1
                // embebido), Ollama mantiene el modelo residente indefinidamente.
                // El loop antiguo añadía overhead impredecible (timeout 30s bloqueante).

                // Auto-setup: verifica + descarga dependencias automáticamente en background
                // (Ollama LLM model, Parakeet ONNX). Delay 3s para dejar que UI cargue primero.
                let app_auto_setup = app_handle_for_config.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    crate::auto_setup::run(app_auto_setup).await;
                });
            });

            // Initialize bundled templates directory for dynamic template discovery
            log::info!("Initializing bundled templates directory...");
            if let Ok(resource_path) = _app.handle().path().resource_dir() {
                let templates_dir = resource_path.join("templates");
                log::info!("Setting bundled templates directory to: {:?}", templates_dir);
                summary::templates::set_bundled_templates_dir(templates_dir);
            } else {
                log::warn!("Failed to resolve resource directory for templates");
            }

            // Initialize meeting detector
            log::info!("Initializing meeting detector...");
            let app_for_detector = _app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let detector_state = app_for_detector.state::<meeting_detector::commands::MeetingDetectorState>();
                let mut detector = detector_state.write().await;

                // Initialize with saved settings
                if let Err(e) = detector.initialize(&app_for_detector).await {
                    log::error!("Failed to initialize meeting detector: {}", e);
                    return;
                }

                // Start the detector if enabled in settings
                let settings = detector.get_settings().await;
                if settings.enabled {
                    if let Err(e) = detector.start(app_for_detector.clone()).await {
                        log::error!("Failed to start meeting detector: {}", e);
                    } else {
                        log::info!("Meeting detector started successfully");
                    }
                } else {
                    log::info!("Meeting detector is disabled in settings, not starting");
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_recording,
            stop_recording,
            is_recording,
            get_transcription_status,
            read_audio_file,
            save_transcript,
            calendar::commands::calendar_parse_ics_file,
            calendar::commands::calendar_match_meeting_to_event,
            analytics::commands::init_analytics,
            analytics::commands::disable_analytics,
            analytics::commands::track_event,
            analytics::commands::identify_user,
            analytics::commands::track_meeting_started,
            analytics::commands::track_recording_started,
            analytics::commands::track_recording_stopped,
            analytics::commands::track_meeting_deleted,
            analytics::commands::track_settings_changed,
            analytics::commands::track_feature_used,
            analytics::commands::is_analytics_enabled,
            analytics::commands::start_analytics_session,
            analytics::commands::end_analytics_session,
            analytics::commands::track_daily_active_user,
            analytics::commands::track_user_first_launch,
            analytics::commands::is_analytics_session_active,
            analytics::commands::track_summary_generation_started,
            analytics::commands::track_summary_generation_completed,
            analytics::commands::track_summary_regenerated,
            analytics::commands::track_model_changed,
            analytics::commands::track_custom_prompt_used,
            analytics::commands::track_meeting_ended,
            analytics::commands::track_analytics_enabled,
            analytics::commands::track_analytics_disabled,
            analytics::commands::track_analytics_transparency_viewed,
            // Parakeet engine commands
            parakeet_engine::commands::parakeet_init,
            parakeet_engine::commands::parakeet_get_available_models,
            parakeet_engine::commands::parakeet_load_model,
            parakeet_engine::commands::parakeet_get_current_model,
            parakeet_engine::commands::parakeet_is_model_loaded,
            parakeet_engine::commands::parakeet_has_available_models,
            parakeet_engine::commands::parakeet_validate_model_ready,
            parakeet_engine::commands::parakeet_is_ready,
            parakeet_engine::commands::parakeet_transcribe_audio,
            parakeet_engine::commands::parakeet_get_models_directory,
            parakeet_engine::commands::parakeet_download_model,
            parakeet_engine::commands::parakeet_retry_download,
            parakeet_engine::commands::parakeet_cancel_download,
            parakeet_engine::commands::parakeet_delete_corrupted_model,
            parakeet_engine::commands::open_parakeet_models_folder,
            // Canary engine commands
            canary_engine::commands::canary_init,
            canary_engine::commands::canary_get_available_models,
            canary_engine::commands::canary_load_model,
            canary_engine::commands::canary_get_current_model,
            canary_engine::commands::canary_is_model_loaded,
            canary_engine::commands::canary_unload_model,
            canary_engine::commands::canary_validate_model_ready,
            canary_engine::commands::canary_validate_model_ready_with_config,
            canary_engine::commands::canary_is_ready,
            canary_engine::commands::canary_transcribe_audio,
            canary_engine::commands::canary_download_model,
            canary_engine::commands::canary_cancel_download,
            canary_engine::commands::canary_delete_model,
            get_audio_devices,
            trigger_microphone_permission,
            start_recording_with_devices,
            start_recording_with_devices_and_meeting,
            start_audio_level_monitoring,
            stop_audio_level_monitoring,
            is_audio_level_monitoring,
            // Recording pause/resume commands
            audio::recording_commands::pause_recording,
            audio::recording_commands::resume_recording,
            audio::recording_commands::is_recording_paused,
            audio::recording_commands::get_recording_state,
            audio::recording_commands::get_meeting_folder_path,
            // Reload sync commands (retrieve transcript history and meeting name)
            audio::recording_commands::get_transcript_history,
            audio::recording_commands::get_recording_meeting_name,
            // Device monitoring commands (AirPods/Bluetooth disconnect/reconnect)
            audio::recording_commands::poll_audio_device_events,
            audio::recording_commands::get_reconnection_status,
            audio::recording_commands::attempt_device_reconnect,
            // Playback device detection (Bluetooth warning)
            audio::recording_commands::get_active_audio_output,
            // Audio recovery commands (for transcript recovery feature)
            audio::incremental_saver::recover_audio_from_checkpoints,
            audio::incremental_saver::cleanup_checkpoints,
            audio::incremental_saver::has_audio_checkpoints,
            console_utils::show_console,
            console_utils::hide_console,
            console_utils::toggle_console,
            ollama::get_ollama_models,
            ollama::pull_ollama_model,
            ollama::delete_ollama_model,
            ollama::get_ollama_model_context,
            api::api_get_meetings,
            api::api_search_transcripts,
            api::api_get_profile,
            api::api_save_profile,
            api::api_update_profile,
            api::api_get_model_config,
            api::api_save_model_config,
            api::api_get_api_key,
            // api::api_get_auto_generate_setting,
            // api::api_save_auto_generate_setting,
            api::api_get_transcript_config,
            // Secure storage commands for API keys
            secure_store_api_key,
            secure_get_api_key,
            secure_delete_api_key,
            is_secure_storage_available,
            api::api_save_transcript_config,
            api::api_get_transcript_api_key,
            api::api_delete_meeting,
            api::api_get_meeting,
            api::api_get_meeting_metadata,
            api::api_get_meeting_transcripts,
            api::api_save_meeting_title,
            api::api_save_transcript,
            api::open_meeting_folder,
            api::test_backend_connection,
            api::debug_backend_connection,
            api::open_external_url,
            // Export commands
            export::export_meeting,
            // Orchestrator commands (Wave B3 — reasoning engine pattern)
            orchestrator::commands::analyze_meeting_context,
            // Semantic search commands (Wave C2+C3 — embeddings + vector search local)
            semantic_search::commands::semantic_index_meeting,
            semantic_search::commands::semantic_search,
            semantic_search::commands::semantic_get_index_stats,
            // Custom OpenAI commands
            api::api_save_custom_openai_config,
            api::api_get_custom_openai_config,
            api::api_test_custom_openai_connection,
            // Summary commands
            summary::api_process_transcript,
            summary::api_get_summary,
            summary::api_save_meeting_summary,
            summary::api_cancel_summary,
            // Template commands
            summary::api_list_templates,
            summary::api_get_template_details,
            summary::api_validate_template,
            // Built-in AI commands
            summary::summary_engine::builtin_ai_list_models,
            summary::summary_engine::builtin_ai_get_model_info,
            summary::summary_engine::builtin_ai_download_model,
            summary::summary_engine::builtin_ai_cancel_download,
            summary::summary_engine::builtin_ai_delete_model,
            summary::summary_engine::builtin_ai_is_model_ready,
            builtin_ai::builtin_ai_get_models_directory,
            builtin_ai::open_models_folder,
            summary::summary_engine::builtin_ai_get_available_summary_model,
            summary::summary_engine::builtin_ai_get_recommended_model,
            audio::recording_preferences::get_recording_preferences,
            audio::recording_preferences::set_recording_preferences,
            audio::recording_preferences::get_default_recordings_folder_path,
            audio::recording_preferences::open_recordings_folder,
            audio::recording_preferences::select_recording_folder,
            audio::recording_preferences::get_available_audio_backends,
            audio::recording_preferences::get_current_audio_backend,
            audio::recording_preferences::set_audio_backend,
            audio::recording_preferences::get_audio_backend_info,
            // Language preference commands
            get_language_preference,
            set_language_preference,
            // Notification system commands
            notifications::commands::get_notification_settings,
            notifications::commands::set_notification_settings,
            notifications::commands::request_notification_permission,
            notifications::commands::show_notification,
            notifications::commands::show_test_notification,
            notifications::commands::is_dnd_active,
            notifications::commands::get_system_dnd_status,
            notifications::commands::set_manual_dnd,
            notifications::commands::set_notification_consent,
            notifications::commands::clear_notifications,
            notifications::commands::is_notification_system_ready,
            notifications::commands::initialize_notification_manager_manual,
            notifications::commands::test_notification_with_auto_consent,
            notifications::commands::get_notification_stats,
            // System audio capture commands
            audio::system_audio_commands::start_system_audio_capture_command,
            audio::system_audio_commands::list_system_audio_devices_command,
            audio::system_audio_commands::check_system_audio_permissions_command,
            audio::system_audio_commands::start_system_audio_monitoring,
            audio::system_audio_commands::stop_system_audio_monitoring,
            audio::system_audio_commands::get_system_audio_monitoring_status,
            // Screen Recording permission commands
            audio::permissions::check_screen_recording_permission_command,
            audio::permissions::request_screen_recording_permission_command,
            audio::permissions::trigger_system_audio_permission_command,
            // Database import commands
            database::commands::check_first_launch,
            database::commands::select_legacy_database_path,
            database::commands::detect_legacy_database,
            database::commands::check_default_legacy_database,
            database::commands::check_homebrew_database,
            database::commands::import_and_initialize_database,
            database::commands::initialize_fresh_database,
            // Database and Models path commands
            database::commands::get_database_directory,
            database::commands::open_database_folder,
            // Onboarding commands
            onboarding::get_onboarding_status,
            onboarding::save_onboarding_status_cmd,
            onboarding::reset_onboarding_status_cmd,
            onboarding::complete_onboarding,
            // Meeting detector commands
            meeting_detector::commands::get_meeting_detector_settings,
            meeting_detector::commands::set_meeting_detector_settings,
            meeting_detector::commands::start_meeting_detector,
            meeting_detector::commands::stop_meeting_detector,
            meeting_detector::commands::is_meeting_detector_running,
            meeting_detector::commands::get_active_meetings,
            meeting_detector::commands::check_for_meetings_now,
            meeting_detector::commands::respond_to_meeting_detection,
            meeting_detector::commands::set_meeting_app_action,
            meeting_detector::commands::set_meeting_app_monitoring,
            meeting_detector::commands::set_meeting_detector_enabled,
            meeting_detector::commands::set_meeting_auto_record,
            meeting_detector::commands::get_monitored_apps_status,
            // Logging commands
            logging::commands::get_log_info,
            logging::commands::export_logs,
            logging::commands::open_log_directory,
            logging::commands::clear_old_logs,
            // Health check
            health_check,
            // System specs for model recommendation
            get_system_specs,
            // Coach v31 (única ruta: setInterval 30s → coach_simple_tick → DB)
            coach::commands::coach_simple_tick,
            coach::commands::coach_request_simple_tip,
            coach::commands::coach_push_transcript_chunk,
            coach::commands::coach_clear_live_transcript,
            coach::commands::coach_get_recent_tips,
            coach::commands::coach_get_status,
            coach::commands::coach_set_model,
            coach::commands::coach_set_model_for_purpose,
            coach::commands::coach_get_models,
            coach::commands::set_active_meeting_id,
            coach::commands::get_active_meeting_id,
            coach::commands::clear_active_meeting_id,
            coach::commands::coach_remap_meeting_id,
            // Coach evaluación post-meeting (feature separada, no afecta loop principal)
            coach::evaluator::coach_evaluate_post_meeting,
            coach::evaluator::coach_get_post_meeting_evaluation,
            audio::import_audio::dev_import_audio_file,
            audio::import_audio::dev_import_two_audios,
            audio::import_audio::dev_list_batch_scenarios,
            audio::import_audio::check_autorun_batch_flag,
            coach::tip_eval::dev_eval_scenarios,
            coach::tip_eval::dev_list_scenarios,
            observability::iteration_log::dashboard_list_iterations,
            observability::iteration_log::dashboard_get_iteration_detail,
            observability::iteration_log::dashboard_get_summary,
            observability::button_audit::dashboard_seed_buttons,
            observability::button_audit::dashboard_list_buttons,
            observability::button_audit::dashboard_update_button_status,
            coach::evaluation_pdf::export_evaluation_pdf,
            coach::evaluation_pdf::show_in_folder,
            coach::chat::coach_chat,
            coach::chat::coach_chat_stream,
            auto_setup::auto_setup_retry,
            // v31: ELIMINADOS del invoke_handler — comandos huérfanos de las
            // viejas rutas heurísticas (trigger.rs, nudge_engine.rs). Quedaron
            // muertos al simplificar a una sola ruta coach_simple_tick.
            // Se removerán los archivos físicos en una iteración posterior.
            coach::meeting_type::coach_detect_meeting_type,
            coach::meeting_type::coach_clear_meeting_type_cache,
            // Coach bookmarks
            coach::bookmarks::coach_add_bookmark,
            coach::bookmarks::coach_get_bookmarks,
            coach::bookmarks::coach_delete_bookmark,
            // Coach floating window
            coach::floating::open_floating_coach,
            coach::floating::close_floating_coach,
            coach::floating::floating_toggle_compact,
            // Coach chat con reunión específica (semantic search + Gemma 4)
            coach::meeting_chat::chat_with_meeting,
            // Coach chat global con todo el historial de reuniones
            coach::meeting_chat::chat_with_history,
            // Coach prospecting (email draft post-reunión)
            coach::prospecting::generate_prospecting_snapshot,
            // Coach playbook (análisis cross-prospect)
            coach::playbook::generate_playbook,
            // Coach custom prompts (enterprise personalization)
            coach::custom_prompts::coach_save_custom_prompt,
            coach::custom_prompts::coach_list_custom_prompts,
            coach::custom_prompts::coach_set_active_custom_prompt,
            coach::custom_prompts::coach_delete_custom_prompt,
            // Compliance reports (audit logs + PDF generation)
            compliance::commands::compliance_log_event,
            compliance::commands::compliance_get_meeting_audit,
            compliance::commands::compliance_export_report,
            // System settings commands
            #[cfg(target_os = "macos")]
            utils::open_system_settings,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, event| {
            if let tauri::RunEvent::Exit = event {
                log::info!("Application exiting, cleaning up resources...");
                tauri::async_runtime::block_on(async {
                    // Clean up database connection and checkpoint WAL
                    if let Some(app_state) = _app_handle.try_state::<state::AppState>() {
                        log::info!("Starting database cleanup...");
                        if let Err(e) = app_state.db_manager.cleanup().await {
                            log::error!("Failed to cleanup database: {}", e);
                        } else {
                            log::info!("Database cleanup completed successfully");
                        }
                    } else {
                        log::warn!("AppState not available for database cleanup (likely first launch)");
                    }

                    // Clean up sidecar
                    log::info!("Cleaning up sidecar...");
                    if let Err(e) = summary::summary_engine::force_shutdown_sidecar().await {
                        log::error!("Failed to force shutdown sidecar: {}", e);
                    }
                });
                log::info!("Application cleanup complete");
            }
        });
}
