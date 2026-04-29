'use client';

import React, { useEffect, useRef, useState } from 'react';
import { CheckCircle2, Loader2, Sparkles, Mic2, AlertCircle } from 'lucide-react';
import { OnboardingContainer } from '../OnboardingContainer';
import { useOnboarding } from '@/contexts/OnboardingContext';
import { listen } from '@tauri-apps/api/event';
import { safeInvoke, quietInvoke } from '@/lib/safeInvoke';
import { logger } from '@/lib/logger';

/**
 * AutoModelSetupStep — Descarga AUTOMÁTICA de los 2 modelos requeridos al
 * montar el componente. SIN opciones técnicas, SIN clicks. El usuario solo
 * ve barras de progreso y un mensaje de bienvenida.
 *
 * Modelos:
 * - Gemma 3n E2B Q4_K_M (~2.8 GB) — IA local para tips y evaluación
 * - Parakeet TDT 0.6B (~270 MB) — transcripción en español
 *
 * Cuando ambos terminan, se avanza automáticamente al siguiente paso.
 */

const COACH_MODEL = 'gemma3:4b';
const TRANSCRIPT_MODEL = 'parakeet-tdt-0.6b-v3-int8';

interface DownloadState {
  /** Progreso 0-100. */
  progress: number;
  /** Listo (descargado y verificado). */
  installed: boolean;
  /** Error si falla. */
  error: string | null;
  /** Timestamp del primer evento de progreso. */
  startedAt: number | null;
  /** MB descargados / total. */
  downloadedMb: number;
  totalMb: number;
}

const initialState = (): DownloadState => ({
  progress: 0,
  installed: false,
  error: null,
  startedAt: null,
  downloadedMb: 0,
  totalMb: 0,
});

function formatEta(s: DownloadState): string {
  if (!s.startedAt || s.progress <= 0 || s.progress >= 100) return '';
  const elapsedSec = (Date.now() - s.startedAt) / 1000;
  if (elapsedSec < 5) return 'calculando…';
  const totalEstimatedSec = (elapsedSec / s.progress) * 100;
  const remainingSec = Math.max(0, totalEstimatedSec - elapsedSec);
  if (remainingSec < 60) return `~${Math.ceil(remainingSec)}s restantes`;
  return `~${Math.ceil(remainingSec / 60)} min restantes`;
}

