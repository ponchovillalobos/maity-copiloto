'use client';

import { Database } from 'lucide-react';

export interface IterationRow {
  id: number;
  meeting_id: string;
  iteration_label: string | null;
  channel_layout: string;
  total_duration_seconds: number;
  decode_ms: number | null;
  transcribe_user_ms: number | null;
  transcribe_interlocutor_ms: number | null;
  evaluation_ms: number | null;
  total_pipeline_ms: number | null;
  wer_global: number | null;
  wer_user: number | null;
  wer_interlocutor: number | null;
  evaluation_score: number | null;
  evaluation_sections_filled: number | null;
  prompt_version: string;
  coach_model: string;
  evaluation_model: string;
  created_at: string;
}

interface Props {
  iterations: IterationRow[];
  onSelect: (id: number) => void;
}

const werColor = (w: number | null) =>
  w == null ? 'text-gray-500' : w < 0.1 ? 'text-emerald-300' : w < 0.2 ? 'text-amber-300' : 'text-rose-300';

export function IterationsTable({ iterations, onSelect }: Props) {
  return (
    <div className="rounded-xl border border-white/10 bg-white/5 p-4 space-y-3">
      <div className="flex items-center gap-2">
        <Database className="w-4 h-4 text-cyan-300" />
        <h3 className="text-sm font-semibold text-gray-100">Histórico de iteraciones ({iterations.length})</h3>
      </div>

      <div className="overflow-x-auto">
        <table className="w-full text-[11px] text-gray-300">
          <thead className="text-[10px] uppercase text-gray-500 border-b border-white/10">
            <tr>
              <th className="text-left p-1.5">ID</th>
              <th className="text-left p-1.5">Label</th>
              <th className="text-left p-1.5">Layout</th>
              <th className="text-right p-1.5">Dur</th>
              <th className="text-right p-1.5">Pipeline</th>
              <th className="text-right p-1.5">WER user</th>
              <th className="text-right p-1.5">WER inter.</th>
              <th className="text-right p-1.5">Score</th>
              <th className="text-right p-1.5">Secciones</th>
              <th className="text-left p-1.5">Modelos</th>
              <th className="text-left p-1.5">Fecha</th>
              <th className="text-center p-1.5">→</th>
            </tr>
          </thead>
          <tbody>
            {iterations.length === 0 ? (
              <tr>
                <td colSpan={12} className="p-6 text-center text-gray-500">
                  Sin iteraciones todavía. Cargá audios en /dev y volvé acá.
                </td>
              </tr>
            ) : (
              iterations.map((i) => (
                <tr key={i.id} className="border-b border-white/5 hover:bg-white/5 cursor-pointer" onClick={() => onSelect(i.id)}>
                  <td className="p-1.5 tabular-nums">{i.id}</td>
                  <td className="p-1.5 truncate max-w-[150px]" title={i.iteration_label ?? ''}>
                    {i.iteration_label ?? '–'}
                  </td>
                  <td className="p-1.5">{i.channel_layout}</td>
                  <td className="p-1.5 text-right tabular-nums">{Math.round(i.total_duration_seconds)}s</td>
                  <td className="p-1.5 text-right tabular-nums">
                    {i.total_pipeline_ms != null ? `${(i.total_pipeline_ms / 1000).toFixed(1)}s` : '–'}
                  </td>
                  <td className={`p-1.5 text-right tabular-nums ${werColor(i.wer_user)}`}>
                    {i.wer_user != null ? `${(i.wer_user * 100).toFixed(1)}%` : '–'}
                  </td>
                  <td className={`p-1.5 text-right tabular-nums ${werColor(i.wer_interlocutor)}`}>
                    {i.wer_interlocutor != null ? `${(i.wer_interlocutor * 100).toFixed(1)}%` : '–'}
                  </td>
                  <td className="p-1.5 text-right tabular-nums">
                    {i.evaluation_score != null ? i.evaluation_score.toFixed(1) : '–'}
                  </td>
                  <td className="p-1.5 text-right tabular-nums">
                    {i.evaluation_sections_filled != null ? `${i.evaluation_sections_filled}/15` : '–'}
                  </td>
                  <td className="p-1.5 text-[10px] text-gray-400 truncate max-w-[100px]">
                    {i.coach_model.split(':')[1]} / {i.evaluation_model.split(':')[1]}
                  </td>
                  <td className="p-1.5 text-[10px] text-gray-500 whitespace-nowrap">
                    {new Date(i.created_at).toLocaleString('es-MX', { dateStyle: 'short', timeStyle: 'short' })}
                  </td>
                  <td className="p-1.5 text-center text-blue-400">→</td>
                </tr>
              ))
            )}
          </tbody>
        </table>
      </div>
    </div>
  );
}
