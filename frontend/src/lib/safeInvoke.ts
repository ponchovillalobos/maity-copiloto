import { invoke, InvokeArgs } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { logger } from '@/lib/logger';

/**
 * Wrapper de `invoke()` que NUNCA lanza excepciones — siempre devuelve `T | null`
 * y muestra un toast amigable al usuario en caso de error.
 *
 * Usar este helper en vez de `invoke()` directo para todo botón / acción que
 * el usuario dispara, así nunca queda un comando "silencioso" cuando falla.
 *
 * @param cmd     Nombre del comando Tauri.
 * @param args    Argumentos del comando.
 * @param userMsg Mensaje opcional para el toast (si se omite, mensaje genérico).
 * @returns       Resultado del comando o `null` si falló.
 */
export async function safeInvoke<T = unknown>(
  cmd: string,
  args?: InvokeArgs,
  userMsg?: string,
): Promise<T | null> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    const detail = e instanceof Error ? e.message : String(e);
    logger.warn(`[safeInvoke ${cmd}] ${detail}`);
    toast.error(userMsg ?? `No se pudo completar la acción: ${cmd}`);
    return null;
  }
}

/**
 * Variante silenciosa: igual que `safeInvoke` pero sin toast (solo log).
 * Para acciones de fondo / polling donde no queremos molestar al usuario.
 */
export async function quietInvoke<T = unknown>(
  cmd: string,
  args?: InvokeArgs,
): Promise<T | null> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    const detail = e instanceof Error ? e.message : String(e);
    logger.debug(`[quietInvoke ${cmd}] ${detail}`);
    return null;
  }
}
