'use client';

import { useEffect, useState } from 'react';
import { Target, BarChart3, AlertTriangle, Clock } from 'lucide-react';
import { quietInvoke } from '@/lib/safeInvoke';

interface DashboardSummary {
  total_iterations: number;
  iterations_last_7d: number;
  avg_wer_user_30d: number | null;
  avg_wer_interlocutor_30d: number | null;
  avg_evaluation_score_30d: number | null;
  avg_total_pipeline_ms_30d: number | null;
  last_iteration_at: string | null;
  broken_button_count: number;
  untested_button_count: number;
}

export function SummaryKPIs() {
  const [s, setS] = useState<DashboardSummary | null>(null);

  useEffect(() => {
    let cancelled = false;
    const tick = async () => {
      const r = await quietInvoke<DashboardSummary>('dashboard_get_summary');
      if (!cancelled && r) setS(r);
    };
    tick();
    const id = setInterval(tick, 5_000);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, []);

  const werColor = (w: number | null | undefined) =>
    w == null ? 'text-gray-400' : w < 0.1 ? 'text-emerald-400' : w < 0.2 ? 'text-amber-400' : 'text-rose-400';

  return (
    <div className="rounded-xl border border-white/10 bg-white/5 p-4 space-y-3">
      <div className="flex items-center gap-2">
        <BarChart3 className="w-4 h-4 text-indigo-300" />
        <h3 className="text-sm font-semibold text-gray-100">KPIs Globales</h3>
      </div>

      <div className="grid grid-cols-2 gap-2">
        <Kpi
          icon={<Target className="w-3 h-3" />}
          label="Iteraciones"
          value={s?.total_iterations.toString() ?? '–'}
          sub={`${s?.iterations_last_7d ?? 0} últ 7d`}
        />
        <Kpi
          icon={<Clock className="w-3 h-3" />}
          label="Pipeline avg"
          value={s?.avg_total_pipeline_ms_30d != null ? `${(s.avg_total_pipeline_ms_30d / 1000).toFixed(1)}s` : '–'}
          sub="30d"
        />
        <Kpi
          icon={<Target className="w-3 h-3" />}
          label="WER user"
          value={s?.avg_wer_user_30d != null ? `${(s.avg_wer_user_30d * 100).toFixed(1)}%` : '–'}
          color={werColor(s?.avg_wer_user_30d)}
          sub="30d"
        />
        <Kpi
          icon={<Target className="w-3 h-3" />}
          label="WER inter."
          value={s?.avg_wer_interlocutor_30d != null ? `${(s.avg_wer_interlocutor_30d * 100).toFixed(1)}%` : '–'}
          color={werColor(s?.avg_wer_interlocutor_30d)}
          sub="30d"
        />
        <Kpi
          icon={<BarChart3 className="w-3 h-3" />}
          label="Eval score"
          value={s?.avg_evaluation_score_30d != null ? s.avg_evaluation_score_30d.toFixed(1) : '–'}
          sub="0-10"
        />
        <Kpi
          icon={<AlertTriangle className="w-3 h-3" />}
          label="Botones rotos"
          value={s?.broken_button_count.toString() ?? '–'}
          color={s && s.broken_button_count > 0 ? 'text-rose-400' : 'text-emerald-400'}
          sub={`${s?.untested_button_count ?? 0} sin probar`}
        />
      </div>

      <div className="text-[10px] text-gray-500 pt-1 border-t border-white/5">
        Última iteración: {s?.last_iteration_at ?? 'nunca'}
      </div>
    </div>
  );
}

function Kpi({ icon, label, value, sub, color }: { icon: React.ReactNode; label: string; value: string; sub?: string; color?: string }) {
  return (
    <div className="rounded-md bg-black/30 p-2">
      <div className="flex items-center gap-1 text-[10px] uppercase text-gray-400">
        {icon} {label}
      </div>
      <div className={`text-xl font-bold tabular-nums ${color || 'text-gray-100'}`}>{value}</div>
      {sub && <div className="text-[10px] text-gray-500">{sub}</div>}
    </div>
  );
}
