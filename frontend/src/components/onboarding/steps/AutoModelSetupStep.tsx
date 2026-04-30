'use client';

import React, { useEffect, useRef, useState } from 'react';
import { CheckCircle2, Loader2, Sparkles } from 'lucide-react';
import { OnboardingContainer } from '../OnboardingContainer';
import { useOnboarding } from '@/contexts/OnboardingContext';
import { listen } from '@tauri-apps/api/event';
import { quietInvoke, safeInvoke } from '@/lib/safeInvoke';
import { logger } from '@/lib/logger';

/**
 * AutoModelSetupStep — verifica que los modelos requeridos estén listos.
 * Si ya están, avanza inmediatamente. Si faltan, los descarga en background
 * SIN exponer detalles técnicos (sin progress bars de GB, sin nombres de
 * modelos, sin botones técnicos). Solo un spinner mientras todo queda listo.
 */

const COACH_MODEL = 'qwen3:0.6b';
const TRANSCRIPT_MODEL = 'parakeet-tdt-0.6b-v3-int8';

export function AutoModelSetupStep() {
  const { goNext } = useOnboarding();
  const [isMac, setIsMac] = useState(false);
  const [coachReady, setCoachReady] = useState(false);
  const [transcriptReady, setTranscriptReady] = useState(false);
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

  useEffect(() => {
    const unsubs: Array<() => void> = [];
    (async () => {
      const u1 = await listen<{ model: string }>('builtin-ai-download-complete', (e) => {
        if (e.payload.model === COACH_MODEL) setCoachReady(true);
      });
      unsubs.push(u1);
      const u2 = await listen<unknown>('parakeet-model-download-complete', () => {
        setTranscriptReady(true);
      });
      unsubs.push(u2);
    })();
    return () => {
      unsubs.forEach((u) => u());
    };
  }, []);

  useEffect(() => {
    if (startedRef.current) return;
    startedRef.current = true;
    (async () => {
      const coachIsReady = await quietInvoke<boolean>('builtin_ai_is_model_ready', { modelName: COACH_MODEL });
      if (coachIsReady) {
        setCoachReady(true);
      } else {
        logger.info('[Onboarding] Iniciando preparación silenciosa…');
        void safeInvoke('builtin_ai_download_model', { modelName: COACH_MODEL }, '');
      }
      void safeInvoke('parakeet_download_model', { modelName: TRANSCRIPT_MODEL }, '');
    })();
  }, []);

  useEffect(() => {
    if (advancedAuto) return;
    if (coachReady && transcriptReady) {
      setAdvancedAuto(true);
      const t = setTimeout(() => goNext(), 800);
      return () => clearTimeout(t);
    }
  }, [coachReady, transcriptReady, advancedAuto, goNext]);

  const allDone = coachReady && transcriptReady;

  return (
    <OnboardingContainer
      title={allDone ? '¡Listo!' : 'Preparando Maity'}
      description={
        allDone
          ? 'Tu copiloto está listo para usar.'
          : 'Estamos terminando la configuración. Esto solo toma unos segundos.'
      }
      step={2}
      totalSteps={isMac ? 6 : 5}
    >
      <div className="flex flex-col items-center justify-center py-12 space-y-6">
        {allDone ? (
          <>
            <div className="rounded-full p-4 bg-emerald-500/20 border border-emerald-500/40">
              <CheckCircle2 className="w-12 h-12 text-emerald-400" />
            </div>
            <p className="text-sm text-emerald-100">Pasamos al siguiente paso automáticamente…</p>
          </>
        ) : (
          <>
            <div className="rounded-full p-4 bg-blue-500/20 border border-blue-500/40 relative">
              <Sparkles className="w-12 h-12 text-blue-300" />
              <Loader2 className="w-6 h-6 text-blue-400 animate-spin absolute -bottom-1 -right-1 bg-gray-900 rounded-full p-0.5" />
            </div>
            <p className="text-sm text-gray-400 text-center max-w-sm">
              Configurando tu copiloto local. Puedes seguir usando tu computadora mientras tanto.
            </p>
          </>
        )}
      </div>
    </OnboardingContainer>
  );
}
