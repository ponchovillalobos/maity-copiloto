//! Detectores de señales sin LLM para disparo inteligente del coach.
//!
//! Estos detectores corren en microsegundos (regex + keyword matching).
//! El frontend los usa para decidir cuándo disparar `coach_suggest` en lugar
//! de un timer fijo cada 20s. Reduce tips genéricos de 180/hora a ~20/hora
//! estratégicos y justo-a-tiempo.
//!
//! También expuestos como comandos Tauri para que el frontend pueda usarlos
//! sin duplicar la lógica en JS.

use serde::{Deserialize, Serialize};

/// Categorías de señales detectables.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerCategory {
    Objection,
    Closing,
    Pacing,
    Rapport,
    Service,
    Discovery,
    Negotiation,
    Persuasion,
}

impl TriggerCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            TriggerCategory::Objection => "objection",
            TriggerCategory::Closing => "closing",
            TriggerCategory::Pacing => "pacing",
            TriggerCategory::Rapport => "rapport",
            TriggerCategory::Service => "service",
            TriggerCategory::Discovery => "discovery",
            TriggerCategory::Negotiation => "negotiation",
            TriggerCategory::Persuasion => "persuasion",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerSignal {
    pub category: TriggerCategory,
    /// "critical" | "important" | "soft"
    pub priority: String,
    /// Nombre de la señal detectada (ej: "price_mention", "buying_signal")
    pub signal: String,
    /// Snippet del texto que disparó la señal (max 100 chars)
    pub snippet: String,
}

/// Normaliza texto para matching: minúsculas + strip accents + sin puntuación.
/// Garantiza que "déjame" y "dejame" matchen igual.
fn normalize(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(strip_accent)
        .filter(|c| c.is_alphabetic() || c.is_whitespace() || *c == '¿' || *c == '?')
        .collect()
}

fn strip_accent(c: char) -> char {
    match c {
        'á' | 'à' | 'ä' | 'â' => 'a',
        'é' | 'è' | 'ë' | 'ê' => 'e',
        'í' | 'ì' | 'ï' | 'î' => 'i',
        'ó' | 'ò' | 'ö' | 'ô' => 'o',
        'ú' | 'ù' | 'ü' | 'û' => 'u',
        'ñ' => 'n',
        _ => c,
    }
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack.contains(n))
}

/// Detecta menciones de precio / costo.
pub fn detect_price_mention(text: &str) -> bool {
    let t = normalize(text);
    contains_any(
        &t,
        &[
            "precio",
            "cuesta",
            "costo",
            "presupuesto",
            "caro",
            "cara",
            "inversion",
            "pagar",
            "cobra",
            "cobran",
            "tarifa",
            "cotizacion",
            "descuento",
            "dolar",
            "peso",
            "euro",
            "barato",
            "barata",
            "economico",
            "economica",
        ],
    )
}

/// Detecta objeciones típicas.
pub fn detect_objection(text: &str) -> bool {
    let t = normalize(text);
    contains_any(
        &t,
        &[
            " caro",
            " cara",
            "carisimo",
            "carisima",
            "no es el momento",
            "dejame pensar",
            "dejame pensarlo",
            "tengo que pensar",
            "lo voy a pensar",
            "te aviso",
            "despues te digo",
            "no tenemos presupuesto",
            "ya tenemos",
            "ya usamos",
            "la competencia",
            "otro proveedor",
            "no estoy seguro",
            "no lo necesitamos",
            "no es prioridad",
        ],
    )
}

/// Detecta señales de compra.
pub fn detect_buying_signal(text: &str) -> bool {
    let t = normalize(text);
    contains_any(
        &t,
        &[
            "cuando empezamos",
            "cuando arrancamos",
            "cuando iniciamos",
            "cuanto tarda",
            "como implementamos",
            "como seria la implementacion",
            "cuando podemos",
            "que sigue",
            "proximo paso",
            "mi equipo lo usaria",
            "mi jefe",
            "le muestro a",
            "le voy a mostrar",
            "donde firmamos",
            "como se paga",
            "forma de pago",
            "plan de pago",
            "empezar la implementacion",
            "podriamos empezar",
            "podemos empezar",
            "podemos arrancar",
            "cuando empiezan",
            "cuando empezamos",
            "cuando comenzamos",
            "como contratamos",
            "como adquirimos",
        ],
    )
}

