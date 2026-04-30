'use client';

import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';

export interface SystemMetrics {
  ts: number;
  cpu_pct: number;
  ram_used_mb: number;
  ram_total_mb: number;
  process_cpu_pct: number;
  process_ram_mb: number;
  thread_count: number;
}

const MAX_POINTS = 120; // 2 min @ 1Hz

/**
 * Listen al evento `system-metrics` (1 Hz) y mantiene buffer ring de 120 puntos.
 */
export function useSystemMetrics() {
  const [series, setSeries] = useState<SystemMetrics[]>([]);
  const [latest, setLatest] = useState<SystemMetrics | null>(null);

  useEffect(() => {
    const unl = listen<SystemMetrics>('system-metrics', (e) => {
      const m = e.payload;
      setLatest(m);
      setSeries((prev) => {
        const next = [...prev, m];
        if (next.length > MAX_POINTS) next.splice(0, next.length - MAX_POINTS);
        return next;
      });
    });
    return () => {
      unl.then((fn) => fn());
    };
  }, []);

  return { series, latest };
}
