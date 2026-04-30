'use client';

import { FileText } from 'lucide-react';

export function PromptsSummary() {
  const prompts = [
    {
      name: 'Coach tips (live)',
      version: 'v3-lite',
      file: 'coach/prompt.rs',
      model: 'qwen3:0.6b',
      desc: 'Tips ultra-rápidos durante grabación. ChatML + /no_think prefilled.',
    },
    {
      name: 'Evaluación post-meeting',
      version: 'v4-condensado',
      file: 'coach/prompts/evaluation_v4.rs',
      model: 'qwen3:1.7b',
      desc: '15 secciones top-level, JSON estructurado.',
    },
    {
      name: 'Meeting chat semántico',
      version: 'default',
      file: 'coach/meeting_chat.rs',
      model: 'qwen3:1.7b',
      desc: 'Q&A sobre transcripción usando embeddings nomic-embed-text.',
    },
  ];

  return (
    <div className="rounded-xl border border-white/10 bg-white/5 p-4 space-y-3">
      <div className="flex items-center gap-2">
        <FileText className="w-4 h-4 text-yellow-300" />
        <h3 className="text-sm font-semibold text-gray-100">Prompts activos</h3>
      </div>

      <div className="space-y-2">
        {prompts.map((p) => (
          <div key={p.name} className="rounded-md bg-black/30 p-2 text-xs">
            <div className="flex items-center justify-between gap-2">
              <span className="font-medium text-gray-100">{p.name}</span>
              <span className="text-[10px] text-amber-300 px-1.5 py-0.5 rounded bg-amber-500/10">{p.version}</span>
            </div>
            <div className="text-[10px] text-gray-500 mt-0.5">
              <code>{p.file}</code> · {p.model}
            </div>
            <div className="text-[11px] text-gray-400 mt-1">{p.desc}</div>
          </div>
        ))}
      </div>

      <div className="text-[10px] text-gray-500 pt-1 border-t border-white/5">
        Estos prompts son los hardcoded actuales. Para versionar histórico por reunión, ver columna `prompt_version` en `dev_iterations`.
      </div>
    </div>
  );
}