/// Detecta frustración / emoción negativa fuerte.
pub fn detect_frustration(text: &str) -> bool {
    let t = normalize(text);
    // Intensificadores + absolutos negativos + amenazas
    contains_any(
        &t,
        &[
            "extremadamente",
            "absolutamente",
            "increible que",
            "nunca funciona",
            "siempre pasa",
            "nadie me ayuda",
            "cancelar",
            "supervisor",
            "gerente",
            "demanda",
            "queja",
            "reclamo",
            "terrible",
            "pesimo",
            "inaceptable",
            "indignante",
            "harto",
            "harta",
            "perdida de tiempo",
        ],
    )
}

/// Detecta duda / hesitación.
pub fn detect_hesitation(text: &str) -> bool {
    let t = normalize(text);
    contains_any(
        &t,
        &[
            "tal vez",
            "quizas",
            "podria ser",
            "no lo se",
            "no se",
            "deberia",
            "podria",
            "lo reviso con",
            "lo consulto con",
            "me pongo a pensar",
        ],
    )
}

/// Detecta lenguaje posesivo (señal de cierre emocional).
pub fn detect_possessive_language(text: &str) -> bool {
    let t = normalize(text);
    contains_any(
        &t,
        &[
            "nuestra plataforma",
            "nuestro equipo",
            "nuestros clientes",
            "cuando implementemos",
            "cuando lo usemos",
            "nuestra solucion",
            "nuestra cuenta",
        ],
    )
}

/// Detecta pregunta del interlocutor (contiene ¿ o ?).
pub fn detect_question(text: &str) -> bool {
    text.contains('¿') || text.contains('?')
}

/// Detecta monólogo del usuario: más de `threshold_sec` segundos de habla continua sin turnos del otro lado.
pub fn detect_monologue(user_continuous_sec: u32) -> bool {
    user_continuous_sec > 120
}

/// Detecta satisfacción / emoción positiva del interlocutor.
pub fn detect_satisfaction(text: &str) -> bool {
    let t = normalize(text);
    contains_any(
        &t,
        &[
            "excelente",
            "perfecto",
            "me encanta",
            "impresionante",
            "impresionado",
            "impresionada",
            "genial",
            "increible",
            "fantastico",
            "fantastica",
            "maravilloso",
            "maravillosa",
            "muy bien",
            "buenisimo",
            "me gusta mucho",
            "estoy contento",
            "estoy contenta",
            "satisfecho",
            "satisfecha",
            "justo lo que necesitaba",
            "justo lo que buscaba",
        ],
    )
}

/// Detecta entusiasmo del interlocutor (exclamaciones + palabras positivas).
pub fn detect_enthusiasm(text: &str) -> bool {
    let t = normalize(text);
    let has_exclamation = text.contains('!') || text.contains('¡');
    let has_energy_word = contains_any(
        &t,
        &[
            "vamos",
            "super",
            "wow",
            "increible",
            "definitivamente",
            "sin duda",
            "por supuesto",
            "claro que si",
            "me encanta",
            "love it",
            "amazing",
            "awesome",
            "great",
            "si si",
        ],
    );
    (has_exclamation && has_energy_word) || contains_any(&t, &["definitivamente si", "por supuesto que si", "claro que si"])
}

/// Detecta talk ratio alto: usuario habla >60% del tiempo.
pub fn detect_high_talk_ratio(ratio: f32) -> bool {
    ratio > 0.60
}

/// Detecta talk ratio bajo (usuario no participa).
pub fn detect_low_talk_ratio(ratio: f32) -> bool {
    ratio < 0.25
}

// ============================================================================
// DETECTORES DE COMUNICACIÓN PERSONAL (plan V3.1 — solo USER)
// ============================================================================
//
// Los 6 detectores siguientes observan la CALIDAD DE COMUNICACIÓN del usuario
// (muletillas, preguntas, validación, ritmo, espirales negativas, empatía).
// Se usan en `analyze_turn` con `is_interlocutor=false` para alimentar tips
// de mejora continua con tono empático ("con cariño").

