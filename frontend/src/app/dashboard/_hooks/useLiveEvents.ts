'use client';

import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';

interface LiveEvent {
  name: string;
  payload: unknown;
  ts: number;
}

const MAX_EVENTS = 100;

/** Captura un set fijo de eventos Tauri y los mantiene en buffer. */
export function useLiveEvents() {
  const [events, setEvents] = useState<LiveEvent[]>([]);

  useEffect(() => {
    const names = [
      'coach-tip-update',
      'coach-tips-clear',
      'meeting-metrics',
      'dev-import-progress',
      'transcript-update',
      'recording-started',
      'recording-stop-complete',
      'system-metrics',
    ];
    const unsubs: Array<() => void> = [];
    (async () => {
      for (const name of names) {
        const unl = await listen(name, (e) => {
          setEvents((prev) => {
            const next = [
              ...prev,
              { name, payload: e.payload, ts: Date.now() },
            ];
            if (next.length > MAX_EVENTS) next.splice(0, next.length - MAX_EVENTS);
            return next;
          });
        });
        unsubs.push(unl);
      }
    })();
    return () => unsubs.forEach((u) => u());
  }, []);

  return events;
}
