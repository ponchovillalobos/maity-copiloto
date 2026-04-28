import { logger } from '@/lib/logger';

/**
 * Cliente directo a la API local de Ollama (puerto 11434 hardcodeado).
 * Reemplaza los comandos Tauri fantasma `get_ollama_models`, `pull_ollama_model`,
 * etc. con llamadas HTTP directas que nunca pueden fallar por "comando no
 * registrado".
 */

const OLLAMA_BASE = 'http://localhost:11434';

export interface OllamaModel {
  name: string;
  size: number;
  modified_at: string;
}

/** Lista los modelos instalados localmente. Devuelve [] si Ollama no responde. */
export async function listOllamaModels(): Promise<OllamaModel[]> {
  try {
    const res = await fetch(`${OLLAMA_BASE}/api/tags`, { signal: AbortSignal.timeout(3000) });
    if (!res.ok) return [];
    const data = await res.json();
    return Array.isArray(data?.models) ? data.models : [];
  } catch (e) {
    logger.debug(`[ollama] listModels fallo: ${e}`);
    return [];
  }
}

/** Verifica que Ollama responda en menos de 2s. */
export async function isOllamaRunning(): Promise<boolean> {
  try {
    const res = await fetch(`${OLLAMA_BASE}/api/tags`, { signal: AbortSignal.timeout(2000) });
    return res.ok;
  } catch {
    return false;
  }
}

/** Verifica si un modelo concreto está instalado. */
export async function hasOllamaModel(name: string): Promise<boolean> {
  const models = await listOllamaModels();
  return models.some((m) => m.name === name || m.name.startsWith(`${name}:`));
}