/// Cuenta muletillas comunes en español en el texto del USER.
/// Retorna el conteo total.
///
/// Optimizado: normaliza y padea el texto UNA sola vez fuera del loop
/// (antes: 26 allocs con `format!()` por turno).
pub fn count_filler_words(text: &str) -> usize {
    let norm = normalize(text);
    let padded = format!(" {} ", norm);
    const FILLERS: &[&str] = &[
        " eh ", " ehh ", " este ", " o sea ", " osea ", " pues ", " tipo ",
        " basicamente ", " digamos ", " como que ", " verdad ", " nomas ",
    ];
    let mut count = 0;
    for filler in FILLERS {
        count += padded.matches(filler).count();
    }
    count
}

/// Devuelve `true` si en el último turno del USER hay ≥ 4 muletillas.
pub fn detect_filler_words(text: &str) -> bool {
    count_filler_words(text) >= 4
}

/// Devuelve true si el USER acumuló ≥ `min_user_words` palabras sin formular
/// una sola pregunta. Usado para detectar "sequía de preguntas" (discovery débil).
///
/// `user_text_last_90s`: texto concatenado del USER en los últimos ~90 segundos.
/// `interlocutor_had_monologue`: si el interlocutor soltó ≥ 30s de habla continua.
pub fn detect_question_drought(
    user_text_last_90s: &str,
    interlocutor_had_monologue: bool,
) -> bool {
    let word_count = user_text_last_90s.split_whitespace().count();
    let has_question = user_text_last_90s.contains('?') || user_text_last_90s.contains('¿');
    word_count >= 120 && !has_question && interlocutor_had_monologue
}

/// Detecta que el USER NO validó al cliente después de un turno emocional.
///
/// `user_response`: lo que el USER dijo justo después del turno del interlocutor.
/// `interlocutor_was_emotional`: si el turno previo del interlocutor mostró frustración / emoción.
pub fn detect_missing_validation(user_response: &str, interlocutor_was_emotional: bool) -> bool {
    if !interlocutor_was_emotional {
        return false;
    }
    let norm = normalize(user_response);
    const VALIDATION_WORDS: &[&str] = &[
        "entiendo",
        "comprendo",
        "valido",
        "escucho",
        "veo que",
        "tiene razon",
        "tienes razon",
        "lamento",
        "siento que",
        "es valido",
    ];
    !contains_any(&norm, VALIDATION_WORDS)
}

/// Detecta habla rápida / atropellada: >50 palabras con muy poca puntuación interna.
/// Heurística: menos de 1 signo de puntuación por cada 15 palabras.
pub fn detect_rapid_fire(text: &str) -> bool {
    let words = text.split_whitespace().count();
    if words < 50 {
        return false;
    }
    let punct_count = text
        .chars()
        .filter(|c| matches!(c, '.' | ',' | ';' | ':' | '!' | '?'))
        .count();
    // 1 puntuación por cada 15 palabras mínimo → si hay menos, es atropellado
    let expected = words / 15;
    punct_count < expected.max(1)
}

/// Detecta "espiral negativa": 3+ frases de cierre de puertas del USER en poco tiempo.
///
/// `user_text_last_60s`: texto concatenado del USER en los últimos 60s.
pub fn detect_negative_spiral(user_text_last_60s: &str) -> bool {
    let norm = normalize(user_text_last_60s);
    const CLOSING_PHRASES: &[&str] = &[
        "no puedo",
        "imposible",
        "no se puede",
        "es la politica",
        "es politica",
        "no hay forma",
        "no hay manera",
        "no permitido",
        "eso no se",
    ];
    let mut count = 0;
    for phrase in CLOSING_PHRASES {
        count += norm.matches(phrase).count();
    }
    count >= 3
}

