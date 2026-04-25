//! Prompts del Coach IA.
//!
//! Cada submódulo expone una constante `*_SYSTEM_PROMPT` con el prompt completo
//! para una capacidad específica. Mantenerlos separados facilita versionar y
//! probar prompts sin tocar la lógica del cliente LLM.

pub mod evaluation_v4;
