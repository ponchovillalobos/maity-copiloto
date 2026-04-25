//! Tipos de la evaluación post-meeting v4 (Gemma 4).
//!
//! Mapea EXACTO el JSON que produce `EVALUATION_V4_SYSTEM_PROMPT`. Todos los
//! campos son opcionales en deserialización (`#[serde(default)]`) para tolerar
//! que el LLM omita secciones — el frontend renderiza solo lo presente.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MeetingEvaluation {
    #[serde(default)]
    pub identificacion: Identificacion,
    #[serde(default)]
    pub historico: Historico,
    #[serde(default)]
    pub contexto: Contexto,
    #[serde(default)]
    pub meta: Meta,
    #[serde(default)]
    pub resumen: Resumen,
    #[serde(default)]
    pub radiografia: Radiografia,
    #[serde(default)]
    pub insights: Vec<Insight>,
    #[serde(default)]
    pub patron: Patron,
    #[serde(default)]
    pub timeline: Timeline,
    #[serde(default)]
    pub dimensiones: Dimensiones,
    #[serde(default)]
    pub por_hablante: HashMap<String, HablanteStats>,
    #[serde(default)]
    pub empatia: HashMap<String, EmpatiaHablante>,
    #[serde(default)]
    pub calidad_global: CalidadGlobal,
    #[serde(default)]
    pub recomendaciones: Vec<Recomendacion>,
    #[serde(default)]
    pub visualizaciones: Visualizaciones,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Identificacion {
    #[serde(default)]
    pub sesion_id: Option<String>,
    #[serde(default)]
    pub nombre_sesion: String,
    #[serde(default)]
    pub fecha_analisis: String,
    #[serde(default)]
    pub version_prompt: String,
    #[serde(default)]
    pub idioma: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Historico {
    #[serde(default)]
    pub sesion_anterior_id: Option<String>,
    #[serde(default)]
    pub tendencia_global: Option<f32>,
    #[serde(default)]
    pub mejoras_detectadas: Vec<String>,
    #[serde(default)]
    pub regresiones_detectadas: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Contexto {
    #[serde(default)]
    pub relacion: String,
    #[serde(default)]
    pub formalidad_esperada: String,
    #[serde(default)]
    pub formalidad_observada: String,
    #[serde(default)]
    pub brecha_formalidad: String,
    #[serde(default)]
    pub objetivo_declarado: String,
    #[serde(default)]
    pub objetivo_real_inferido: String,
    #[serde(default)]
    pub alineacion_objetivo: f32,
    #[serde(default)]
    pub tipo_comunicacion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Meta {
    #[serde(default)]
    pub tipo: String,
    #[serde(default)]
    pub hablantes: Vec<String>,
    #[serde(default)]
    pub palabras_totales: u32,
    #[serde(default)]
    pub oraciones_totales: u32,
    #[serde(default)]
    pub turnos_totales: u32,
    #[serde(default)]
    pub duracion_minutos: u32,
    #[serde(default)]
    pub palabras_por_hablante: HashMap<String, u32>,
    #[serde(default)]
    pub fecha: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Resumen {
    #[serde(default)]
    pub puntuacion_global: f32,
    #[serde(default)]
    pub nivel: String,
    #[serde(default)]
    pub descripcion: String,
    #[serde(default)]
    pub fortaleza: String,
    #[serde(default)]
    pub fortaleza_hint: String,
    #[serde(default)]
    pub mejorar: String,
    #[serde(default)]
    pub mejorar_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Radiografia {
    #[serde(default)]
    pub muletillas_total: u32,
    #[serde(default)]
    pub muletillas_detalle: HashMap<String, u32>,
    #[serde(default)]
    pub muletillas_frecuencia: String,
    #[serde(default)]
    pub ratio_habla: f32,
    #[serde(default)]
    pub preguntas: HashMap<String, u32>,
    #[serde(default)]
    pub puertas_emocionales: PuertasEmocionales,
    #[serde(default)]
    pub puertas_detalle: Vec<PuertaDetalle>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PuertasEmocionales {
    #[serde(default)]
    pub momentos_vulnerabilidad: u32,
    #[serde(default)]
    pub abiertas: u32,
    #[serde(default)]
    pub exploradas: u32,
    #[serde(default)]
    pub no_exploradas: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PuertaDetalle {
    #[serde(default)]
    pub quien: String,
    #[serde(default)]
    pub minuto: u32,
    #[serde(default)]
    pub cita: String,
    #[serde(default)]
    pub explorada: bool,
    #[serde(default)]
    pub respuesta: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Insight {
    #[serde(default)]
    pub dato: String,
    #[serde(default)]
    pub por_que: String,
    #[serde(default)]
    pub sugerencia: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Patron {
    #[serde(default)]
    pub actual: String,
    #[serde(default)]
    pub evolucion: String,
    #[serde(default)]
    pub senales: Vec<String>,
    #[serde(default)]
    pub que_cambiaria: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Timeline {
    #[serde(default)]
    pub segmentos: Vec<TimelineSegmento>,
    #[serde(default)]
    pub momentos_clave: Vec<MomentoClave>,
    #[serde(default)]
    pub lectura: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimelineSegmento {
    #[serde(default)]
    pub tipo: String,
    #[serde(default)]
    pub pct: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MomentoClave {
    #[serde(default)]
    pub nombre: String,
    #[serde(default)]
    pub minuto: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Dimensiones {
    #[serde(default)]
    pub claridad: DimensionConCita,
    #[serde(default)]
    pub proposito: DimensionConCita,
    #[serde(default)]
    pub emociones: DimensionEmociones,
    #[serde(default)]
    pub estructura: DimensionConCita,
    #[serde(default)]
    pub persuasion: DimensionConCita,
    #[serde(default)]
    pub muletillas: DimensionMuletillas,
    #[serde(default)]
    pub adaptacion: DimensionConCita,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DimensionConCita {
    #[serde(default)]
    pub puntaje: f32,
    #[serde(default)]
    pub nivel: String,
    #[serde(default)]
    pub que_mide: String,
    #[serde(default)]
    pub tu_resultado: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DimensionEmociones {
    #[serde(default)]
    pub tono_general: String,
    #[serde(default)]
    pub polaridad: f32,
    #[serde(default)]
    pub radar: RadarEmociones,
    #[serde(default)]
    pub por_hablante: HashMap<String, EmocionHablante>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RadarEmociones {
    #[serde(default)]
    pub alegria: f32,
    #[serde(default)]
    pub confianza: f32,
    #[serde(default)]
    pub miedo: f32,
    #[serde(default)]
    pub sorpresa: f32,
    #[serde(default)]
    pub tristeza: f32,
    #[serde(default)]
    pub disgusto: f32,
    #[serde(default)]
    pub ira: f32,
    #[serde(default)]
    pub anticipacion: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmocionHablante {
    #[serde(default)]
    pub emocion_dominante: String,
    #[serde(default)]
    pub valor: f32,
    #[serde(default)]
    pub subtexto: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DimensionMuletillas {
    #[serde(default)]
    pub total: u32,
    #[serde(default)]
    pub frecuencia: String,
    #[serde(default)]
    pub nivel: String,
    #[serde(default)]
    pub detalle: HashMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HablanteStats {
    #[serde(default)]
    pub palabras: u32,
    #[serde(default)]
    pub oraciones: u32,
    #[serde(default)]
    pub resumen: String,
    #[serde(default)]
    pub claridad: f32,
    #[serde(default)]
    pub persuasion: f32,
    #[serde(default)]
    pub formalidad: f32,
    #[serde(default)]
    pub emociones: HablanteEmociones,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HablanteEmociones {
    #[serde(default)]
    pub dominante: String,
    #[serde(default)]
    pub valor: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EmpatiaHablante {
    #[serde(default)]
    pub evaluable: bool,
    #[serde(default)]
    pub puntaje: f32,
    #[serde(default)]
    pub nivel: String,
    #[serde(default)]
    pub tu_resultado: String,
    #[serde(default)]
    pub reconocimiento_emocional: f32,
    #[serde(default)]
    pub escucha_activa: f32,
    #[serde(default)]
    pub tono_empatico: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalidadGlobal {
    #[serde(default)]
    pub puntaje: f32,
    #[serde(default)]
    pub nivel: String,
    #[serde(default)]
    pub formula_usada: String,
    #[serde(default)]
    pub componentes: ComponentesCalidad,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ComponentesCalidad {
    #[serde(default)]
    pub claridad: f32,
    #[serde(default)]
    pub estructura: f32,
    #[serde(default)]
    pub persuasion: f32,
    #[serde(default)]
    pub proposito: f32,
    #[serde(default)]
    pub empatia: f32,
    #[serde(default)]
    pub adaptacion: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Recomendacion {
    #[serde(default)]
    pub prioridad: u32,
    #[serde(default)]
    pub titulo: String,
    #[serde(default)]
    pub texto_mejorado: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Visualizaciones {
    #[serde(default)]
    pub gauge: GaugeViz,
    #[serde(default)]
    pub radar_calidad: RadarViz,
    #[serde(default)]
    pub muletillas_chart: BarChartViz,
    #[serde(default)]
    pub timeline_chart: TimelineViz,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GaugeViz {
    #[serde(default)]
    pub valor: f32,
    #[serde(default)]
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RadarViz {
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub valores: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BarChartViz {
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub valores: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimelineViz {
    #[serde(default)]
    pub segmentos: Vec<TimelineSegmento>,
    #[serde(default)]
    pub momentos: Vec<MomentoClave>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_evaluation_json() {
        let raw = r#"{
            "identificacion": {"nombre_sesion": "Test"},
            "resumen": {"puntuacion_global": 72.5, "nivel": "competente"}
        }"#;
        let eval: MeetingEvaluation = serde_json::from_str(raw).unwrap();
        assert_eq!(eval.identificacion.nombre_sesion, "Test");
        assert!((eval.resumen.puntuacion_global - 72.5).abs() < 0.01);
        assert_eq!(eval.resumen.nivel, "competente");
    }

    #[test]
    fn defaults_fill_missing_fields() {
        let raw = "{}";
        let eval: MeetingEvaluation = serde_json::from_str(raw).unwrap();
        assert_eq!(eval.resumen.puntuacion_global, 0.0);
        assert_eq!(eval.recomendaciones.len(), 0);
        assert_eq!(eval.dimensiones.claridad.puntaje, 0.0);
    }

    #[test]
    fn round_trip_preserves_radar_emociones() {
        let mut eval = MeetingEvaluation::default();
        eval.dimensiones.emociones.radar.alegria = 0.7;
        eval.dimensiones.emociones.radar.confianza = 0.4;
        let s = serde_json::to_string(&eval).unwrap();
        let back: MeetingEvaluation = serde_json::from_str(&s).unwrap();
        assert!((back.dimensiones.emociones.radar.alegria - 0.7).abs() < 0.01);
        assert!((back.dimensiones.emociones.radar.confianza - 0.4).abs() < 0.01);
    }

    #[test]
    fn parses_visualizaciones_section() {
        let raw = r#"{
            "visualizaciones": {
                "gauge": {"valor": 65, "label": "aceptable"},
                "radar_calidad": {
                    "labels": ["Claridad","Estructura","Persuasión","Propósito","Empatía","Adaptación"],
                    "valores": [60, 55, 70, 65, 72, 58]
                },
                "muletillas_chart": {"labels": ["este","pues"], "valores": [12, 8]}
            }
        }"#;
        let eval: MeetingEvaluation = serde_json::from_str(raw).unwrap();
        assert_eq!(eval.visualizaciones.gauge.valor, 65.0);
        assert_eq!(eval.visualizaciones.radar_calidad.valores.len(), 6);
        assert_eq!(eval.visualizaciones.muletillas_chart.labels[0], "este");
    }

    #[test]
    fn parses_insights_array() {
        let raw = r#"{
            "insights": [
                {"dato": "3 de 5 preguntas no respondidas", "por_que": "pierdes valor", "sugerencia": "Cierra cada pregunta antes de seguir"}
            ]
        }"#;
        let eval: MeetingEvaluation = serde_json::from_str(raw).unwrap();
        assert_eq!(eval.insights.len(), 1);
        assert!(eval.insights[0].dato.starts_with("3 de"));
    }
}