export function AutoModelSetupStep() {
  const { goNext } = useOnboarding();
  const [isMac, setIsMac] = useState(false);
  const [coach, setCoach] = useState<DownloadState>(initialState);
  const [transcript, setTranscript] = useState<DownloadState>(initialState);
  const [advancedAuto, setAdvancedAuto] = useState(false);
  const startedRef = useRef(false);

  useEffect(() => {
    (async () => {
      try {
        const { platform } = await import('@tauri-apps/plugin-os');
        setIsMac(platform() === 'macos');
      } catch {
        setIsMac(navigator.userAgent.includes('Mac'));
      }
    })();
  }, []);

  // Setup listeners
  useEffect(() => {
    const unsubs: Array<() => void> = [];
    (async () => {
      // Coach (builtin-ai)
      const u1 = await listen<{ model: string; progress: number; downloaded_bytes?: number; total_bytes?: number }>(
        'builtin-ai-download-progress',
        (e) => {
          if (e.payload.model !== COACH_MODEL) return;
          setCoach((prev) => ({
            ...prev,
            progress: Math.round(e.payload.progress),
            installed: e.payload.progress >= 100,
            startedAt: prev.startedAt ?? Date.now(),
            downloadedMb: e.payload.downloaded_bytes ? Math.round(e.payload.downloaded_bytes / (1024 * 1024)) : prev.downloadedMb,
            totalMb: e.payload.total_bytes ? Math.round(e.payload.total_bytes / (1024 * 1024)) : prev.totalMb,
          }));
        },
      );
      unsubs.push(u1);
      const u2 = await listen<{ model: string }>('builtin-ai-download-complete', (e) => {
        if (e.payload.model === COACH_MODEL) setCoach((p) => ({ ...p, installed: true, progress: 100 }));
      });
      unsubs.push(u2);
      const u3 = await listen<{ model: string; error: string }>('builtin-ai-download-error', (e) => {
        if (e.payload.model === COACH_MODEL) setCoach((p) => ({ ...p, error: e.payload.error }));
      });
      unsubs.push(u3);

      // Parakeet (transcripción)
      const t1 = await listen<{ progress: number; downloaded_bytes?: number; total_bytes?: number }>(
        'parakeet-model-download-progress',
        (e) => {
          setTranscript((prev) => ({
            ...prev,
            progress: Math.round(e.payload.progress),
            installed: e.payload.progress >= 100,
            startedAt: prev.startedAt ?? Date.now(),
            downloadedMb: e.payload.downloaded_bytes ? Math.round(e.payload.downloaded_bytes / (1024 * 1024)) : prev.downloadedMb,
            totalMb: e.payload.total_bytes ? Math.round(e.payload.total_bytes / (1024 * 1024)) : prev.totalMb,
          }));
        },
      );
      unsubs.push(t1);
      const t2 = await listen<unknown>('parakeet-model-download-complete', () => {
        setTranscript((p) => ({ ...p, installed: true, progress: 100 }));
      });
      unsubs.push(t2);
      const t3 = await listen<{ error: string }>('parakeet-model-download-error', (e) => {
        setTranscript((p) => ({ ...p, error: e.payload.error }));
      });
      unsubs.push(t3);
    })();
    return () => {
      unsubs.forEach((u) => u());
    };
  }, []);

  // Auto-iniciar descargas al montar el componente
  useEffect(() => {
    if (startedRef.current) return;
    startedRef.current = true;
    (async () => {
      // Verificar si los modelos ya están descargados (idempotente).
      const coachReady = await quietInvoke<boolean>('builtin_ai_is_model_ready', { modelName: COACH_MODEL });
      if (coachReady) {
        setCoach({ ...initialState(), installed: true, progress: 100 });
      } else {
        logger.info('[Onboarding] Auto-iniciando descarga del modelo de IA…');
        setCoach((p) => ({ ...p, startedAt: Date.now() }));
        void safeInvoke(
          'builtin_ai_download_model',
          { modelName: COACH_MODEL },
          'No se pudo descargar el modelo de IA. Verifica tu conexión a internet.',
        );
      }

      // Parakeet — el comando es idempotente: si el modelo ya existe en disco
      // termina inmediatamente sin descargar nada y emite progress=100.
      logger.info('[Onboarding] Auto-iniciando descarga del modelo de transcripción…');
      setTranscript((p) => ({ ...p, startedAt: Date.now() }));
      void safeInvoke(
        'parakeet_download_model',
        { modelName: TRANSCRIPT_MODEL },
        'No se pudo descargar el modelo de transcripción. Verifica tu conexión a internet.',
      );
    })();
  }, []);

  // Avanzar automáticamente cuando ambos terminen.
  useEffect(() => {
    if (advancedAuto) return;
    if (coach.installed && transcript.installed) {
      setAdvancedAuto(true);
      const t = setTimeout(() => goNext(), 1200);
      return () => clearTimeout(t);
    }
  }, [coach.installed, transcript.installed, advancedAuto, goNext]);

  const overallProgress = Math.round((coach.progress + transcript.progress) / 2);
  const hasError = !!(coach.error || transcript.error);
  const allDone = coach.installed && transcript.installed;

  return (
    <OnboardingContainer
      title={allDone ? '¡Todo listo!' : 'Preparando tu copiloto'}
      description={
        allDone
          ? 'Tu IA local está activa. En segundos pasamos al siguiente paso.'
          : 'Estamos descargando todo lo que Maity necesita para funcionar 100% en tu equipo. Sin internet ni servicios externos después de esto.'
      }
      step={2}
      totalSteps={isMac ? 6 : 5}
    >
      <div className="space-y-6">
        {/* Progreso global */}
        <div className="rounded-xl border border-white/10 bg-white/5 p-5">
          <div className="flex items-center justify-between text-sm mb-3">
            <span className="font-semibold text-gray-100">Progreso general</span>
            <span className="text-2xl font-bold text-blue-300 tabular-nums">{overallProgress}%</span>
          </div>
          <div className="h-3 rounded-full bg-white/10 overflow-hidden">
            <div
              className="h-full bg-gradient-to-r from-blue-400 to-emerald-400 transition-all duration-500"
              style={{ width: `${overallProgress}%` }}
            />
          </div>
          {!allDone && (
            <p className="text-xs text-gray-400 mt-3 leading-relaxed">
              La descarga ocurre solo una vez. Después Maity funciona sin internet.
              Podés seguir usando tu computadora mientras tanto.
            </p>
          )}
        </div>

        {/* Modelo IA */}
        <ModelRow
          icon={<Sparkles className="w-5 h-5 text-blue-300" />}
          title="Inteligencia artificial local"
          subtitle="Genera tips en vivo y evaluaciones detalladas"
          state={coach}
          sizeHint="~2.4 GB"
        />

        {/* Modelo transcripción */}
        <ModelRow
          icon={<Mic2 className="w-5 h-5 text-emerald-300" />}
          title="Transcripción en español"
          subtitle="Convierte tu voz a texto en tiempo real"
          state={transcript}
          sizeHint="~270 MB"
        />

        {hasError && (
          <div className="rounded-lg border border-rose-500/40 bg-rose-500/10 p-4 text-sm">
            <div className="flex items-start gap-2">
              <AlertCircle className="w-4 h-4 text-rose-400 mt-0.5 flex-shrink-0" />
              <div className="flex-1 text-rose-100">
                <div className="font-semibold mb-1">Hubo un problema con la descarga</div>
                <div className="text-xs leading-relaxed">
                  {coach.error || transcript.error}. Verifica tu conexión a internet
                  y reintenta. Si el problema persiste, contacta soporte.
                </div>
              </div>
            </div>
          </div>
        )}

        {allDone && (
          <div className="rounded-lg border border-emerald-500/40 bg-emerald-500/10 p-4 flex items-center gap-2 text-sm">
            <CheckCircle2 className="w-5 h-5 text-emerald-400" />
            <span className="text-emerald-100">
              Pasamos al siguiente paso automáticamente…
            </span>
          </div>
        )}
      </div>
    </OnboardingContainer>
  );
}

