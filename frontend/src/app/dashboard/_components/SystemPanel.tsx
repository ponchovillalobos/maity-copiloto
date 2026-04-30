'use client';

import { Cpu, MemoryStick, Activity } from 'lucide-react';
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  Tooltip,
  ResponsiveContainer,
  CartesianGrid,
} from 'recharts';
import { useSystemMetrics } from '../_hooks/useSystemMetrics';

export function SystemPanel() {
  const { latest, series } = useSystemMetrics();

  const ramPct = latest && latest.ram_total_mb > 0
    ? (latest.ram_used_mb / latest.ram_total_mb) * 100
    : 0;

  const chartData = series.map((m, i) => ({
    idx: i,
    cpu: Math.round(m.cpu_pct),
    proc_cpu: Math.round(m.process_cpu_pct),
    ram: Math.round(m.ram_used_mb / 100) / 10, // GB
    proc_ram: Math.round(m.process_ram_mb),
  }));

  return (
    <div className="rounded-xl border border-white/10 bg-white/5 p-4 space-y-3">
      <div className="flex items-center gap-2">
        <Activity className="w-4 h-4 text-blue-300" />
        <h3 className="text-sm font-semibold text-gray-100">Sistema</h3>
      </div>

      <div className="grid grid-cols-2 gap-3">
        <div className="rounded-md bg-black/30 p-2">
          <div className="flex items-center gap-1 text-[10px] uppercase text-gray-400">
            <Cpu className="w-3 h-3" /> CPU global
          </div>
          <div className="text-2xl font-bold text-blue-300 tabular-nums">
            {latest ? Math.round(latest.cpu_pct) : '–'}%
          </div>
          <div className="text-[10px] text-gray-500">
            Proceso: {latest ? latest.process_cpu_pct.toFixed(1) : '–'}%
          </div>
        </div>
        <div className="rounded-md bg-black/30 p-2">
          <div className="flex items-center gap-1 text-[10px] uppercase text-gray-400">
            <MemoryStick className="w-3 h-3" /> RAM
          </div>
          <div className="text-2xl font-bold text-emerald-300 tabular-nums">
            {Math.round(ramPct)}%
          </div>
          <div className="text-[10px] text-gray-500">
            {latest ? `${(latest.ram_used_mb / 1024).toFixed(1)}` : '–'} /
            {latest ? `${(latest.ram_total_mb / 1024).toFixed(1)}` : '–'} GB
            (proceso {latest ? latest.process_ram_mb : 0} MB)
          </div>
        </div>
      </div>

      <div className="h-32 -mx-2">
        <ResponsiveContainer width="100%" height="100%">
          <AreaChart data={chartData}>
            <CartesianGrid strokeDasharray="3 3" stroke="#ffffff10" />
            <XAxis dataKey="idx" hide />
            <YAxis hide domain={[0, 100]} />
            <Tooltip
              contentStyle={{ backgroundColor: '#0a0a0a', border: '1px solid #ffffff20', fontSize: 11 }}
              labelStyle={{ color: '#888' }}
            />
            <Area
              type="monotone"
              dataKey="cpu"
              stroke="#60a5fa"
              fill="#60a5fa20"
              strokeWidth={2}
              dot={false}
              name="CPU %"
            />
            <Area
              type="monotone"
              dataKey="proc_cpu"
              stroke="#34d399"
              fill="#34d39920"
              strokeWidth={1.5}
              dot={false}
              name="Proc %"
            />
          </AreaChart>
        </ResponsiveContainer>
      </div>
      <div className="text-[10px] text-gray-500">
        Threads: {latest?.thread_count ?? '–'} · Refresh 1Hz · 120 ptos máx
      </div>
    </div>
  );
}