/// Detecta empathy gap: cliente emocional + USER responde sin palabras empáticas.
///
/// `user_response`: turno del USER justo después del cliente emocional.
pub fn detect_empathy_gap(user_response: &str, interlocutor_was_frustrated: bool) -> bool {
    if !interlocutor_was_frustrated {
        return false;
    }
    let norm = normalize(user_response);
    const EMPATHY_WORDS: &[&str] = &[
        "entiendo",
        "lamento",
        "siento",
        "tiene razon",
        "tienes razon",
        "es valido",
        "comprendo",
        "escucho",
    ];
    !contains_any(&norm, EMPATHY_WORDS) && user_response.split_whitespace().count() >= 8
}

/// Analiza un turno de transcripción y devuelve todas las señales detectadas,
/// ordenadas por prioridad (critical primero).
///
/// `is_interlocutor` = true si el texto vino del interlocutor (otro speaker).
///
/// Genera señales con semántica diferente según la atribución:
/// - Interlocutor (cliente): señales de oportunidad y venta
/// - Usuario (vendedor): señales de autorregulación y control
pub fn analyze_turn(text: &str, is_interlocutor: bool) -> Vec<TriggerSignal> {
    let mut signals = Vec::new();
    let snippet: String = text.chars().take(100).collect();

    // Frustración: cliente frustrado vs usuario perdiendo control
    if detect_frustration(text) {
        signals.push(TriggerSignal {
            category: TriggerCategory::Service,
            priority: "critical".into(),
            signal: if is_interlocutor {
                "client_frustrated".into()
            } else {
                "user_losing_control".into()
            },
            snippet: snippet.clone(),
        });
    }

    // Señal de compra: oportunidad de cierre vs usuario siendo demasiado presuntivo
    if detect_buying_signal(text) {
        signals.push(TriggerSignal {
            category: TriggerCategory::Closing,
            priority: "critical".into(),
            signal: if is_interlocutor {
                "buying_signal".into()
            } else {
                "user_assumptive_close".into()
            },
            snippet: snippet.clone(),
        });
    }

    // Objeciones: cliente objeta vs usuario siendo preemptivo
    if detect_objection(text) {
        signals.push(TriggerSignal {
            category: TriggerCategory::Objection,
            priority: "critical".into(),
            signal: if is_interlocutor {
                "client_objection".into()
            } else {
                "user_preemptive_objection".into()
            },
            snippet: snippet.clone(),
        });
    }

    // Lenguaje posesivo: señal de cierre
    if detect_possessive_language(text) {
        signals.push(TriggerSignal {
            category: TriggerCategory::Closing,
            priority: "important".into(),
            signal: if is_interlocutor {
                "client_possessive".into()
            } else {
                "user_possessive".into()
            },
            snippet: snippet.clone(),
        });
    }

    // Mención de precio
    if detect_price_mention(text) && !detect_objection(text) {
        signals.push(TriggerSignal {
            category: TriggerCategory::Negotiation,
            priority: "important".into(),
            signal: if is_interlocutor {
                "client_asked_price".into()
            } else {
                "user_mentioned_price".into()
            },
            snippet: snippet.clone(),
        });
    }

    // Preguntas: oportunidad de Rapport para ambos
    if detect_question(text) {
        signals.push(TriggerSignal {
            category: TriggerCategory::Rapport,
            priority: "soft".into(),
            signal: "question_detected".into(),
            snippet: snippet.clone(),
        });
    }

    // Hesitación: cliente duda vs usuario uncertain
    if detect_hesitation(text) {
        signals.push(TriggerSignal {
            category: TriggerCategory::Persuasion,
            priority: "important".into(),
            signal: if is_interlocutor {
                "client_hesitating".into()
            } else {
                "user_uncertain".into()
            },
            snippet: snippet.clone(),
        });
    }

    // Satisfacción: cliente satisfecho vs usuario enthusiastic
    if detect_satisfaction(text) {
        signals.push(TriggerSignal {
            category: TriggerCategory::Closing,
            priority: "important".into(),
            signal: if is_interlocutor {
                "client_satisfied".into()
            } else {
                "user_enthusiastic".into()
            },
            snippet: snippet.clone(),
        });
    }

    // Entusiasmo: cliente entusiasta vs usuario over-excited
    if detect_enthusiasm(text) {
        signals.push(TriggerSignal {
            category: TriggerCategory::Closing,
            priority: "important".into(),
            signal: if is_interlocutor {
                "client_enthusiastic".into()
            } else {
                "user_over_excited".into()
            },
            snippet: snippet.clone(),
        });
    }

    // === Comunicación personal (solo USER) — V3.1 ===
    if !is_interlocutor {
        // Muletillas (fillers)
        if detect_filler_words(text) {
            signals.push(TriggerSignal {
                category: TriggerCategory::Pacing,
                priority: "important".into(),
                signal: "user_verbal_fillers".into(),
                snippet: snippet.clone(),
            });
        }

        // Ritmo atropellado
        if detect_rapid_fire(text) {
            signals.push(TriggerSignal {
                category: TriggerCategory::Pacing,
                priority: "soft".into(),
                signal: "user_rapid_fire_pace".into(),
                snippet: snippet.clone(),
            });
        }
    }

    let _ = snippet; // silence unused warning if last branch no lo usa

    // Ordenar por prioridad
    signals.sort_by_key(|s| match s.priority.as_str() {
        "critical" => 0,
        "important" => 1,
        _ => 2,
    });
    signals
}

