'use client';

import { useEffect, useMemo, useRef } from 'react';
import { emit } from '@tauri-apps/api/event';
import { getAllWebviewWindows } from '@tauri-apps/api/webviewWindow';
import { useRecordingState } from '@/contexts/RecordingStateContext';
import { useTranscripts } from '@/contexts/TranscriptContext';
import { useCoach } from '@/contexts/CoachContext';
import type { Transcript } from '@/types';

/**
 * Broadcast cross-window robusto. Tauri 2.x: `emit()` global emite a TODAS las
 * webviews por defecto, pero en algunas builds Windows con ventanas
 * `transparent + decorations:false` la propagación falla silenciosamente. Como
 * fallback iteramos las webviews y emitimos directamente. Más caro pero
 * garantiza que la burbuja flotante reciba siempre.
 */
async function broadcastEvent(name: string, payload: unknown) {
  try {
    await emit(name, payload);
  } catch {
    /* ignore — caemos al fallback */
  }
  try {
    const windows = await getAllWebviewWindows();
    await Promise.all(
      windows.map((w) => w.emit(name, payload).catch(() => undefined)),
    );
  } catch {
    /* ignore — la flotante puede no estar abierta */
  }
}

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

  // Pre-cómputo memoizado de talk-time + word counts.
  // Antes: full-loop sobre transcripts cada 2s en `tick` → con 2000 transcripts
  // = 1000 iter/min puro garbage. Ahora se recomputa solo cuando transcripts
  // cambia (eventos discretos al llegar nuevo segment).
  const aggregates = useMemo(() => {
    let userWords = 0;
    let interlocutorWords = 0;
    let userSegmentSec = 0;
    let interlocutorSegmentSec = 0;
    for (const t of transcripts as Transcript[]) {
      const words = (t.text || '').trim().split(/\s+/).filter(Boolean).length;
      const dur = typeof t.duration === 'number' ? t.duration : 0;
      if (t.source_type === 'user') {
        userWords += words;
        userSegmentSec += dur;
      } else if (t.source_type === 'interlocutor') {
        interlocutorWords += words;
        interlocutorSegmentSec += dur;
      }
    }
    return { userWords, interlocutorWords, userSegmentSec, interlocutorSegmentSec };
  }, [transcripts]);

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

      const { userWords, interlocutorWords, userSegmentSec, interlocutorSegmentSec } = aggregates;
      // WPM contextual: si el usuario casi no habla (escucha conferencia/audiencia)
      // mostramos el WPM del interlocutor para que la métrica siga teniendo valor.
      // Si ambos hablan, mostramos el del usuario (rendimiento propio).
      const userIsListening = userWords < 20 && interlocutorWords > userWords * 3;
      const wpm = userIsListening
        ? interlocutorWords / minutes
        : userWords / minutes;

      // Tiempo hablado por persona (segundos): preferir audio_*_time si existe,
      // sino fallback en suma de palabras→segundos (~0.4s/palabra).
      const userSec =
        userSegmentSec > 0 ? userSegmentSec : userWords * 0.4;
      const interlocutorSec =
        interlocutorSegmentSec > 0
          ? interlocutorSegmentSec
          : interlocutorWords * 0.4;
      const totalSpeakSec = userSec + interlocutorSec;
      const userTalkPct = totalSpeakSec > 0 ? (userSec / totalSpeakSec) * 100 : 0;

      const health = coachMetrics?.connectionScore ?? 50;
      const tipsCount = suggestions.length;

      const interlocutorQuestions = (coachMetrics?.questionHistory ?? [])
        .filter((q) => q.speaker === 'interlocutor')
        .slice(-20)
        .map((q) => ({ text: q.text, timestamp: q.timestamp }));

      void broadcastEvent('meeting-metrics', {
        health,
        wpm,
        durationSec,
        tipsCount,
        userTalkSec: userSec,
        interlocutorTalkSec: interlocutorSec,
        userTalkPct,
        userWords,
        interlocutorWords,
        connectionScore: coachMetrics?.connectionScore ?? 50,
        connectionTrend: coachMetrics?.connectionTrend ?? 'stable',
        interlocutorQuestions,
        listeningMode: userIsListening,
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
  }, [isRecording, activeDuration, aggregates, suggestions, coachMetrics]);

  return null;
}
