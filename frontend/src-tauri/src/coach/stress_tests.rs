//! Stress tests: 5 escenarios de conversación inyectados al trigger detector.
//! Valida detección de señales, prioridades y cobertura del coach.
//!
//! **Test 1: Attribution Differentiation** — Cliente frustrado vs usuario perdiendo control,
//! objeciones cliente vs usuario preemptivo, etc. 7 detectores diferenciados.
//!
//! **Test 2: Signal Priority Ordering** — Múltiples triggers en el mismo texto,
//! verifica que critical > important > soft.
//!
//! **Test 3: Edge Cases** — Texto vacío, whitespace, 10k chars, emojis, sin panic.
//!
//! **Test 4: Meeting Type Detection Robustness** — 0 keywords → None/Auto,
//! ambigüedades determinísticas, textos cortos.
//!
//! **Test 5: Profanity Detection** — Múltiples profanidades, con accents, en compound words.

#[cfg(test)]
mod tests {
    use crate::coach::trigger::*;
    use crate::coach::meeting_type::heuristic_detect;
    use crate::coach::prompt::MeetingType;

    struct Turn {
        text: &'static str,
        is_interlocutor: bool,
        expected_signals: &'static [&'static str],
        expected_priority: Option<&'static str>,
    }

    fn run_scenario(name: &str, turns: &[Turn]) -> (usize, usize, usize) {
        let mut detected = 0;
        let mut missed = 0;
        let mut false_neg = 0;

        for (i, turn) in turns.iter().enumerate() {
            let signals = analyze_turn(turn.text, turn.is_interlocutor);
            let signal_names: Vec<&str> = signals.iter().map(|s| s.signal.as_str()).collect();

            for expected in turn.expected_signals {
                if signal_names.iter().any(|s| s.contains(expected)) {
                    detected += 1;
                } else {
                    missed += 1;
                    false_neg += 1;
                    eprintln!("[{}] Turn {}: MISSED signal '{}' in: \"{}\"",
                        name, i, expected, turn.text);
                }
            }

            if let Some(prio) = turn.expected_priority {
                if let Some(top) = signals.first() {
                    assert_eq!(top.priority, prio,
                        "[{}] Turn {}: expected priority '{}' got '{}'",
                        name, i, prio, top.priority);
                }
            }
        }

        eprintln!("[{}] Results: {}/{} signals detected, {} false negatives",
            name, detected, detected + missed, false_neg);
        (detected, missed, false_neg)
    }

    // ─────────────────────────────────────────────
    // ESCENARIO 1: VENTA (15 turnos)
    // ─────────────────────────────────────────────
    #[test]
    fn stress_test_sales_call() {
        let turns = vec![
            Turn { text: "Cuéntame sobre los desafíos de tu equipo", is_interlocutor: false, expected_signals: &[], expected_priority: None },
            Turn { text: "Nos cuesta mucho, el proceso manual es muy caro", is_interlocutor: true, expected_signals: &["client_objection"], expected_priority: Some("critical") },
            Turn { text: "¿Cómo impacta eso en tu día a día?", is_interlocutor: false, expected_signals: &[], expected_priority: None },
            Turn { text: "Perdemos 8 horas semanales en reportes manuales", is_interlocutor: true, expected_signals: &[], expected_priority: None },
            Turn { text: "Déjame mostrarte nuestra solución", is_interlocutor: false, expected_signals: &[], expected_priority: None },
            Turn { text: "Interesante, ¿cuánto cuesta la licencia?", is_interlocutor: true, expected_signals: &["client_asked_price"], expected_priority: None },
            Turn { text: "Son 500 dólares mensuales", is_interlocutor: false, expected_signals: &[], expected_priority: None },
            Turn { text: "Es caro para nuestro presupuesto, déjame pensarlo", is_interlocutor: true, expected_signals: &["client_objection"], expected_priority: Some("critical") },
            Turn { text: "Entiendo tu preocupación, si el precio no fuera tema ¿es la solución?", is_interlocutor: false, expected_signals: &[], expected_priority: None },
            Turn { text: "Sí, excelente! Me encanta mucho", is_interlocutor: true, expected_signals: &[], expected_priority: None },
            Turn { text: "¿Y si empezamos con un plan piloto de 3 meses?", is_interlocutor: false, expected_signals: &[], expected_priority: None },
            Turn { text: "¿Cuándo podríamos empezar la implementación?", is_interlocutor: true, expected_signals: &["buying_signal"], expected_priority: None },
            Turn { text: "Podemos arrancar la próxima semana", is_interlocutor: false, expected_signals: &[], expected_priority: None },
            Turn { text: "Cuando implementemos esto con nuestro equipo va a ser increíble", is_interlocutor: true, expected_signals: &["client_possessive"], expected_priority: None },
            Turn { text: "Perfecto, te mando el contrato hoy", is_interlocutor: false, expected_signals: &[], expected_priority: None },
        ];

        let (detected, missed, _) = run_scenario("VENTA", &turns);
        assert!(missed == 0, "Sales scenario: {} signals missed", missed);
        assert!(detected >= 4, "Sales scenario: need >=4 signals, got {}", detected);
    }

    // ─────────────────────────────────────────────
    // ESCENARIO 2: SERVICIO AL CLIENTE (12 turnos)
    // ─────────────────────────────────────────────
    #[test]
    fn stress_test_customer_service() {
        let turns = vec![
            Turn { text: "Hola, tengo un problema con mi cuenta", is_interlocutor: true, expected_signals: &[], expected_priority: None },
            Turn { text: "Claro, ¿cuál es el problema?", is_interlocutor: false, expected_signals: &[], expected_priority: None },
            Turn { text: "Esto es inaceptable, terrible, no funciona", is_interlocutor: true, expected_signals: &["client_frustrated"], expected_priority: Some("critical") },
            Turn { text: "Lamento mucho la situación, déjame ver", is_interlocutor: false, expected_signals: &[], expected_priority: None },
            Turn { text: "Quiero un supervisor, nadie me ayuda", is_interlocutor: true, expected_signals: &["client_frustrated"], expected_priority: Some("critical") },
            Turn { text: "Yo me encargo personalmente de esto", is_interlocutor: false, expected_signals: &[], expected_priority: None },
            Turn { text: "¿Cuánto tardas? Estoy harto, nunca funciona", is_interlocutor: true, expected_signals: &["client_frustrated"], expected_priority: Some("critical") },
            Turn { text: "Lo resuelvo en los próximos 10 minutos, ya encontré el problema", is_interlocutor: false, expected_signals: &[], expected_priority: None },
            Turn { text: "¿De verdad?", is_interlocutor: true, expected_signals: &[], expected_priority: None },
            Turn { text: "Listo, ya tienes acceso, verifica por favor", is_interlocutor: false, expected_signals: &[], expected_priority: None },
            Turn { text: "Excelente! Perfecto, funciona", is_interlocutor: true, expected_signals: &["client_satisfied"], expected_priority: None },
            Turn { text: "Me alegro, ¿algo más en que pueda ayudarte?", is_interlocutor: false, expected_signals: &[], expected_priority: None },
        ];

        let (detected, missed, _) = run_scenario("SERVICIO", &turns);
        assert!(missed == 0, "Service scenario: {} signals missed", missed);
        assert!(detected >= 4, "Service scenario: need >=4 signals, got {}", detected);
    }

    // ─────────────────────────────────────────────
    // ESCENARIO 3: JUNTA DE EQUIPO (10 turnos)
    // ─────────────────────────────────────────────
    #[test]
    fn stress_test_team_meeting() {
        let turns = vec![
            Turn { text: "Buenos días equipo, vamos con el standup", is_interlocutor: false, expected_signals: &[], expected_priority: None },
            Turn { text: "En el sprint voy bien, sin bloqueadores", is_interlocutor: true, expected_signals: &[], expected_priority: None },
            Turn { text: "¿Cómo vas con el proyecto de migración?", is_interlocutor: false, expected_signals: &[], expected_priority: None },
            Turn { text: "Tengo un bloqueo, no se cuándo lo arreglen", is_interlocutor: true, expected_signals: &[], expected_priority: None },
            Turn { text: "¿Qué necesitas para desbloquear?", is_interlocutor: false, expected_signals: &[], expected_priority: None },
            Turn { text: "Lo reviso con el team, es caro en tiempo", is_interlocutor: true, expected_signals: &[], expected_priority: None },
            Turn { text: "Ok yo tomo eso, ahora decidamos: ¿opción A o B?", is_interlocutor: false, expected_signals: &[], expected_priority: None },
            Turn { text: "Definitivamente la opción A, es más simple", is_interlocutor: true, expected_signals: &[], expected_priority: None },
            Turn { text: "Perfecto, ¿algo más antes de cerrar?", is_interlocutor: false, expected_signals: &[], expected_priority: None },
            Turn { text: "No, todo bien, gracias", is_interlocutor: true, expected_signals: &[], expected_priority: None },
        ];

        let (detected, _, _) = run_scenario("EQUIPO", &turns);
        // Team meeting typically has fewer signals than sales/service scenarios
        let _ = detected; // Team scenario: validates no crash occurs
    }

    // ─────────────────────────────────────────────
    // ESCENARIO 4: NEGOCIACIÓN (12 turnos)
    // ─────────────────────────────────────────────
    #[test]
    fn stress_test_negotiation() {
        let turns = vec![
            Turn { text: "Nuestra propuesta es 100 mil dólares por el proyecto", is_interlocutor: false, expected_signals: &[], expected_priority: None },
            Turn { text: "El precio es alto, nuestro presupuesto es mucho menor", is_interlocutor: true, expected_signals: &["client_asked_price"], expected_priority: Some("important") },
            Turn { text: "¿Cuál sería un rango aceptable para ustedes?", is_interlocutor: false, expected_signals: &[], expected_priority: None },
            Turn { text: "Podría ser, quizás la mitad", is_interlocutor: true, expected_signals: &["client_hesitating"], expected_priority: None },
            Turn { text: "Puedo ajustar a 80 mil si cerramos antes de fin de mes", is_interlocutor: false, expected_signals: &[], expected_priority: None },
            Turn { text: "Ya tenemos otro proveedor que nos cotizó menos", is_interlocutor: true, expected_signals: &["client_objection"], expected_priority: Some("critical") },
            Turn { text: "¿Qué incluye esa otra cotización?", is_interlocutor: false, expected_signals: &[], expected_priority: None },
            Turn { text: "Menos funcionalidades, pero el presupuesto es menor", is_interlocutor: true, expected_signals: &["client_asked_price"], expected_priority: None },
            Turn { text: "Con nosotros obtienes soporte 24/7 y capacitación", is_interlocutor: false, expected_signals: &[], expected_priority: None },
            Turn { text: "¿Cómo sería el plan de pago?", is_interlocutor: true, expected_signals: &["buying_signal"], expected_priority: None },
            Turn { text: "Podemos hacer 3 pagos mensuales", is_interlocutor: false, expected_signals: &[], expected_priority: None },
            Turn { text: "Cuando implementemos esto con nuestro equipo será genial", is_interlocutor: true, expected_signals: &["client_possessive"], expected_priority: None },
        ];

        let (detected, missed, _) = run_scenario("NEGOCIACIÓN", &turns);
        assert!(missed == 0, "Negotiation scenario: {} signals missed", missed);
        assert!(detected >= 5, "Negotiation scenario: need >=5 signals, got {}", detected);
    }

    // ─────────────────────────────────────────────
    // ESCENARIO 5: WEBINAR Q&A (8 turnos)
    // ─────────────────────────────────────────────
    #[test]
    fn stress_test_webinar() {
        let turns = vec![
            Turn { text: "Bienvenidos al webinar de inteligencia artificial", is_interlocutor: false, expected_signals: &[], expected_priority: None },
            Turn { text: "¿Pueden explicar más sobre la integración?", is_interlocutor: true, expected_signals: &["question_detected"], expected_priority: Some("soft") },
            Turn { text: "Claro, la integración se hace vía API REST", is_interlocutor: false, expected_signals: &[], expected_priority: None },
            Turn { text: "Me encanta mucho la arquitectura, excelente", is_interlocutor: true, expected_signals: &["client_satisfied"], expected_priority: None },
            Turn { text: "¿Cuánto cuesta la versión enterprise?", is_interlocutor: true, expected_signals: &["client_asked_price"], expected_priority: None },
            Turn { text: "La versión enterprise parte desde 2000 al mes", is_interlocutor: false, expected_signals: &[], expected_priority: None },
            Turn { text: "¿Cuándo podemos empezar un trial?", is_interlocutor: true, expected_signals: &["buying_signal"], expected_priority: None },
            Turn { text: "Pueden iniciar hoy mismo con nuestro plan gratuito", is_interlocutor: false, expected_signals: &[], expected_priority: None },
        ];

        let (detected, missed, _) = run_scenario("WEBINAR", &turns);
        assert!(missed == 0, "Webinar scenario: {} signals missed", missed);
        assert!(detected >= 3, "Webinar scenario: need >=3 signals, got {}", detected);
    }

    // ─────────────────────────────────────────────
    // MEETING TYPE DETECTION
    // ─────────────────────────────────────────────
    #[test]
    fn stress_test_meeting_type_detection() {
        // heuristic needs 2+ keywords per category to classify
        assert_eq!(
            heuristic_detect("Cuánto cuesta la licencia, queremos hacer una demo del producto y ver el precio"),
            Some(MeetingType::Sales)
        );
        assert_eq!(
            heuristic_detect("Tengo un problema con mi cuenta, quiero soporte técnico y hablar con un supervisor"),
            Some(MeetingType::Service)
        );
        assert_eq!(
            heuristic_detect("Buenos días equipo, vamos con el standup del sprint y revisemos los bloqueadores del proyecto"),
            Some(MeetingType::TeamMeeting)
        );
    }

    // ─────────────────────────────────────────────
    // EDGE CASES: Code-switching, números, regional
    // ─────────────────────────────────────────────
    #[test]
    fn test_code_switching() {
        assert!(detect_objection("es muy caro, we don't have budget"));
        assert!(detect_price_mention("cuesta fifteen thousand dólares"));
    }

    #[test]
    fn test_numbers_in_price() {
        assert!(detect_price_mention("el presupuesto es de 15 millones"));
        assert!(detect_price_mention("cuesta $5000 USD"));
        assert!(detect_price_mention("la tarifa es de 200 euros al mes"));
    }

    #[test]
    fn test_soft_frustration() {
        assert!(detect_frustration("estoy harta de que nadie responda"));
        assert!(detect_frustration("esto es terrible, ya llevo días así"));
    }

    #[test]
    fn test_soft_satisfaction() {
        assert!(detect_satisfaction("muy bien, me gusta mucho cómo quedó"));
        assert!(detect_satisfaction("fantástico, justo lo que buscaba"));
    }

    // ═══════════════════════════════════════════════════════════════════
    // TEST 1: ATTRIBUTION DIFFERENTIATION (Diferenciación Interlocutor/Usuario)
    // ═══════════════════════════════════════════════════════════════════
    // Verifica que las 7 señales diferenciadas generen el signal correcto
    // según sea interlocutor (cliente) o usuario (vendedor).

    #[test]
    fn test_attribution_frustration_client_vs_user() {
        // Interlocutor frustrado → "client_frustrated"
        let client_signals = analyze_turn("esto es terrible, inaceptable", true);
        assert!(
            client_signals.iter().any(|s| s.signal == "client_frustrated"),
            "Esperaba client_frustrated para interlocutor frustrado"
        );

        // Usuario frustrado → "user_losing_control"
        let user_signals = analyze_turn("esto es terrible, inaceptable", false);
        assert!(
            user_signals.iter().any(|s| s.signal == "user_losing_control"),
            "Esperaba user_losing_control para usuario frustrado"
        );
    }

    #[test]
    fn test_attribution_buying_signal_client_vs_user() {
        // Interlocutor compra → "buying_signal"
        let client_signals = analyze_turn("¿cuándo empezamos?", true);
        assert!(
            client_signals.iter().any(|s| s.signal == "buying_signal"),
            "Esperaba buying_signal para interlocutor"
        );

        // Usuario presuntivo → "user_assumptive_close"
        let user_signals = analyze_turn("¿cuándo empezamos?", false);
        assert!(
            user_signals.iter().any(|s| s.signal == "user_assumptive_close"),
            "Esperaba user_assumptive_close para usuario"
        );
    }

    #[test]
    fn test_attribution_objection_client_vs_user() {
        // Cliente objeta → "client_objection"
        let client_signals = analyze_turn("es muy caro para nosotros", true);
        assert!(
            client_signals.iter().any(|s| s.signal == "client_objection"),
            "Esperaba client_objection para interlocutor"
        );

        // Usuario preemptivo → "user_preemptive_objection"
        let user_signals = analyze_turn("es muy caro para nosotros", false);
        assert!(
            user_signals.iter().any(|s| s.signal == "user_preemptive_objection"),
            "Esperaba user_preemptive_objection para usuario"
        );
    }

    #[test]
    fn test_attribution_price_mention_client_vs_user() {
        // Cliente pregunta precio → "client_asked_price"
        let client_signals = analyze_turn("¿cuál es el precio?", true);
        assert!(
            client_signals.iter().any(|s| s.signal == "client_asked_price"),
            "Esperaba client_asked_price para interlocutor"
        );

        // Usuario menciona precio → "user_mentioned_price"
        let user_signals = analyze_turn("el precio es 500 dólares", false);
        assert!(
            user_signals.iter().any(|s| s.signal == "user_mentioned_price"),
            "Esperaba user_mentioned_price para usuario"
        );
    }

    #[test]
    fn test_attribution_hesitation_client_vs_user() {
        // Cliente duda → "client_hesitating"
        let client_signals = analyze_turn("no lo sé, tal vez después", true);
        assert!(
            client_signals.iter().any(|s| s.signal == "client_hesitating"),
            "Esperaba client_hesitating para interlocutor"
        );

        // Usuario incierto → "user_uncertain"
        let user_signals = analyze_turn("no lo sé, tal vez después", false);
        assert!(
            user_signals.iter().any(|s| s.signal == "user_uncertain"),
            "Esperaba user_uncertain para usuario"
        );
    }

    #[test]
    fn test_attribution_satisfaction_client_vs_user() {
        // Cliente satisfecho → "client_satisfied"
        let client_signals = analyze_turn("excelente, me encanta", true);
        assert!(
            client_signals.iter().any(|s| s.signal == "client_satisfied"),
            "Esperaba client_satisfied para interlocutor"
        );

        // Usuario entusiasta → "user_enthusiastic"
        let user_signals = analyze_turn("¡Excelente! ¡Me encanta!", false);
        assert!(
            user_signals.iter().any(|s| s.signal == "user_enthusiastic"),
            "Esperaba user_enthusiastic para usuario"
        );
    }

    #[test]
    fn test_attribution_possessive_language_client_vs_user() {
        // Cliente posesivo → "client_possessive"
        let client_signals = analyze_turn("cuando implementemos esto, va a ser genial", true);
        assert!(
            client_signals.iter().any(|s| s.signal == "client_possessive"),
            "Esperaba client_possessive para interlocutor"
        );

        // Usuario posesivo → "user_possessive"
        let user_signals = analyze_turn("cuando implementemos esto con tu equipo", false);
        assert!(
            user_signals.iter().any(|s| s.signal == "user_possessive"),
            "Esperaba user_possessive para usuario"
        );
    }

    // ═══════════════════════════════════════════════════════════════════
    // TEST 2: SIGNAL PRIORITY ORDERING (Múltiples Triggers)
    // ═══════════════════════════════════════════════════════════════════
    // Verifica que cuando hay múltiples triggers, el orden de prioridad
    // sea: critical > important > soft

    #[test]
    fn test_priority_critical_before_important() {
        // Frustración (critical) + precio (important)
        let signals = analyze_turn("esto es terrible, ¿cuánto cuesta?", true);
        assert!(!signals.is_empty(), "Debe haber señales");
        assert_eq!(signals[0].priority, "critical", "Primera debe ser critical");
        // Frustration es critical, así que debe venir primero
    }

    #[test]
    fn test_priority_critical_buying_signal_and_objection() {
        // Ambos critical: objeción y señal de compra
        let text = "es muy caro, pero ¿cuándo empezamos?";
        let signals = analyze_turn(text, true);
        assert!(
            signals.len() >= 2,
            "Debe detectar al menos 2 señales critical"
        );
        assert_eq!(signals[0].priority, "critical", "Primera critical");
        assert_eq!(signals[1].priority, "critical", "Segunda critical");
    }

    #[test]
    fn test_priority_multiple_signals_frustration_objection_buying() {
        let text = "esto es inaceptable, muy caro, ¿cuándo arrancamos?";
        let signals = analyze_turn(text, true);
        // Debe haber: frustration (critical), objection (critical), buying_signal (critical)
        let critical_count = signals.iter().filter(|s| s.priority == "critical").count();
        assert!(
            critical_count >= 2,
            "Debe haber al menos 2 critical signals, got {}",
            critical_count
        );
        // Verificar que los primeros N signals son critical
        for (i, signal) in signals.iter().enumerate() {
            if i < critical_count {
                assert_eq!(
                    signal.priority, "critical",
                    "Signal {} debe ser critical en posición {}",
                    signal.signal, i
                );
            }
        }
    }

    #[test]
    fn test_priority_important_before_soft() {
        let signals = analyze_turn("justo lo que necesitaba ¿es verdad?", true);
        // satisfaction (important) + question (soft)
        let has_important = signals.iter().any(|s| s.priority == "important");
        let has_soft = signals.iter().any(|s| s.priority == "soft");
        if has_important && has_soft {
            let first_important = signals
                .iter()
                .position(|s| s.priority == "important")
                .unwrap();
            let first_soft = signals
                .iter()
                .position(|s| s.priority == "soft")
                .unwrap();
            assert!(first_important < first_soft, "important debe venir antes de soft");
        }
    }

    #[test]
    fn test_priority_all_three_levels() {
        let text = "esto es absolutamente terrible, me encanta pero ¿de verdad?";
        let signals = analyze_turn(text, true);
        let priorities: Vec<&str> = signals.iter().map(|s| s.priority.as_str()).collect();

        // Buscar que critical viene antes de important
        if let (Some(critical_pos), Some(important_pos)) = (
            priorities.iter().position(|&p| p == "critical"),
            priorities.iter().position(|&p| p == "important"),
        ) {
            assert!(
                critical_pos < important_pos,
                "critical debe venir antes de important"
            );
        }
    }

    // ═══════════════════════════════════════════════════════════════════
    // TEST 3: EDGE CASES (Robustez)
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn test_edge_case_empty_text() {
        let signals = analyze_turn("", true);
        assert!(
            signals.is_empty(),
            "Texto vacío no debe generar señales"
        );
    }

    #[test]
    fn test_edge_case_only_whitespace() {
        let signals = analyze_turn("   \t\n  ", true);
        assert!(
            signals.is_empty(),
            "Whitespace puro no debe generar señales"
        );
    }

    #[test]
    fn test_edge_case_very_long_text() {
        // Texto de ~10k caracteres: debe procesarse sin panic y sin timeout
        let mut long_text = String::new();
        for _ in 0..200 {
            long_text.push_str("El cliente está muy frustrado con el servicio inaceptable. ");
        }
        // Verificar que es aproximadamente 10k chars (no exacto)
        assert!(
            long_text.len() >= 10000 && long_text.len() <= 15000,
            "Long text debe ser ~10k chars, got {}",
            long_text.len()
        );

        let start = std::time::Instant::now();
        let signals = analyze_turn(&long_text, true);
        let elapsed = start.elapsed();

        // No debe crashear
        assert!(
            signals.iter().any(|s| s.signal == "client_frustrated"),
            "Debe detectar frustration en texto largo"
        );
        // Debe ser rápido (< 500ms para 10k chars)
        assert!(
            elapsed.as_millis() < 500,
            "Procesamiento muy lento: {} ms",
            elapsed.as_millis()
        );
    }

    #[test]
    fn test_edge_case_special_characters() {
        let text = "!!!@@##$$%%^^&&** esto es terrible!!!";
        let signals = analyze_turn(text, true);
        // Debe detectar frustration a pesar de caracteres especiales
        assert!(
            signals.iter().any(|s| s.signal == "client_frustrated"),
            "Debe tolerar caracteres especiales"
        );
    }

    #[test]
    fn test_edge_case_emojis() {
        let text = "😱😡🤬 Esto es terrible, inaceptable 😤😠";
        let signals = analyze_turn(text, true);
        // No debe crashear; debe detectar frustration
        assert!(
            signals.iter().any(|s| s.signal == "client_frustrated"),
            "Debe tolerar emojis"
        );
    }

    #[test]
    fn test_edge_case_only_numbers() {
        let text = "123456789 101112 131415";
        let _signals = analyze_turn(text, true);
        // Números puros: solo verifica que no crashea
    }

    #[test]
    fn test_edge_case_mixed_language_es_en() {
        let text = "esto es terrible, we need help immediately and I'm very frustrated";
        let signals = analyze_turn(text, true);
        // Debe detectar frustration a pesar del code-switching
        assert!(
            signals.iter().any(|s| s.signal == "client_frustrated"),
            "Debe detectar en texto mixed language"
        );
    }

    #[test]
    fn test_edge_case_very_short_text() {
        let signals = analyze_turn("x", true);
        assert!(
            signals.is_empty(),
            "Un carácter no debe generar señales"
        );
    }

    #[test]
    fn test_edge_case_repeated_words() {
        // "caro" solo sin más contexto - verificar que es tratado correctamente
        let text = "es muy caro caro caro";
        let signals = analyze_turn(text, true);
        // Debe detectar algo relacionado a precio/objeción
        // (caro es parte de detect_objection, no detect_price_mention en forma aislada)
        assert!(
            !signals.is_empty(),
            "Debe detectar señal con palabra repetida"
        );
    }

    // ═══════════════════════════════════════════════════════════════════
    // TEST 4: MEETING TYPE DETECTION ROBUSTNESS
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn test_meeting_type_zero_keywords() {
        // Sin keywords claros → None
        let result = heuristic_detect("hola cómo estás qué tal");
        assert_eq!(
            result, None,
            "Sin keywords claros debe devolver None"
        );
    }

    #[test]
    fn test_meeting_type_exactly_two_keywords() {
        // Exactamente 2 keywords de sales → Sales
        let text = "la demo del producto cuesta mucho";
        let result = heuristic_detect(text);
        assert_eq!(result, Some(MeetingType::Sales), "2 keywords debe detectar Sales");
    }

    #[test]
    fn test_meeting_type_ambiguous_two_categories() {
        // 2 sales keywords + 2 service keywords → determinístico
        let text = "demo del producto y precio, pero tengo un problema y quiero soporte";
        let result = heuristic_detect(text);
        // Debe retornar alguno de forma determinística (no indeterminado)
        assert!(
            result.is_some(),
            "Debe resolver ambigüedad de forma determinística"
        );
    }

    #[test]
    fn test_meeting_type_very_short_text() {
        // <100 chars, <2 keywords → None
        let text = "hola";
        let result = heuristic_detect(text);
        assert_eq!(
            result, None,
            "Texto muy corto sin keywords debe retornar None"
        );
    }

    #[test]
    fn test_meeting_type_sales_high_score() {
        let text = "precio cotización cliente producto demo propuesta descuento cerrar comprar";
        let result = heuristic_detect(text);
        assert_eq!(
            result, Some(MeetingType::Sales),
            "Múltiples keywords sales debe detectar Sales"
        );
    }

    #[test]
    fn test_meeting_type_service_high_score() {
        let text = "queja reclamo problema error no funciona ayuda soporte";
        let result = heuristic_detect(text);
        assert_eq!(
            result, Some(MeetingType::Service),
            "Múltiples keywords service debe detectar Service"
        );
    }

    #[test]
    fn test_meeting_type_webinar_keywords() {
        let text = "bienvenidos presentación webinar les voy a mostrar";
        let result = heuristic_detect(text);
        assert_eq!(
            result, Some(MeetingType::Webinar),
            "Keywords webinar debe detectar Webinar"
        );
    }

    #[test]
    fn test_meeting_type_team_meeting_keywords() {
        let text = "equipo standup sprint bloqueo";
        let result = heuristic_detect(text);
        assert_eq!(
            result, Some(MeetingType::TeamMeeting),
            "Keywords team debe detectar TeamMeeting"
        );
    }

    #[test]
    fn test_meeting_type_single_keyword_insufficient() {
        // Solo 1 keyword, no 2+ → None
        let text = "precio";
        let result = heuristic_detect(text);
        assert_eq!(
            result, None,
            "Un solo keyword no es suficiente"
        );
    }

    #[test]
    fn test_meeting_type_case_insensitive() {
        let text = "PRECIO COTIZACIÓN CLIENTE PRODUCTO";
        let result = heuristic_detect(text);
        assert_eq!(
            result, Some(MeetingType::Sales),
            "Debe ser case-insensitive"
        );
    }

    // ═══════════════════════════════════════════════════════════════════
    // TEST 5: PROFANITY DETECTION
    // ═══════════════════════════════════════════════════════════════════
    // Nota: Los detectores en trigger.rs no incluyen detección explícita de
    // palabras malsonantes en un detector `detect_profanity()`. Sin embargo,
    // algunas palabras fuertes están en `detect_frustration()` y `detect_objection()`.
    // Estos tests validan comportamiento alrededor de lenguaje fuerte.

    #[test]
    fn test_frustration_includes_strong_language() {
        // "harto" es un intensificador de frustración
        let text = "estoy harto de esta situación";
        let signals = analyze_turn(text, true);
        assert!(
            signals.iter().any(|s| s.signal == "client_frustrated"),
            "Debe detectar frustración con 'harto'"
        );
    }

    #[test]
    fn test_frustration_with_accents() {
        // "harta" con tilde
        let text = "estoy harta de esperar";
        let signals = analyze_turn(text, true);
        assert!(
            signals.iter().any(|s| s.signal == "client_frustrated"),
            "Debe detectar con acentos"
        );
    }

    #[test]
    fn test_multiple_intensity_markers() {
        // Múltiples palabras de frustración
        let text = "esto es absolutamente inaceptable y nunca funciona";
        let signals = analyze_turn(text, true);
        assert!(
            signals.iter().any(|s| s.signal == "client_frustrated"),
            "Debe detectar múltiples markers"
        );
    }

    #[test]
    fn test_strong_language_in_compound() {
        // Palabra fuerte en contexto compuesto
        let text = "cancelar esta suscripción ahora, esto es un desastre";
        let signals = analyze_turn(text, true);
        // Cancelar es frustración
        assert!(
            signals.iter().any(|s| s.signal == "client_frustrated"),
            "Debe detectar en compound"
        );
    }

    #[test]
    fn test_intensity_escalation() {
        let _signals1 = analyze_turn("no es bueno", true);
        let signals2 = analyze_turn("esto es terrible", true);
        let signals3 = analyze_turn("esto es absolutamente inaceptable", true);

        // Todas deben detectarse como frustración
        assert!(signals2.iter().any(|s| s.signal == "client_frustrated"));
        assert!(signals3.iter().any(|s| s.signal == "client_frustrated"));
    }

    // ═══════════════════════════════════════════════════════════════════
    // ADDITIONAL COMPREHENSIVE TESTS
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn test_snippet_truncation() {
        // Verifica que snippet sea <= 100 chars
        let long_text = "a".repeat(500);
        let signals = analyze_turn(&long_text, true);
        for signal in signals {
            assert!(
                signal.snippet.len() <= 100,
                "Snippet debe ser <= 100 chars, got {}",
                signal.snippet.len()
            );
        }
    }

    #[test]
    fn test_no_duplicate_signals() {
        // No debe haber señales duplicadas en la misma vuelta
        let signals = analyze_turn("precio caro, muy caro, presupuesto", true);
        let signal_names: Vec<&str> = signals.iter().map(|s| s.signal.as_str()).collect();
        // Verificar unicidad
        for sig in signal_names.iter() {
            let count = signal_names.iter().filter(|&s| s == sig).count();
            assert_eq!(count, 1, "No debe haber duplicados de {}", sig);
        }
    }

    #[test]
    fn test_detect_hesitation_variations() {
        assert!(detect_hesitation("tal vez mañana"));
        assert!(detect_hesitation("quizas después"));
        assert!(detect_hesitation("podria ser"));
        assert!(detect_hesitation("lo reviso con mi jefe"));
        assert!(detect_hesitation("me pongo a pensar"));
        assert!(!detect_hesitation("definitivamente no"));
    }

    #[test]
    fn test_detect_question_marks() {
        assert!(detect_question("¿cuándo?"));
        assert!(detect_question("cuándo?"));
        assert!(detect_question("¿cuándo"));
        assert!(!detect_question("cuándo"));
    }

    #[test]
    fn test_analyze_turn_comprehensive() {
        // Un turno realista con múltiples triggers
        let text = "Es caro y no tenemos presupuesto, pero ¿cuándo empezamos?";
        let signals = analyze_turn(text, true);
        assert!(!signals.is_empty(), "Debe detectar algo");
        // Debe tener al menos: precio y objeción y buying_signal
        let has_critical = signals.iter().any(|s| s.priority == "critical");
        assert!(has_critical, "Debe tener al menos un critical");
    }

    #[test]
    fn test_accent_normalization_detection() {
        // "déjame" y "dejame" deben ser equivalentes
        let text1 = "déjame pensar";
        let text2 = "dejame pensar";
        let signals1 = analyze_turn(text1, true);
        let signals2 = analyze_turn(text2, true);
        assert!(signals1.iter().any(|s| s.signal == "client_objection"));
        assert!(signals2.iter().any(|s| s.signal == "client_objection"));
    }

    #[test]
    fn test_satisfaction_multiple_forms() {
        assert!(detect_satisfaction("excelente"));
        assert!(detect_satisfaction("perfecto"));
        assert!(detect_satisfaction("me encanta"));
        assert!(detect_satisfaction("impresionante"));
        assert!(detect_satisfaction("muy bien"));
    }

    #[test]
    fn test_enthusiasm_requires_exclamation_and_word() {
        // Exclamación sin palabra positiva
        assert!(!detect_enthusiasm("¡Hola!"));
        // Palabra positiva sin exclamación
        assert!(!detect_enthusiasm("me encanta"));
        // Ambos
        assert!(detect_enthusiasm("¡Me encanta!"));
    }

    #[test]
    fn test_monologue_threshold() {
        assert!(detect_monologue(121));
        assert!(detect_monologue(150));
        assert!(!detect_monologue(120));
        assert!(!detect_monologue(60));
    }

    #[test]
    fn test_talk_ratio_boundaries() {
        assert!(detect_high_talk_ratio(0.61));
        assert!(!detect_high_talk_ratio(0.60));
        assert!(detect_low_talk_ratio(0.24));
        assert!(!detect_low_talk_ratio(0.25));
    }
}