/// Contexto extendido para detectores multi-turno (V3.1).
///
/// Los 3 detectores siguientes NO pueden decidir con solo `text`: necesitan saber
/// qué hizo el interlocutor en el turno anterior. `analyze_turn_with_context`
/// llama a `analyze_turn` y añade señales contextuales cuando aplican.
#[derive(Debug, Clone, Default)]
pub struct TurnContext<'a> {
    /// Texto concatenado del USER en los últimos ~90s (para detectar question drought).
    pub user_text_last_90s: &'a str,
    /// Texto concatenado del USER en los últimos ~60s (para detectar negative spiral).
    pub user_text_last_60s: &'a str,
    /// True si el interlocutor tuvo un monólogo >=30s justo antes.
    pub interlocutor_had_monologue: bool,
    /// True si el interlocutor mostró emoción fuerte (frustración) en turno previo.
    pub interlocutor_was_emotional: bool,
}

/// Analiza un turno con contexto de turnos previos. Usa `analyze_turn` como base
/// y añade señales que requieren historia multi-turno.
pub fn analyze_turn_with_context(
    text: &str,
    is_interlocutor: bool,
    ctx: &TurnContext<'_>,
) -> Vec<TriggerSignal> {
    let mut signals = analyze_turn(text, is_interlocutor);
    let snippet: String = text.chars().take(100).collect();

    if !is_interlocutor {
        if detect_question_drought(ctx.user_text_last_90s, ctx.interlocutor_had_monologue) {
            signals.push(TriggerSignal {
                category: TriggerCategory::Discovery,
                priority: "important".into(),
                signal: "user_not_asking_questions".into(),
                snippet: snippet.clone(),
            });
        }

        if detect_missing_validation(text, ctx.interlocutor_was_emotional) {
            signals.push(TriggerSignal {
                category: TriggerCategory::Service,
                priority: "important".into(),
                signal: "user_missing_validation".into(),
                snippet: snippet.clone(),
            });
        }

        if detect_negative_spiral(ctx.user_text_last_60s) {
            signals.push(TriggerSignal {
                category: TriggerCategory::Service,
                priority: "critical".into(),
                signal: "user_closing_doors".into(),
                snippet: snippet.clone(),
            });
        }

        if detect_empathy_gap(text, ctx.interlocutor_was_emotional) {
            signals.push(TriggerSignal {
                category: TriggerCategory::Service,
                priority: "critical".into(),
                signal: "user_empathy_gap".into(),
                snippet,
            });
        }
    }

    // Re-ordenar tras añadir señales
    signals.sort_by_key(|s| match s.priority.as_str() {
        "critical" => 0,
        "important" => 1,
        _ => 2,
    });

    signals
}

