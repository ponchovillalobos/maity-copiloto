'use client';

import { useEffect, useState } from 'react';
import { X } from 'lucide-react';
import { quietInvoke } from '@/lib/safeInvoke';
import type { IterationRow } from './IterationsTable';

interface IterationDetail {
  row: IterationRow;
  hypothesis_full: string | null;
  reference_user: string | null;
  reference_interlocutor: string | null;
  audio_user_path: string | null;
  audio_interlocutor_path: string | null;
  cpu_avg_pct: number | null;
  ram_peak_mb: number | null;
  notes: string | null;
}

interface Props {
  iterationId: number | null;
  onClose: () => void;
}

export function IterationDetailModal({ iterationId, onClose }: Props) {
  const [detail, setDetail] = useState<IterationDetail | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (iterationId == null) {
      setDetail(null);
      return;
    }
    setLoading(true);
    quietInvoke<IterationDetail | null>('dashboard_get_iteration_detail', { iterationId })
      .then((d) => setDetail(d ?? null))
      .finally(() => setLoading(false));
  }, [iterationId]);

  if (iterationId == null) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center bg-black/70 backdrop-blur-sm overflow-y-auto p-8"
      onClick={onClose}
    >
      <div
        className="w-full max-w-4xl rounded-xl border border-white/10 bg-gray-900 p-6 shadow-2xl text-gray-100"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold">
            Iteración #{iterationId} {detail?.row.iteration_label ? `— ${detail.row.iteration_label}` : ''}
          </h2>
          <button onClick={onClose} className="text-gray-400 hover:text-white">
            <X className="w-5 h-5" />
          </button>
        </div>

        {loading && <p className="text-sm text-gray-400">Cargando…</p>}
        {!loading && !detail && <p className="text-sm text-rose-400">No se encontró la iteración.</p>}

        {detail && (
          <div className="space-y-4">
            <div className="grid grid-cols-2 md:grid-cols-4 gap-3 text-xs">
              <Stat label="Duration" value={`${Math.round(detail.row.total_duration_seconds)}s`} />
              <Stat label="Pipeline" value={detail.row.total_pipeline_ms != null ? `${(detail.row.total_pipeline_ms / 1000).toFixed(1)}s` : '–'} />
              <Stat label="Decode" value={detail.row.decode_ms != null ? `${detail.row.decode_ms}ms` : '–'} />
              <Stat label="STT user" value={detail.row.transcribe_user_ms != null ? `${(detail.row.transcribe_user_ms / 1000).toFixed(1)}s` : '–'} />
              <Stat label="STT inter" value={detail.row.transcribe_interlocutor_ms != null ? `${(detail.row.transcribe_interlocutor_ms / 1000).toFixed(1)}s` : '–'} />
              <Stat label="Eval" value={detail.row.evaluation_ms != null ? `${(detail.row.evaluation_ms / 1000).toFixed(1)}s` : '–'} />
              <Stat label="WER user" value={detail.row.wer_user != null ? `${(detail.row.wer_user * 100).toFixed(1)}%` : '–'} />
              <Stat label="WER inter" value={detail.row.wer_interlocutor != null ? `${(detail.row.wer_interlocutor * 100).toFixed(1)}%` : '–'} />
              <Stat label="Score eval" value={detail.row.evaluation_score != null ? detail.row.evaluation_score.toFixed(1) : '–'} />
              <Stat label="Secciones" value={detail.row.evaluation_sections_filled != null ? `${detail.row.evaluation_sections_filled}/15` : '–'} />
              <Stat label="Coach" value={detail.row.coach_model} />
              <Stat label="Evaluator" value={detail.row.evaluation_model} />
            </div>

            {detail.audio_user_path && (
              <div className="text-[11px] text-gray-400">
                <b>Audio user</b>: <code className="text-gray-300">{detail.audio_user_path}</code>
              </div>
            )}
            {detail.audio_interlocutor_path && (
              <div className="text-[11px] text-gray-400">
                <b>Audio interlocutor</b>: <code className="text-gray-300">{detail.audio_interlocutor_path}</code>
              </div>
            )}

            {(detail.reference_user || detail.reference_interlocutor) && (
              <div className="grid md:grid-cols-2 gap-3">
                {detail.reference_user && (
                  <div>
                    <h3 className="text-xs font-semibold text-blue-300 mb-1">Ground truth USER</h3>
                    <pre className="text-[11px] text-gray-300 whitespace-pre-wrap bg-black/30 p-3 rounded max-h-40 overflow-y-auto">
                      {detail.reference_user}
                    </pre>
                  </div>
                )}
                {detail.reference_interlocutor && (
                  <div>
                    <h3 className="text-xs font-semibold text-purple-300 mb-1">Ground truth INTERLOCUTOR</h3>
                    <pre className="text-[11px] text-gray-300 whitespace-pre-wrap bg-black/30 p-3 rounded max-h-40 overflow-y-auto">
                      {detail.reference_interlocutor}
                    </pre>
                  </div>
                )}
              </div>
            )}

            {detail.hypothesis_full && (
              <div>
                <h3 className="text-xs font-semibold text-emerald-300 mb-1">Transcripción Maity</h3>
                <pre className="text-[11px] text-gray-300 whitespace-pre-wrap bg-black/30 p-3 rounded max-h-60 overflow-y-auto">
                  {detail.hypothesis_full}
                </pre>
              </div>
            )}

            <a
              href={`/meeting-details?id=${encodeURIComponent(detail.row.meeting_id)}&source=test`}
              className="inline-block text-xs text-blue-400 underline hover:text-blue-300"
            >
              Ver reunión completa →
            </a>
          </div>
        )}
      </div>
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-md bg-black/30 p-2">
      <div className="text-[10px] uppercase text-gray-500">{label}</div>
      <div className="text-sm font-semibold text-gray-100 tabular-nums">{value}</div>
    </div>
  );
}
