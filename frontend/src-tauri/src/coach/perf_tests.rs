//! Pruebas de desempeño del Coach IA.
//!
//! Mide tiempos de operaciones locales (no llamadas a Ollama) para detectar
//! regresiones de latencia en hot paths. Los tests de LLM real requieren
//! Ollama corriendo y son manuales.
//!
//! Targets:
//! - `build_user_prompt_v3`: <1ms (solo concatenación)
//! - `parse_llm_output` con JSON válido: <1ms
//! - `analyze_turn` (todos los detectores): <5ms para texto de 1KB
//! - Clonado de contexto 4k chars: <1ms
//!
//! Si algún test tarda más del target, hay regresión de performance.

#[cfg(test)]
mod tests {
    use crate::coach::prompt::{build_user_prompt_v3, MeetingType};
    use crate::coach::trigger::analyze_turn;
    use std::time::Instant;

    const TEXTO_SALES: &str = "El producto es un poco caro, no sé si mi jefe me aprueba el presupuesto. \
        Tendría que revisarlo con el equipo técnico antes de decidir. Cuando empecemos \
        con esto, ¿qué tan rápido podríamos ver resultados? Me interesa pero necesito \
        más información sobre el ROI y las integraciones disponibles con nuestro stack actual.";

    #[test]
    fn perf_build_user_prompt_v3_bajo_1ms() {
        let contexto = "USUARIO: Hola\nINTERLOCUTOR: Buenos días\n".repeat(50); // ~2KB
        let previous_tips: Vec<String> = vec![];

        // Warmup
        for _ in 0..10 {
            let _ = build_user_prompt_v3(&contexto, MeetingType::Sales, 5, &previous_tips, None, None);
        }

        let iters = 100;
        let start = Instant::now();
        for _ in 0..iters {
            let _ = build_user_prompt_v3(&contexto, MeetingType::Sales, 5, &previous_tips, Some("objection"), Some("client_objection"));
        }
        let avg_us = start.elapsed().as_micros() / iters;

        assert!(
            avg_us < 1000,
            "build_user_prompt_v3 tardó {}µs (target <1000µs)", avg_us
        );
    }

    #[test]
    fn perf_analyze_turn_bajo_5ms() {
        // Warmup
        for _ in 0..10 {
            let _ = analyze_turn(TEXTO_SALES, true);
        }

        let iters = 50;
        let start = Instant::now();
        for _ in 0..iters {
            let _ = analyze_turn(TEXTO_SALES, true);
        }
        let avg_us = start.elapsed().as_micros() / iters;

        assert!(
            avg_us < 5000,
            "analyze_turn tardó {}µs (target <5000µs=5ms)", avg_us
        );
    }

    #[test]
    fn perf_analyze_turn_texto_grande_bajo_15ms() {
        let texto_grande = TEXTO_SALES.repeat(10); // ~5KB
        for _ in 0..5 {
            let _ = analyze_turn(&texto_grande, true);
        }

        let iters = 20;
        let start = Instant::now();
        for _ in 0..iters {
            let _ = analyze_turn(&texto_grande, true);
        }
        let avg_us = start.elapsed().as_micros() / iters;

        assert!(
            avg_us < 15_000,
            "analyze_turn(5KB) tardó {}µs (target <15000µs=15ms)", avg_us
        );
    }

    #[test]
    fn perf_clone_contexto_4k_bajo_500us() {
        let contexto: String = "USUARIO: texto de prueba ".repeat(160); // ~4KB
        for _ in 0..10 {
            let _ = contexto.clone();
        }

        let iters = 1000;
        let start = Instant::now();
        for _ in 0..iters {
            let _ = contexto.clone();
        }
        let avg_ns = start.elapsed().as_nanos() / iters;

        assert!(
            avg_ns < 500_000,
            "clone(4KB) tardó {}ns (target <500000ns=500µs)", avg_ns
        );
    }
}
