'use client';

import { useEffect, useState } from 'react';
import { Cpu, Sparkles } from 'lucide-react';
import { quietInvoke } from '@/lib/safeInvoke';

interface ModelStatus {
  coach: { name: string; ready: boolean };
  evaluation: { name: string; ready: boolean };
  parakeet: { name: string; ready: boolean };
}

const COACH_MODEL = 'qwen3:1.7b';
const EVAL_MODEL = 'qwen3:1.7b';
const PARAKEET_MODEL = 'parakeet-tdt-0.6b-v3-int8';

export function ModelPanel() {
  const [status, setStatus] = useState<ModelStatus | null>(null);

  useEffect(() => {
    let cancelled = false;
    const tick = async () => {
      const [coachReady, evalReady, parakeetReady] = await Promise.all([
        quietInvoke<boolean>('builtin_ai_is_model_ready', { modelName: COACH_MODEL }),
        quietInvoke<boolean>('builtin_ai_is_model_ready', { modelName: EVAL_MODEL }),
        quietInvoke<boolean>('parakeet_is_model_loaded'),
      ]);
      if (cancelled) return;
      setStatus({
        coach: { name: COACH_MODEL, ready: !!coachReady },
        evaluation: { name: EVAL_MODEL, ready: !!evalReady },
        parakeet: { name: PARAKEET_MODEL, ready: !!parakeetReady },
      });
    };
    tick();
    const id = setInterval(tick, 10_000);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, []);

  return (
    <div className="rounded-xl border border-white/10 bg-white/5 p-4 space-y-3">
      <div className="flex items-center gap-2">
        <Sparkles className="w-4 h-4 text-amber-300" />
        <h3 className="text-sm font-semibold text-gray-100">Modelos activos</h3>
      </div>

      <ModelRow label="Coach (tips live)" name={status?.coach.name ?? '…'} ready={status?.coach.ready} purpose="Tips ultra-rápidos" />
      <ModelRow label="Evaluación post" name={status?.evaluation.name ?? '…'} ready={status?.evaluation.ready} purpose="Análisis profundo" />
      <ModelRow label="Transcripción" name={status?.parakeet.name ?? '…'} ready={status?.parakeet.ready} purpose="STT español 16kHz" />

      <div className="text-[10px] text-gray-500 pt-2 border-t border-white/5">
        <Cpu className="inline w-3 h-3 mr-1" />
        100% CPU · Sin GPU · Privacy-first
      </div>
    </div>
  );
}

function ModelRow({ label, name, ready, purpose }: { label: string; name: string; ready?: boolean; purpose: string }) {
  return (
    <div className="flex items-start gap-2 rounded-md bg-black/30 p-2">
      <div className={`w-2 h-2 rounded-full mt-1.5 ${ready ? 'bg-emerald-400' : 'bg-gray-600'}`} />
      <div className="flex-1 min-w-0">
        <div className="flex items-center justify-between gap-2">
          <span className="text-xs font-medium text-gray-100 truncate">{label}</span>
          <span className={`text-[10px] ${ready ? 'text-emerald-300' : 'text-gray-500'}`}>
            {ready ? 'listo' : 'no cargado'}
          </span>
        </div>
        <div className="text-[10px] text-gray-400 truncate">{name}</div>
        <div className="text-[10px] text-gray-500">{purpose}</div>
      </div>
    </div>
  );
}