/// Comando Tauri: analiza un texto y devuelve las señales detectadas.
#[tauri::command]
pub fn coach_analyze_trigger(text: String, is_interlocutor: bool) -> Vec<TriggerSignal> {
    analyze_turn(&text, is_interlocutor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_price() {
        assert!(detect_price_mention("el precio es alto"));
        assert!(detect_price_mention("es muy caro"));
        assert!(detect_price_mention("no tengo presupuesto"));
        assert!(!detect_price_mention("me encanta el producto"));
    }

    #[test]
    fn test_detect_objection() {
        assert!(detect_objection("es caro para nosotros"));
        assert!(detect_objection("déjame pensarlo"));
        assert!(detect_objection("ya tenemos otro proveedor"));
        assert!(!detect_objection("cuéntame más"));
    }

    #[test]
    fn test_detect_buying_signal() {
        assert!(detect_buying_signal("¿cuándo empezamos?"));
        assert!(detect_buying_signal("cómo implementamos esto"));
        assert!(detect_buying_signal("le voy a mostrar a mi jefe"));
        assert!(!detect_buying_signal("no me interesa"));
    }

    #[test]
    fn test_detect_frustration() {
        assert!(detect_frustration("esto es absolutamente inaceptable"));
        assert!(detect_frustration("quiero hablar con un supervisor"));
        assert!(detect_frustration("nunca funciona nada"));
        assert!(!detect_frustration("todo bien hasta ahora"));
    }

    #[test]
    fn test_detect_possessive() {
        assert!(detect_possessive_language("cuando implementemos esto"));
        assert!(detect_possessive_language("nuestra plataforma sería genial"));
        assert!(!detect_possessive_language("su plataforma es interesante"));
    }

    #[test]
    fn test_detect_question() {
        assert!(detect_question("¿qué piensas?"));
        assert!(detect_question("pero por qué?"));
        assert!(!detect_question("no me interesa."));
    }

    #[test]
    fn test_detect_monologue() {
        assert!(detect_monologue(150));
        assert!(!detect_monologue(60));
    }

    #[test]
    fn test_detect_talk_ratio() {
        assert!(detect_high_talk_ratio(0.75));
        assert!(!detect_high_talk_ratio(0.50));
        assert!(detect_low_talk_ratio(0.15));
        assert!(!detect_low_talk_ratio(0.40));
    }

    #[test]
    fn test_analyze_turn_buying_signal_from_interlocutor() {
        let signals = analyze_turn("cuándo arrancamos con esto", true);
        assert!(signals.iter().any(|s| s.signal == "buying_signal"));
    }

    #[test]
    fn test_analyze_turn_buying_signal_from_user() {
        let signals = analyze_turn("cuándo arrancamos con esto", false);
        assert!(signals.iter().any(|s| s.signal == "user_assumptive_close"));
    }

    #[test]
    fn test_analyze_turn_objection_critical() {
        let signals = analyze_turn("es muy caro para nosotros", true);
        let first = signals.first().unwrap();
        assert_eq!(first.priority, "critical");
        assert_eq!(first.signal, "client_objection");
    }

    #[test]
    fn test_analyze_turn_objection_from_user() {
        let signals = analyze_turn("es muy caro para nosotros", false);
        let first = signals.first().unwrap();
        assert_eq!(first.signal, "user_preemptive_objection");
    }

    #[test]
    fn test_analyze_turn_frustration() {
        let signals = analyze_turn("esto es absolutamente terrible, quiero un supervisor", true);
        assert_eq!(signals.first().unwrap().signal, "client_frustrated");
    }

    #[test]
    fn test_analyze_turn_frustration_from_user() {
        let signals = analyze_turn("esto es absolutamente terrible, quiero un supervisor", false);
        assert_eq!(signals.first().unwrap().signal, "user_losing_control");
    }

    #[test]
    fn test_analyze_turn_empty() {
        assert!(analyze_turn("hola", true).is_empty());
    }

    #[test]
    fn test_analyze_turn_sorted_by_priority() {
        // frustration (critical) + question (soft) en el mismo texto
        let signals = analyze_turn("¿por qué nunca funciona esto? quiero un supervisor", true);
        assert_eq!(signals.first().unwrap().priority, "critical");
    }

    #[test]
    fn test_detect_satisfaction() {
        assert!(detect_satisfaction("esto es excelente, me encanta"));
        assert!(detect_satisfaction("perfecto, justo lo que necesitaba"));
        assert!(detect_satisfaction("estoy impresionado con los resultados"));
        assert!(!detect_satisfaction("está bien, nada especial"));
    }

    #[test]
    fn test_detect_enthusiasm() {
        assert!(detect_enthusiasm("¡Definitivamente sí!"));
        assert!(detect_enthusiasm("¡Me encanta, vamos!"));
        assert!(!detect_enthusiasm("sí, está bien")); // no exclamation
        assert!(!detect_enthusiasm("ok")); // no energy word
    }

    #[test]
    fn test_analyze_turn_satisfaction_signal() {
        let signals = analyze_turn("excelente, me encanta esta propuesta", true);
        assert!(signals.iter().any(|s| s.signal == "client_satisfied"));
    }

    #[test]
    fn test_analyze_turn_satisfaction_from_user() {
        let signals = analyze_turn("excelente, me encanta esta propuesta", false);
        assert!(signals.iter().any(|s| s.signal == "user_enthusiastic"));
    }

    // ========================================================================
    // Tests de los 6 detectores de comunicación personal (V3.1)
    // ========================================================================

    #[test]
    fn test_count_filler_words_simple() {
        assert_eq!(count_filler_words("hola eh este pues o sea me explico"), 4);
    }

    #[test]
    fn test_count_filler_words_ignora_ehcomo_embebido() {
        // "este" como palabra suelta cuenta, dentro de otra no (normalize filtra puntuación).
        // "esta" NO debe contar como "este".
        assert_eq!(count_filler_words("esta es la propuesta"), 0);
    }

    #[test]
    fn test_detect_filler_words_umbral_4() {
        assert!(detect_filler_words("eh este o sea pues digamos tipo"));
        assert!(!detect_filler_words("eh este"));
    }

    #[test]
    fn test_detect_filler_words_triggered_by_user_turn() {
        let s = analyze_turn("eh este o sea pues la verdad es que digamos esto es bueno", false);
        assert!(s.iter().any(|x| x.signal == "user_verbal_fillers"));
    }

    #[test]
    fn test_detect_filler_words_not_triggered_by_interlocutor() {
        // Los fillers del interlocutor no nos importan — no coacheamos al cliente.
        let s = analyze_turn("eh este o sea pues digamos", true);
        assert!(!s.iter().any(|x| x.signal == "user_verbal_fillers"));
    }

    #[test]
    fn test_detect_rapid_fire_positive() {
        // 70+ palabras sin ningún signo de puntuación interna
        let text = "tenemos una solucion integral que ayuda a las empresas a mejorar sus procesos \
                    de ventas con inteligencia artificial y analiticas avanzadas para que puedan \
                    tomar mejores decisiones en tiempo real usando datos de multiples fuentes y \
                    ademas ofrecemos soporte premium con capacitacion inicial para todos los \
                    miembros del equipo sin costo adicional en los planes enterprise";
        assert!(detect_rapid_fire(text));
    }

    #[test]
    fn test_detect_rapid_fire_negative_pocas_palabras() {
        assert!(!detect_rapid_fire("hola, cómo estás"));
    }

    #[test]
    fn test_detect_rapid_fire_negative_con_puntuacion() {
        let text = "tenemos una solucion. Es integral. Ayuda mucho. Mejora procesos. \
                    Usa IA. Las empresas ven resultados. Los equipos se adaptan. Todo funciona.";
        assert!(!detect_rapid_fire(text));
    }

    #[test]
    fn test_detect_question_drought_triggers() {
        // 120+ palabras sin ni un signo de interrogación
        let text = "bueno nosotros tenemos una solucion que cubre todo lo que necesitas \
                    es muy completa y tiene integraciones con varios sistemas \
                    ademas ofrecemos soporte premium para todos los planes \
                    y tenemos casos de exito en tu industria especifica \
                    el precio es competitivo frente a la competencia existente \
                    y la implementacion toma solo dos semanas tipicamente para empresas \
                    con un equipo tecnico normal como el tuyo \
                    tambien nos adaptamos a procesos existentes sin disrupcion \
                    y proporcionamos reportes semanales de progreso \
                    con nuestro equipo de customer success dedicado para cada cuenta \
                    que puede acompañar la adopcion de manera cercana durante todo el onboarding \
                    y la post implementacion hasta que el equipo se sienta totalmente autonomo \
                    ademas tenemos dashboards de analitica que permiten ver KPIs en tiempo real \
                    y el ROI suele ser positivo desde el primer trimestre de uso";
        assert!(detect_question_drought(text, true));
    }

    #[test]
    fn test_detect_question_drought_no_si_hay_pregunta() {
        let text = "tenemos una solucion. ¿qué retos enfrentas hoy? podemos ayudar.";
        assert!(!detect_question_drought(text, true));
    }

    #[test]
    fn test_detect_missing_validation_positive() {
        assert!(detect_missing_validation("bueno, lo que pasa es que el producto es así", true));
    }

    #[test]
    fn test_detect_missing_validation_negative_con_empatia() {
        assert!(!detect_missing_validation("entiendo tu frustración, déjame ayudarte", true));
        assert!(!detect_missing_validation("veo que esto te preocupa", true));
    }

    #[test]
    fn test_detect_missing_validation_no_aplica_si_no_emocional() {
        assert!(!detect_missing_validation("bueno claro", false));
    }

    #[test]
    fn test_detect_negative_spiral_triple() {
        let text = "no puedo hacer eso. imposible por la politica. no se puede sinceramente.";
        assert!(detect_negative_spiral(text));
    }

    #[test]
    fn test_detect_negative_spiral_solo_dos() {
        let text = "no puedo hacer eso. pero veamos que si puedo. imposible solo algunas cosas.";
        assert!(!detect_negative_spiral(text));
    }

    #[test]
    fn test_detect_empathy_gap_positive() {
        // cliente frustrado + user responde técnico sin empatía
        let user = "bueno el sistema funciona asi porque la base de datos requiere ese formato";
        assert!(detect_empathy_gap(user, true));
    }

    #[test]
    fn test_detect_empathy_gap_negative_con_empatia() {
        let user = "entiendo tu frustracion, dejame revisar qué opciones tenemos";
        assert!(!detect_empathy_gap(user, true));
    }

    #[test]
    fn test_detect_empathy_gap_negative_sin_emocion_cliente() {
        assert!(!detect_empathy_gap("el sistema funciona asi", false));
    }

    #[test]
    fn test_analyze_turn_with_context_integration() {
        let ctx = TurnContext {
            user_text_last_90s: "bueno tenemos muchas cosas por decirte aqui sobre la solucion \
                                  que ofrecemos y los planes disponibles para empresas grandes \
                                  con soporte premium y capacitacion inicial para el equipo \
                                  ademas del roadmap de producto para los proximos trimestres \
                                  vamos a invertir mucho en nuevas features para clientes enterprise \
                                  nuestra plataforma integra con los sistemas que ya usan los equipos \
                                  y el onboarding suele ser rapido gracias al diseño intuitivo \
                                  ademas ofrecemos servicios de migracion sin costo extra \
                                  y garantizamos un roi positivo en el primer trimestre de uso \
                                  con metricas claras que mostramos en reportes semanales \
                                  para que los stakeholders vean el progreso de manera transparente \
                                  tambien tenemos un customer success dedicado para cada cuenta \
                                  y los dashboards son totalmente configurables por rol \
                                  ademas publicamos casos de estudio cada mes con resultados \
                                  y el producto tiene una uptime historico del noventa y nueve punto nueve \
                                  tambien ofrecemos capacitacion continua y webinars mensuales \
                                  para que el equipo se mantenga al dia con las nuevas funcionalidades",
            user_text_last_60s: "no puedo hacer eso. imposible por politica. no se puede.",
            interlocutor_had_monologue: true,
            interlocutor_was_emotional: true,
        };
        // Texto actual del user: sin empatía, sin preguntas
        let signals = analyze_turn_with_context(
            "el sistema funciona así y no hay manera de cambiarlo es la politica vigente",
            false,
            &ctx,
        );
        // Debe disparar al menos: question_drought, negative_spiral, closing_doors
        let names: Vec<_> = signals.iter().map(|s| s.signal.as_str()).collect();
        assert!(names.contains(&"user_not_asking_questions"));
        assert!(names.contains(&"user_closing_doors"));
    }
}