interface ModelRowProps {
  icon: React.ReactNode;
  title: string;
  subtitle: string;
  state: DownloadState;
  sizeHint: string;
}

function ModelRow({ icon, title, subtitle, state, sizeHint }: ModelRowProps) {
  const eta = formatEta(state);
  const isDownloading = !state.installed && !state.error && state.startedAt !== null;
  return (
    <div className="rounded-lg border border-white/10 bg-white/5 p-4">
      <div className="flex items-start gap-3">
        <div className="flex-shrink-0 mt-0.5">{icon}</div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center justify-between gap-2 flex-wrap">
            <div className="font-semibold text-gray-50">{title}</div>
            {state.installed ? (
              <span className="flex items-center gap-1 text-xs text-emerald-400">
                <CheckCircle2 className="w-3.5 h-3.5" /> Listo
              </span>
            ) : isDownloading ? (
              <span className="flex items-center gap-1 text-xs text-blue-300">
                <Loader2 className="w-3.5 h-3.5 animate-spin" /> {state.progress}%
              </span>
            ) : (
              <span className="text-xs text-gray-400">Esperando…</span>
            )}
          </div>
          <div className="text-xs text-gray-400 mt-0.5">{subtitle}</div>
          {!state.installed && (
            <>
              <div className="h-1.5 rounded-full bg-white/8 overflow-hidden mt-3">
                <div
                  className="h-full bg-blue-400 transition-all duration-300"
                  style={{ width: `${state.progress}%` }}
                />
              </div>
              <div className="flex items-center justify-between text-[10px] text-gray-400 mt-1.5">
                <span>
                  {state.totalMb > 0
                    ? `${state.downloadedMb}/${state.totalMb} MB`
                    : sizeHint}
                </span>
                {eta && <span className="text-blue-300">{eta}</span>}
              </div>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
