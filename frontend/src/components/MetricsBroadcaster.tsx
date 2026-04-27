'use client';

import { useEffect, useRef } from 'react';
import { emit } from '@tauri-apps/api/event';
import { useRecordingState } from '@/contexts/RecordingStateContext';
import { useTranscripts } from '@/contexts/TranscriptContext';
import { useCoach } from '@/contexts/CoachContext';
import type { Transcript } from '@/types';

/**
 * Broadcaster sin UI: cada 2s mientras isRecording, emite el evento Tauri
 * `meeting-metrics` con health/WPM/duration/tipsCount para que la ventana
 * flotante always-on-top los reciba (Tauri propaga emit cross-window).
 *
 * Calcula:
 * - durationSec: segundos desde recordingStartTime
 * - wpm: total palabras transcritas USER / minutos transcurridos
 * - health: connectionScore del CoachContext (0-100)
 * - tipsCount: longitud de suggestions
 */
export function MetricsBroadcaster() {
  const { isRecording, activeDuration } = useRecordingState();
  const { transcripts } = useTranscripts();
  const { suggestions, metrics: coachMetrics } = useCoach();
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => {
    if (!isRecording) {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
      return;
    }

    const tick = () => {
      const durationSec = Math.max(0, (activeDuration ?? 0));
      const minutes = Math.max(0.05, durationSec / 60);

      const userWords = transcripts
        .filter((t: Transcript) => t.source_type === 'user')
        .reduce(
          (acc: number, t: Transcript) =>
            acc + (t.text || '').trim().split(/\s+/).filter(Boolean).length,
          0
        );
      const wpm = userWords / minutes;

      const health = coachMetrics?.connectionScore ?? 50;
      const tipsCount = suggestions.length;

      emit('meeting-metrics', {
        health,
        wpm,
        durationSec,
        tipsCount,
        connectionScore: coachMetrics?.connectionScore ?? 50,
        connectionTrend: coachMetrics?.connectionTrend ?? 'stable',
      }).catch(() => {
        /* ignore — flotante puede no estar abierta */
      });
    };

    tick();
    intervalRef.current = setInterval(tick, 2000);

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
        intervalRef.current = null;
      }
    };
  }, [isRecording, activeDuration, transcripts, suggestions, coachMetrics]);

  return null;
}
