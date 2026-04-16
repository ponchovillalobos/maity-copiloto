'use client';

/**
 * AutoSetupOverlay — muestra el estado del auto-setup de dependencias
 * (Ollama, modelo LLM, modelo Parakeet) en una barra inferior discreta.
 *
 * Escucha el evento `auto-setup-progress` emitido por el backend.
 * Auto-desaparece cuando `phase === 'done'` después de 3s.
 * Si `phase === 'ollama_missing'`, muestra link para descargar Ollama.
 */

import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { motion, AnimatePresence } from 'framer-motion';
import { CheckCircle2, Download, AlertCircle, ExternalLink, Loader2 } from 'lucide-react';

interface Progress {
  phase: string;
  step: number;
  totalSteps: number;
  message: string;
  percent?: number;
  resource?: string;
}

export function AutoSetupOverlay() {
  const [progress, setProgress] = useState<Progress | null>(null);
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    let hideTimer: ReturnType<typeof setTimeout> | undefined;
    let unlistenSetup: (() => void) | undefined;
    let unlistenOllamaProgress: (() => void) | undefined;
    let unlistenParakeetProgress: (() => void) | undefined;

    const attach = async () => {
      unlistenSetup = await listen<Progress>('auto-setup-progress', (event) => {
        setProgress(event.payload);
        setVisible(true);
        if (event.payload.phase === 'done') {
          if (hideTimer) clearTimeout(hideTimer);
          hideTimer = setTimeout(() => setVisible(false), 3000);
        }
      });

      // Merge de progress de Ollama pull: sobre-escribe percent del progress actual
      unlistenOllamaProgress = await listen<{ status?: string; completed?: number; total?: number; modelName?: string }>(
        'ollama-model-download-progress',
        (event) => {
          const pct =
            event.payload.total && event.payload.completed
              ? Math.round((event.payload.completed / event.payload.total) * 100)
              : undefined;
          setProgress((prev) => ({
            phase: 'pulling_llm',
            step: 2,
            totalSteps: 3,
            message: event.payload.status
              ? `Modelo IA: ${event.payload.status}`
              : `Descargando modelo IA...`,
            percent: pct,
            resource: event.payload.modelName ?? prev?.resource,
          }));
          setVisible(true);
        },
      );

      unlistenParakeetProgress = await listen<{ percent?: number; progress?: number; downloaded?: number; total?: number }>(
        'parakeet-model-download-progress',
        (event) => {
          const pct =
            event.payload.percent ??
            event.payload.progress ??
            (event.payload.total && event.payload.downloaded
              ? Math.round((event.payload.downloaded / event.payload.total) * 100)
              : undefined);
          setProgress({
            phase: 'downloading_parakeet',
            step: 3,
            totalSteps: 3,
            message: 'Descargando modelo de transcripción...',
            percent: pct,
            resource: 'parakeet',
          });
          setVisible(true);
        },
      );
    };

    attach();
    return () => {
      if (hideTimer) clearTimeout(hideTimer);
      unlistenSetup?.();
      unlistenOllamaProgress?.();
      unlistenParakeetProgress?.();
    };
  }, []);

  if (!visible || !progress) return null;

  const isError = progress.phase === 'error';
  const isOllamaMissing = progress.phase === 'ollama_missing';
  const isDone = progress.phase === 'done';
  const isDownloading = progress.phase === 'pulling_llm' || progress.phase === 'downloading_parakeet';

  const bgColor = isError
    ? 'bg-red-900/90 border-red-600/50'
    : isOllamaMissing
    ? 'bg-amber-900/90 border-amber-600/50'
    : isDone
    ? 'bg-green-900/90 border-green-600/50'
    : 'bg-blue-900/90 border-blue-600/50';

  const icon = isError ? (
    <AlertCircle className="w-5 h-5 text-red-300 flex-shrink-0" />
  ) : isOllamaMissing ? (
    <AlertCircle className="w-5 h-5 text-amber-300 flex-shrink-0" />
  ) : isDone ? (
    <CheckCircle2 className="w-5 h-5 text-green-300 flex-shrink-0" />
  ) : isDownloading ? (
    <Download className="w-5 h-5 text-blue-300 flex-shrink-0 animate-pulse" />
  ) : (
    <Loader2 className="w-5 h-5 text-blue-300 flex-shrink-0 animate-spin" />
  );

  return (
    <AnimatePresence>
      <motion.div
        initial={{ y: 60, opacity: 0 }}
        animate={{ y: 0, opacity: 1 }}
        exit={{ y: 60, opacity: 0 }}
        transition={{ type: 'spring', bounce: 0.2, duration: 0.4 }}
        className={`fixed bottom-4 right-4 z-50 max-w-sm rounded-lg border backdrop-blur-md shadow-xl ${bgColor}`}
        role="status"
        aria-live="polite"
      >
        <div className="px-4 py-3">
          <div className="flex items-start gap-3">
            {icon}
            <div className="flex-1 min-w-0">
              <div className="flex items-center justify-between mb-1">
                <span className="text-xs font-medium text-white/90">
                  Preparando Maity · Paso {progress.step}/{progress.totalSteps}
                </span>
                {typeof progress.percent === 'number' && (
                  <span className="text-xs font-mono text-white/70 ml-2">{progress.percent}%</span>
                )}
              </div>
              <p className="text-sm text-white leading-snug">{progress.message}</p>

              {typeof progress.percent === 'number' && (
                <div className="mt-2 h-1 rounded-full bg-white/10 overflow-hidden">
                  <motion.div
                    className="h-full bg-white/70"
                    initial={{ width: 0 }}
                    animate={{ width: `${progress.percent}%` }}
                    transition={{ duration: 0.3 }}
                  />
                </div>
              )}

              {isOllamaMissing && (
                <div className="mt-2 flex gap-2">
                  <a
                    href="https://ollama.com/download"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="inline-flex items-center gap-1 px-2 py-1 text-xs rounded bg-amber-700/70 hover:bg-amber-700 text-white transition"
                  >
                    Descargar Ollama <ExternalLink className="w-3 h-3" />
                  </a>
                  <button
                    onClick={() => invoke('auto_setup_retry').catch(() => {})}
                    className="px-2 py-1 text-xs rounded bg-white/10 hover:bg-white/20 text-white transition"
                  >
                    Reintentar
                  </button>
                </div>
              )}

              {isError && (
                <button
                  onClick={() => invoke('auto_setup_retry').catch(() => {})}
                  className="mt-2 px-2 py-1 text-xs rounded bg-white/10 hover:bg-white/20 text-white transition"
                >
                  Reintentar
                </button>
              )}
            </div>
          </div>
        </div>
      </motion.div>
    </AnimatePresence>
  );
}
