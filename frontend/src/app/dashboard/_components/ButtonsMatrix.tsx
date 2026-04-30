'use client';

import { useEffect, useState } from 'react';
import { CheckCircle2, XCircle, AlertCircle, HelpCircle, Archive, MousePointer2 } from 'lucide-react';
import { quietInvoke, safeInvoke } from '@/lib/safeInvoke';

interface ButtonRow {
  id: string;
  display_name: string;
  source_file: string;
  source_line: number | null;
  category: string;
  status: 'ok' | 'broken' | 'warn' | 'untested' | 'deprecated';
  notes: string | null;
  last_checked_at: string | null;
  last_checked_iteration_id: number | null;
}

const STATUS_OPTIONS: Array<ButtonRow['status']> = ['ok', 'warn', 'broken', 'untested', 'deprecated'];

const statusIcon = (s: ButtonRow['status']) => {
  switch (s) {
    case 'ok': return <CheckCircle2 className="w-3 h-3 text-emerald-400" />;
    case 'broken': return <XCircle className="w-3 h-3 text-rose-400" />;
    case 'warn': return <AlertCircle className="w-3 h-3 text-amber-400" />;
    case 'deprecated': return <Archive className="w-3 h-3 text-gray-500" />;
    default: return <HelpCircle className="w-3 h-3 text-gray-400" />;
  }
};

export function ButtonsMatrix() {
  const [rows, setRows] = useState<ButtonRow[]>([]);
  const [filter, setFilter] = useState<string>('all');

  const reload = async () => {
    const r = await quietInvoke<ButtonRow[]>('dashboard_list_buttons');
    if (r) setRows(r);
  };

  useEffect(() => {
    quietInvoke('dashboard_seed_buttons').then(reload);
  }, []);

  const updateStatus = async (id: string, status: string, notes: string | null) => {
    await safeInvoke('dashboard_update_button_status', { buttonId: id, status, notes }, 'No se pudo actualizar.');
    reload();
  };

  const filtered = filter === 'all' ? rows : rows.filter((r) => r.category === filter || r.status === filter);
  const categories = Array.from(new Set(rows.map((r) => r.category)));

  return (
    <div className="rounded-xl border border-white/10 bg-white/5 p-4 space-y-3">
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-2">
          <MousePointer2 className="w-4 h-4 text-pink-300" />
          <h3 className="text-sm font-semibold text-gray-100">Matriz de botones ({rows.length})</h3>
        </div>
        <select
          value={filter}
          onChange={(e) => setFilter(e.target.value)}
          className="bg-black/40 border border-white/15 rounded text-xs px-2 py-1 text-gray-200"
        >
          <option value="all">Todos</option>
          <optgroup label="Por categoría">
            {categories.map((c) => (
              <option key={c} value={c}>{c}</option>
            ))}
          </optgroup>
          <optgroup label="Por status">
            {STATUS_OPTIONS.map((s) => (
              <option key={s} value={s}>{s}</option>
            ))}
          </optgroup>
        </select>
      </div>

      <div className="overflow-x-auto">
        <table className="w-full text-[11px] text-gray-300">
          <thead className="text-[10px] uppercase text-gray-500 border-b border-white/10">
            <tr>
              <th className="text-left p-1.5 w-24">Status</th>
              <th className="text-left p-1.5">Botón</th>
              <th className="text-left p-1.5">Categoría</th>
              <th className="text-left p-1.5">Archivo</th>
              <th className="text-left p-1.5">Notas</th>
              <th className="text-left p-1.5">Última check</th>
            </tr>
          </thead>
          <tbody>
            {filtered.map((b) => (
              <tr key={b.id} className="border-b border-white/5 hover:bg-white/5">
                <td className="p-1.5">
                  <div className="flex items-center gap-1">
                    {statusIcon(b.status)}
                    <select
                      value={b.status}
                      onChange={(e) => updateStatus(b.id, e.target.value, b.notes)}
                      className="bg-transparent text-[10px] text-gray-200 outline-none cursor-pointer"
                    >
                      {STATUS_OPTIONS.map((s) => (
                        <option key={s} value={s}>{s}</option>
                      ))}
                    </select>
                  </div>
                </td>
                <td className="p-1.5">
                  <div className="font-medium text-gray-100">{b.display_name}</div>
                  <div className="text-[10px] text-gray-500">{b.id}</div>
                </td>
                <td className="p-1.5"><span className="px-1.5 py-0.5 rounded bg-white/10 text-[10px]">{b.category}</span></td>
                <td className="p-1.5 text-[10px] text-gray-400 truncate max-w-[200px]" title={b.source_file}>
                  {b.source_file.split('/').pop()}{b.source_line ? `:${b.source_line}` : ''}
                </td>
                <td className="p-1.5">
                  <input
                    type="text"
                    placeholder="–"
                    defaultValue={b.notes ?? ''}
                    onBlur={(e) => {
                      if (e.target.value !== (b.notes ?? '')) {
                        updateStatus(b.id, b.status, e.target.value || null);
                      }
                    }}
                    className="w-full bg-transparent border border-transparent hover:border-white/15 focus:border-blue-400 rounded px-1 text-[11px] text-gray-200 outline-none"
                  />
                </td>
                <td className="p-1.5 text-[10px] text-gray-500 whitespace-nowrap">
                  {b.last_checked_at
                    ? new Date(b.last_checked_at).toLocaleString('es-MX', { dateStyle: 'short', timeStyle: 'short' })
                    : 'nunca'}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <div className="text-[10px] text-gray-500">
        Tip: cambiá el dropdown de status, agrega notas (blur guarda). Datos persisten en SQLite.
      </div>
    </div>
  );
}
