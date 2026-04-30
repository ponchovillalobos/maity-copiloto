'use client';

import { Radio } from 'lucide-react';
import { useLiveEvents } from '../_hooks/useLiveEvents';

export function LiveEventsStream() {
  const events = useLiveEvents();
  const recent = [...events].reverse().slice(0, 30);

  return (
    <div className="rounded-xl border border-white/10 bg-white/5 p-4 space-y-3">
      <div className="flex items-center gap-2">
        <Radio className="w-4 h-4 text-rose-300 animate-pulse" />
        <h3 className="text-sm font-semibold text-gray-100">Eventos en vivo (últimos 30)</h3>
      </div>

      {recent.length === 0 ? (
        <p className="text-[11px] text-gray-500">Esperando eventos…</p>
      ) : (
        <div className="space-y-1 max-h-80 overflow-y-auto pr-2">
          {recent.map((e, idx) => (
            <div key={idx} className="text-[10px] flex gap-2 border-b border-white/5 pb-1">
              <span className="text-gray-500 tabular-nums whitespace-nowrap">
                {new Date(e.ts).toLocaleTimeString('es-MX', { hour12: false })}
              </span>
              <span className="text-blue-300 font-mono whitespace-nowrap">{e.name}</span>
              <span className="text-gray-400 truncate flex-1">
                {(() => {
                  try {
                    const s = JSON.stringify(e.payload);
                    return s.length > 120 ? s.slice(0, 120) + '…' : s;
                  } catch {
                    return '[unserializable]';
                  }
                })()}
              </span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
