'use client';

import { CartesianGrid, Line, LineChart, ResponsiveContainer, Tooltip, XAxis, YAxis, Legend } from 'recharts';
import { TrendingUp } from 'lucide-react';
import type { IterationRow } from './IterationsTable';

interface Props {
  iterations: IterationRow[];
}

export function QualityTrendsChart({ iterations }: Props) {
  const data = iterations
    .slice(0, 50)
    .reverse()
    .map((i) => ({
      label: `#${i.id}`,
      wer_user_pct: i.wer_user != null ? Math.round(i.wer_user * 1000) / 10 : null,
      wer_inter_pct: i.wer_interlocutor != null ? Math.round(i.wer_interlocutor * 1000) / 10 : null,
      eval_score: i.evaluation_score != null ? i.evaluation_score : null,
    }));

  const haveAny = data.some(d => d.wer_user_pct != null || d.wer_inter_pct != null || d.eval_score != null);

  return (
    <div className="rounded-xl border border-white/10 bg-white/5 p-4 space-y-3">
      <div className="flex items-center gap-2">
        <TrendingUp className="w-4 h-4 text-emerald-300" />
        <h3 className="text-sm font-semibold text-gray-100">Tendencia de calidad — últimas 50 iteraciones</h3>
      </div>

      {!haveAny ? (
        <p className="text-xs text-gray-500">
          Sin métricas todavía. Carga audios con ground truth en `/dev` → modo QA → texto referencia.
        </p>
      ) : (
        <div className="h-56">
          <ResponsiveContainer width="100%" height="100%">
            <LineChart data={data}>
              <CartesianGrid strokeDasharray="3 3" stroke="#ffffff10" />
              <XAxis dataKey="label" tick={{ fontSize: 10, fill: '#888' }} />
              <YAxis yAxisId="wer" tick={{ fontSize: 10, fill: '#888' }} domain={[0, 'dataMax + 5']} label={{ value: 'WER %', angle: -90, position: 'insideLeft', fill: '#888', fontSize: 10 }} />
              <YAxis yAxisId="score" orientation="right" tick={{ fontSize: 10, fill: '#888' }} domain={[0, 10]} label={{ value: 'Score', angle: 90, position: 'insideRight', fill: '#888', fontSize: 10 }} />
              <Tooltip
                contentStyle={{ backgroundColor: '#0a0a0a', border: '1px solid #ffffff20', fontSize: 11 }}
                labelStyle={{ color: '#ccc' }}
              />
              <Legend wrapperStyle={{ fontSize: 11 }} />
              <Line yAxisId="wer" type="monotone" dataKey="wer_user_pct" stroke="#60a5fa" strokeWidth={2} dot={{ r: 2 }} connectNulls name="WER user %" />
              <Line yAxisId="wer" type="monotone" dataKey="wer_inter_pct" stroke="#a78bfa" strokeWidth={2} dot={{ r: 2 }} connectNulls name="WER inter. %" />
              <Line yAxisId="score" type="monotone" dataKey="eval_score" stroke="#34d399" strokeWidth={2} dot={{ r: 2 }} connectNulls name="Eval score" />
            </LineChart>
          </ResponsiveContainer>
        </div>
      )}
    </div>
  );
}
