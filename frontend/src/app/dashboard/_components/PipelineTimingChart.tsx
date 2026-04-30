'use client';

import { Bar, BarChart, CartesianGrid, ResponsiveContainer, Tooltip, XAxis, YAxis, Legend } from 'recharts';
import { Clock } from 'lucide-react';
import type { IterationRow } from './IterationsTable';

interface Props {
  iterations: IterationRow[];
}

export function PipelineTimingChart({ iterations }: Props) {
  const data = iterations
    .slice(0, 20)
    .reverse()
    .map((i) => ({
      label: `#${i.id}`,
      decode: Math.round((i.decode_ms ?? 0) / 100) / 10,
      transcribe_user: Math.round(((i.transcribe_user_ms ?? 0)) / 100) / 10,
      transcribe_inter: Math.round(((i.transcribe_interlocutor_ms ?? 0)) / 100) / 10,
      evaluation: Math.round((i.evaluation_ms ?? 0) / 100) / 10,
    }));

  return (
    <div className="rounded-xl border border-white/10 bg-white/5 p-4 space-y-3">
      <div className="flex items-center gap-2">
        <Clock className="w-4 h-4 text-purple-300" />
        <h3 className="text-sm font-semibold text-gray-100">Timing pipeline (s) — últimas 20 iteraciones</h3>
      </div>

      {data.length === 0 ? (
        <p className="text-xs text-gray-500">Sin iteraciones aún. Carga audio en /dev → vuelve.</p>
      ) : (
        <div className="h-56">
          <ResponsiveContainer width="100%" height="100%">
            <BarChart data={data} stackOffset="sign">
              <CartesianGrid strokeDasharray="3 3" stroke="#ffffff10" />
              <XAxis dataKey="label" tick={{ fontSize: 10, fill: '#888' }} />
              <YAxis tick={{ fontSize: 10, fill: '#888' }} />
              <Tooltip
                contentStyle={{ backgroundColor: '#0a0a0a', border: '1px solid #ffffff20', fontSize: 11 }}
                labelStyle={{ color: '#ccc' }}
              />
              <Legend wrapperStyle={{ fontSize: 11 }} />
              <Bar dataKey="decode" stackId="a" fill="#60a5fa" name="Decode" />
              <Bar dataKey="transcribe_user" stackId="a" fill="#34d399" name="STT user" />
              <Bar dataKey="transcribe_inter" stackId="a" fill="#a78bfa" name="STT inter." />
              <Bar dataKey="evaluation" stackId="a" fill="#fbbf24" name="Evaluación" />
            </BarChart>
          </ResponsiveContainer>
        </div>
      )}
    </div>
  );
}
