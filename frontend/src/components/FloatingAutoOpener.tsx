'use client';

import { useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { logger } from '@/lib/logger';

/**
 * Auto-abre la ventana flotante always-on-top cuando la grabación inicia
 * y la cierra cuando termina. Componente sin UI: solo escucha eventos
 * Tauri globales y dispara comandos.
 *
 * Vital para video llamadas: el usuario no debe tener que clickear el
 * botón PiP cada vez que graba.
 */
export function FloatingAutoOpener() {
  useEffect(() => {
    let unlistenStart: (() => void) | null = null;
    let unlistenStop: (() => void) | null = null;

    (async () => {
      try {
        unlistenStart = await listen('recording-started', async () => {
          logger.debug('[FloatingAutoOpener] recording-started → abriendo flotante');
          try {
            await invoke('open_floating_coach');
          } catch (e) {
            logger.warn('[FloatingAutoOpener] No se pudo abrir flotante:', e);
          }
        });

        unlistenStop = await listen('recording-stop-complete', async () => {
          logger.debug('[FloatingAutoOpener] recording-stop-complete → cerrando flotante');
          try {
            await invoke('close_floating_coach');
          } catch (e) {
            logger.warn('[FloatingAutoOpener] No se pudo cerrar flotante:', e);
          }
        });
      } catch (e) {
        logger.error('[FloatingAutoOpener] Error setup listeners:', e);
      }
    })();

    return () => {
      unlistenStart?.();
      unlistenStop?.();
    };
  }, []);

  return null;
}
