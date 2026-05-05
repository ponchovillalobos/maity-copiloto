'use client';

import React, { useEffect, useState } from 'react';
import { listen, emit } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import {
  X, Minimize2, Maximize2, Sparkles,
  Activity, Timer, HelpCircle, Zap,
} from 'lucide-react';
import { categoryMeta, priorityMeta } from '@/components/Coach/tipMeta';

interface CoachTip {
  tip: string;
  category?: string;
  priority?: 'critical' | 'important' | 'soft' | 'high' | 'medium' | 'low';
  confidence?: number;
  tip_type?: string;
  timestamp?: number;
  id?: number;
}

interface InterlocutorQuestion {
  text: string;
  timestamp: number;
}

interface MeetingMetrics {
  health?: number;
  wpm?: number;
  durationSec?: number;
  tipsCount?: number;
  userTalkSec?: number;
  interlocutorTalkSec?: number;
  userTalkPct?: number;
  userWords?: number;
  interlocutorWords?: number;
  interlocutorQuestions?: InterlocutorQuestion[];
  connectionScore?: number;
  connectionTrend?: 'up' | 'down' | 'flat' | 'stable' | 'rising' | 'falling';
}

type Section = 'tip' | 'questions';

function formatDuration(sec: number): string {
  const m = Math.floor(sec / 60);
  const s = Math.floor(sec % 60);
  return `${m}:${s.toString().padStart(2, '0')}`;
}

// priorityHex/priorityLabel/categoryMeta vienen de tipMeta.ts (compartidos
// con CoachPanel principal — etiquetas idénticas garantizadas).

function healthColor(score: number): string {
  if (score >= 70) return '#1bea9a';
  if (score >= 40) return '#f59e0b';
  return '#ff0050';
}

/** v32.3: zonas semáforo del WPM USUARIO. Verde/ámbar/rojo según ritmo de habla.
 *  - 0           → "Escuchando" (gris)
 *  - 1-139       → "Bien" (verde)
 *  - 140-179     → "Acelera" (ámbar)
 *  - 180+        → "Lento" (rojo, dispara warn=true para pulso visual). */
function wpmZone(wpm: number): { label: string; color: string; warn: boolean } {
  if (wpm <= 0) return { label: 'Escuchando', color: 'rgba(255,255,255,0.45)', warn: false };
  if (wpm < 140) return { label: 'Bien',       color: '#1bea9a',                warn: false };
  if (wpm < 180) return { label: 'Acelera',    color: '#f59e0b',                warn: false };
  return         { label: 'Lento',             color: '#ff0050',                warn: true };
}

function trendArrow(trend?: string): string {
  if (trend === 'rising' || trend === 'up') return '▲';
  if (trend === 'falling' || trend === 'down') return '▼';
  return '·';
}

/** v32.3: panel "Pulso conversacional" — fusiona Salud + WPM + Tiempo + barra
 *  dual de tiempo de palabra en UN solo widget. Reemplaza el HealthGauge SVG
 *  + 2 MetricCards + barra suelta de v32.2. Lectura de un vistazo: 4 datos
 *  clave en una sola pasada de ojos. */
/** Mini sparkline SVG del historial de WPM. Últimos N valores, línea suave. */
function WpmSparkline({ values, color }: { values: number[]; color: string }) {
  if (values.length < 2) return null;
  const W = 68, H = 16;
  const maxV = Math.max(...values, 160);
  const pts = values
    .map((v, i) => {
      const x = (i / (values.length - 1)) * W;
      const y = H - (v / maxV) * H;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    })
    .join(' ');
  return (
    <svg width={W} height={H} style={{ display: 'block', overflow: 'visible', marginTop: 2 }}>
      <polyline
        points={pts}
        fill="none"
        stroke={color}
        strokeWidth={1.5}
        strokeLinecap="round"
        strokeLinejoin="round"
        opacity={0.75}
      />
      {/* punto final — valor actual */}
      <circle
        cx={(((values.length - 1) / (values.length - 1)) * W).toFixed(1)}
        cy={(H - (values[values.length - 1] / maxV) * H).toFixed(1)}
        r={2.5}
        fill={color}
        opacity={0.95}
      />
    </svg>
  );
}

function PulsoPanel({
  health,
  wpm,
  wpmHistory,
  duration,
  userTalkPct,
  trend,
}: {
  health: number;
  wpm: number;
  wpmHistory: number[];
  duration: number;
  userTalkPct: number;
  trend?: string;
}) {
  const hColor = healthColor(health);
  const zone = wpmZone(wpm);
  const arrow = trendArrow(trend);
  return (
    <div
      className="rounded-xl border border-white/12 bg-white/4 px-3 py-2.5 mb-3 flex-shrink-0"
      data-tauri-drag-region
      style={{ backdropFilter: 'blur(6px)' }}
    >
      {/* TOP ROW — 3 chips: Salud · WPM · Tiempo */}
      <div className="flex items-center justify-between gap-2 mb-2" data-tauri-drag-region>
        {/* SALUD */}
        <div className="flex flex-col items-center min-w-0 flex-1" data-tauri-drag-region>
          <div className="flex items-baseline gap-1">
            <span className="text-[28px] font-bold tabular-nums leading-none" style={{ color: hColor }}>
              {Math.round(health)}
            </span>
            <span className="text-[12px] font-bold leading-none" style={{ color: hColor }} title={`Tendencia: ${trend ?? 'estable'}`}>
              {arrow}
            </span>
          </div>
          <div className="text-[9px] uppercase tracking-wider text-white/55 mt-0.5">Salud</div>
        </div>
        {/* separador vertical */}
        <div className="w-px h-10 bg-white/10" />
        {/* WPM USUARIO */}
        <div
          className="flex flex-col items-center min-w-0 flex-1 relative"
          data-tauri-drag-region
          style={zone.warn ? { animation: 'wpmPulse 800ms ease-in-out infinite' } : undefined}
        >
          <div className="flex items-baseline gap-1">
            <Activity className="w-3 h-3" style={{ color: zone.color }} />
            <span className="text-[28px] font-bold tabular-nums leading-none" style={{ color: zone.color }}>
              {Math.round(wpm)}
            </span>
            <span className="text-[10px] font-semibold uppercase tracking-wider" style={{ color: zone.color }}>
              wpm
            </span>
          </div>
          {wpmHistory.length >= 2
            ? <WpmSparkline values={wpmHistory} color={zone.color} />
            : <div className="text-[9px] uppercase tracking-wider mt-0.5" style={{ color: zone.color }}>{zone.label}</div>
          }
        </div>
        {/* separador vertical */}
        <div className="w-px h-10 bg-white/10" />
        {/* TIEMPO TOTAL */}
        <div className="flex flex-col items-center min-w-0 flex-1" data-tauri-drag-region>
          <div className="flex items-baseline gap-1">
            <Timer className="w-3 h-3 text-[#a8b3ff]" />
            <span className="text-[22px] font-bold tabular-nums leading-none text-[#a8b3ff]">
              {formatDuration(duration)}
            </span>
          </div>
          <div className="text-[9px] uppercase tracking-wider text-white/55 mt-0.5">Sesión</div>
        </div>
      </div>
      {/* BOTTOM ROW — barra dual de tiempo de palabra integrada */}
      <div data-tauri-drag-region>
        <div
          className="flex h-3.5 rounded-full overflow-hidden border border-white/10"
          style={{ background: 'rgba(255,255,255,0.05)' }}
          data-tauri-drag-region
        >
          <div
            className="flex items-center justify-center text-[9px] font-bold text-white transition-all duration-300"
            style={{
              width: `${Math.max(0, Math.min(100, userTalkPct))}%`,
              background: '#485df4',
            }}
          >
            {userTalkPct > 18 ? `Tú ${Math.round(userTalkPct)}%` : ''}
          </div>
          <div
            className="flex items-center justify-center text-[9px] font-bold text-white transition-all duration-300"
            style={{
              width: `${Math.max(0, Math.min(100, 100 - userTalkPct))}%`,
              background: '#1bea9a',
            }}
          >
            {100 - userTalkPct > 18 ? `Otro ${Math.round(100 - userTalkPct)}%` : ''}
          </div>
        </div>
      </div>
    </div>
  );
}

export default function FloatingPage() {
  const [compact, setCompact] = useState(false);
  // v32.2: opacidad ajustable de la burbuja (persistida en localStorage).
  const [bgOpacity, setBgOpacity] = useState<number>(() => {
    if (typeof window === 'undefined') return 0.92;
    const v = parseFloat(window.localStorage.getItem('maity_floating_opacity') || '');
    return Number.isFinite(v) && v >= 0.3 && v <= 1.0 ? v : 0.92;
  });
  const updateOpacity = (v: number) => {
    setBgOpacity(v);
    try { window.localStorage.setItem('maity_floating_opacity', String(v)); } catch {}
  };
  const [tipsHistory, setTipsHistory] = useState<CoachTip[]>([]);
  const [tipIndex, setTipIndex] = useState(0); // 0 = most recent
  const [metrics, setMetrics] = useState<MeetingMetrics>({});
  const [wpmHistory, setWpmHistory] = useState<number[]>([]);
  const [section, setSection] = useState<Section>('tip');
  const [requestingTip, setRequestingTip] = useState(false);
  // v32.2: feedback efímero del botón "Pedir tip ahora" (None del backend, error, etc.).
  const [tipRequestStatus, setTipRequestStatus] = useState<string | null>(null);

  useEffect(() => {
    const unlisteners: Array<() => void> = [];
    let pollTimer: ReturnType<typeof setInterval> | null = null;
    let lastSeenId = 0;

    // BUG #15 fix (2026-05-02): SIMPLIFICACIÓN RADICAL.
    // El flujo de eventos cross-webview (Tauri emit + listen) demostró ser
    // frágil incluso después de v29.2/v29.3/v29.4 (backend `app.emit` global,
    // `emit_to` específico, bridge command). El usuario sigue sin ver tips.
    // Solución pragmática: la burbuja POLLEA `coach_get_recent_tips` cada 3s
    // y muestra cualquier tip nuevo (id > lastSeenId). Sin eventos, sin
    // dedup complejo, sin race conditions. La DB es la única fuente de verdad.
    const fetchAndAppend = async (initial: boolean = false) => {
      try {
        // BUG #16 fix (asamblea): Tauri 2 aísla sessionStorage por origin
        // (main `/` vs floating `/floating`), así que la burbuja NUNCA podía
        // leer el meeting_id escrito por TranscriptContext en sessionStorage.
        // Ahora consultamos el AppState compartido vía comando Rust.
        const activeMeetingId = await invoke<string | null>('get_active_meeting_id').catch(() => null);
        if (!activeMeetingId) {
          if (initial) console.warn('[floating] active_meeting_id es null — burbuja vacía hasta iniciar grabación');
          return;
        }
        const recent = await invoke<(CoachTip & { id?: number })[]>('coach_get_recent_tips', {
          meetingId: activeMeetingId,
          limit: 50,
        });
        if (!recent || recent.length === 0) return;
        if (initial) {
          // Carga histórica completa
          setTipsHistory(recent);
          setTipIndex(0);
          // Marca el id máximo conocido (orden DESC en backend, primer = más reciente)
          const maxId = Math.max(...recent.map((r) => r.id ?? 0));
          if (maxId > 0) lastSeenId = maxId;
        } else {
          // Filtra tips nuevos por id > lastSeenId
          const fresh = recent.filter((r) => (r.id ?? 0) > lastSeenId);
          if (fresh.length === 0) return;
          // Backend devuelve DESC, fresh está ordenado DESC también
          setTipsHistory((prev) => {
            // Evita duplicados defensivamente
            const seen = new Set(prev.map((p) => p.tip + '|' + (p.timestamp ?? 0)));
            const merged = [...fresh.filter((f) => !seen.has(f.tip + '|' + (f.timestamp ?? 0))), ...prev];
            return merged.slice(0, 50);
          });
          setTipIndex(0);
          setRequestingTip(false);
          const maxId = Math.max(...fresh.map((r) => r.id ?? 0));
          if (maxId > lastSeenId) lastSeenId = maxId;
          console.info(`[floating] +${fresh.length} tips nuevos via poll (lastSeenId=${lastSeenId})`);
        }
      } catch (e) {
        console.warn('[floating] poll failed:', e);
      }
    };

    fetchAndAppend(true);
    pollTimer = setInterval(() => fetchAndAppend(false), 3000);
    // v30: el ciclo de generación de tips (cada 30s) lo dispara el
    // CoachContext (en webview "main") porque allí vive `transcriptsRef`
    // con el contexto en vivo. La burbuja solo CONSUME — escucha eventos
    // y pollea DB.

    // v31: ELIMINADO listener de `coach-tip-update`. La DB es la única fuente
    // de verdad: el polling cada 3s garantiza que la burbuja muestre cualquier
    // tip nuevo en <3s tras INSERT. Sin eventos cross-webview = sin race
    // conditions ni duplicados con el polling.

    listen('coach-tips-clear', () => {
      setTipsHistory([]);
      setTipIndex(0);
      lastSeenId = 0;
    }).then((u) => unlisteners.push(u));

    listen<MeetingMetrics>('meeting-metrics', (e) => {
      setMetrics(e.payload);
      if (typeof e.payload.wpm === 'number' && e.payload.wpm > 0) {
        setWpmHistory((prev) => [...prev.slice(-29), e.payload.wpm!]);
      }
    }).then((u) => unlisteners.push(u));

    return () => {
      if (pollTimer) clearInterval(pollTimer);
      unlisteners.forEach((u) => u());
    };
  }, []);

  const handleClose = async () => {
    try { await invoke('close_floating_coach'); } catch (e) { console.error(e); }
  };

  const handleToggleCompact = async () => {
    const next = !compact;
    setCompact(next);
    try { await invoke('floating_toggle_compact', { compact: next }); } catch (e) { console.error(e); }
  };

  // Drag manual robusto: en Windows con transparent+decorations:false el atributo
  // data-tauri-drag-region a veces no funciona. Llamamos startDragging() programático
  // en cualquier mousedown que NO ocurra sobre elementos interactivos (button, a, input).
  const handleDragMouseDown = async (e: React.MouseEvent) => {
    if (e.button !== 0) return;
    const target = e.target as HTMLElement;
    if (target.closest('button, a, input, textarea, select, [role="button"]')) {
      return;
    }
    try {
      const { getCurrentWebviewWindow } = await import('@tauri-apps/api/webviewWindow');
      await getCurrentWebviewWindow().startDragging();
    } catch (err) {
      console.warn('startDragging failed', err);
    }
  };

  const tip = tipsHistory[tipIndex];
  const totalTips = tipsHistory.length;
  const canPrev = tipIndex < totalTips - 1;
  const canNext = tipIndex > 0;
  const goPrev = () => canPrev && setTipIndex((i) => i + 1);
  const goNext = () => canNext && setTipIndex((i) => i - 1);

  // Las métricas vienen vía evento `meeting-metrics` del MetricsBroadcaster
  // (ver components/MetricsBroadcaster.tsx). Los campos son: health, wpm,
  // userTalkPct, userTalkSec, interlocutorTalkSec, durationSec.
  const health = metrics.health ?? metrics.connectionScore ?? 50;
  const wpm = metrics.wpm ?? 0;
  const duration = metrics.durationSec ?? 0;
  const prio = priorityMeta(tip?.priority);
  const cat = categoryMeta(tip?.category);
  const tipColor = prio.hex;
  const questions = metrics.interlocutorQuestions ?? [];

  if (compact) {
    return (
      <div
        onMouseDown={handleDragMouseDown}
        className="h-screen w-screen flex flex-col p-2 select-none"
        style={{
          background: 'rgba(15, 16, 24, 0.88)',
          backdropFilter: 'blur(18px) saturate(180%)',
          WebkitBackdropFilter: 'blur(18px) saturate(180%)',
          border: '1px solid rgba(255,255,255,0.12)',
          borderRadius: 12,
          boxShadow: '0 8px 32px rgba(0,0,0,0.4)',
        }}
        data-tauri-drag-region
      >
        <div className="flex items-center justify-between mb-1.5" data-tauri-drag-region>
          <div className="flex items-center gap-1 text-[9px] uppercase font-bold tracking-wider text-white/70" data-tauri-drag-region>
            <Sparkles className="w-3 h-3 text-[#485df4]" /> Maity
          </div>
          <div className="flex items-center gap-0.5">
            <button onClick={handleToggleCompact} className="p-0.5 hover:bg-white/15 rounded text-white/70" title="Expandir">
              <Maximize2 className="w-3 h-3" />
            </button>
            <button onClick={handleClose} className="p-0.5 hover:bg-red-500/30 rounded text-white/70" title="Cerrar">
              <X className="w-3 h-3" />
            </button>
          </div>
        </div>
        <div className="flex-1 flex items-center gap-2 px-1" data-tauri-drag-region>
          <div className="text-3xl font-bold tabular-nums leading-none" style={{ color: healthColor(health) }}>
            {Math.round(health)}
          </div>
          <div className="flex-1 min-w-0">
            <div className="text-[10px] text-white/55 uppercase tracking-wider mb-0.5">
              {tip ? prio.label : 'Esperando'}
            </div>
            <div className="text-[11px] text-white/95 line-clamp-2 leading-tight font-medium">
              {tip?.tip || 'Sin tips aún'}
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div
      onMouseDown={handleDragMouseDown}
      className="h-screen w-screen flex flex-col p-3 select-none text-white overflow-hidden"
      style={{
        background: `rgba(15, 16, 24, ${bgOpacity})`,
        backdropFilter: 'blur(22px) saturate(180%)',
        WebkitBackdropFilter: 'blur(22px) saturate(180%)',
        border: '1px solid rgba(255,255,255,0.14)',
        borderRadius: 14,
        boxShadow: '0 12px 40px rgba(0,0,0,0.5)',
      }}
      data-tauri-drag-region
    >
      <div className="flex items-center justify-between mb-3 flex-shrink-0" data-tauri-drag-region>
        <div className="flex items-center gap-1.5 text-xs" data-tauri-drag-region>
          <div className="flex items-center gap-1.5" data-tauri-drag-region>
            <div className="relative w-2 h-2">
              <div className="absolute inset-0 rounded-full bg-[#1bea9a] animate-ping opacity-75"/>
              <div className="absolute inset-0 rounded-full bg-[#1bea9a]"/>
            </div>
            <span className="font-bold tracking-wider text-white">MAITY COACH</span>
          </div>
          {totalTips > 0 && (
            <span className="ml-1 text-[9px] px-1.5 py-0.5 rounded bg-[#485df4]/30 text-[#a8b3ff] font-semibold">
              {totalTips} tip{totalTips !== 1 ? 's' : ''}
            </span>
          )}
        </div>
        <div className="flex items-center gap-1">
          {/* v32.2: slider opacidad */}
          <div className="flex items-center gap-1 px-1.5 rounded bg-white/5" title="Opacidad de la burbuja">
            <span className="text-[9px] text-white/50">◔</span>
            <input
              type="range"
              min={0.3}
              max={1.0}
              step={0.05}
              value={bgOpacity}
              onChange={(e) => updateOpacity(parseFloat(e.target.value))}
              className="w-12 h-1 cursor-pointer accent-[#485df4]"
              onMouseDown={(e) => e.stopPropagation()}
            />
            <span className="text-[9px] text-white/50">●</span>
          </div>
          <button
            onClick={() => invoke('recenter_floating_coach').catch(console.error)}
            className="p-1 hover:bg-white/15 rounded text-white/70 transition"
            title="Recentrar ventana"
          >
            <Maximize2 className="w-3.5 h-3.5" />
          </button>
          <button
            onClick={handleToggleCompact}
            className="p-1 hover:bg-white/15 rounded text-white/70 transition"
            title="Modo compacto"
          >
            <Minimize2 className="w-3.5 h-3.5" />
          </button>
          <button
            onClick={handleClose}
            className="p-1 hover:bg-red-500/30 hover:text-red-300 rounded text-white/70 transition"
            title="Cerrar"
          >
            <X className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      {/* v32.3: Panel "Pulso conversacional" — Salud + WPM USUARIO + Tiempo
          + barra dual de tiempo de palabra fusionados en un solo widget para
          lectura "de un vistazo". Reemplaza HealthGauge SVG + 2 MetricCards
          + barra suelta de v32.2. */}
      <PulsoPanel
        health={health}
        wpm={wpm}
        wpmHistory={wpmHistory}
        duration={duration}
        userTalkPct={metrics.userTalkPct ?? 0}
        trend={metrics.connectionTrend}
      />

      {/* SECCIÓN SELECTORA: TIP / PREGUNTAS */}
      <div className="flex gap-1 mb-2 flex-shrink-0">
        <button
          onClick={() => setSection('tip')}
          className={`flex-1 flex items-center justify-center gap-1 px-2 py-1.5 text-[10px] font-bold uppercase tracking-wider rounded transition ${
            section === 'tip'
              ? 'bg-white/12 text-white'
              : 'bg-white/4 text-white/55 hover:bg-white/8'
          }`}
        >
          <Sparkles className="w-3 h-3" /> Tip {totalTips > 0 ? `(${totalTips})` : ''}
        </button>
        <button
          onClick={() => setSection('questions')}
          className={`flex-1 flex items-center justify-center gap-1 px-2 py-1.5 text-[10px] font-bold uppercase tracking-wider rounded transition ${
            section === 'questions'
              ? 'bg-white/12 text-white'
              : 'bg-white/4 text-white/55 hover:bg-white/8'
          }`}
        >
          <HelpCircle className="w-3 h-3" /> Preguntas {questions.length > 0 ? `(${questions.length})` : ''}
        </button>
      </div>

      {/* v26.2: SECCIÓN TIPS — vista scroll con TODOS los tips visibles. */}
      {section === 'tip' && (
        <div className="flex-1 rounded-lg border p-2 flex flex-col min-h-0 overflow-hidden bg-white/4 border-white/10">
          <div className="flex items-center justify-between mb-2 flex-shrink-0 px-1">
            <span className="text-[10px] font-bold uppercase tracking-wider text-white/70">
              Tips coach ({totalTips})
            </span>
            {requestingTip && (
              <span className="text-[10px] text-white/60 italic flex items-center gap-1">
                <span className="w-2.5 h-2.5 rounded-full border-2 border-white/30 border-t-white animate-spin" />
                generando…
              </span>
            )}
          </div>

          <div className="flex-1 overflow-y-auto custom-scrollbar space-y-2 pr-1">
            {/* v28.1: animación shimmer cuando está generando — feedback visual constante. */}
            {requestingTip && (
              <div
                className="rounded-md border p-3 mb-1 relative overflow-hidden"
                style={{
                  borderColor: 'rgba(168, 139, 250, 0.5)',
                  background: 'linear-gradient(135deg, rgba(168,139,250,0.18) 0%, rgba(72,93,244,0.12) 100%)',
                }}
              >
                <div className="absolute inset-0 -translate-x-full animate-[shimmer_1.6s_ease-in-out_infinite]" style={{
                  background: 'linear-gradient(90deg, transparent 0%, rgba(255,255,255,0.18) 50%, transparent 100%)',
                }} />
                <div className="relative flex items-center gap-2">
                  <div className="flex gap-1">
                    <span className="w-1.5 h-1.5 rounded-full bg-purple-300 animate-bounce" style={{ animationDelay: '0ms' }} />
                    <span className="w-1.5 h-1.5 rounded-full bg-purple-300 animate-bounce" style={{ animationDelay: '150ms' }} />
                    <span className="w-1.5 h-1.5 rounded-full bg-purple-300 animate-bounce" style={{ animationDelay: '300ms' }} />
                  </div>
                  <span className="text-[11px] font-bold tracking-wider text-purple-200">
                    Coach analizando conversación…
                  </span>
                </div>
                <div className="text-[10px] text-white/55 mt-1.5 relative">
                  Tip personalizado en camino · ~4-10s
                </div>
              </div>
            )}
            {tipsHistory.length === 0 && !requestingTip ? (
              <div className="text-white/55 text-xs italic text-center py-6">
                Sin tips aún. Empezá grabación o presioná "Pedir tip ahora".
              </div>
            ) : tipsHistory.length === 0 ? (
              <div className="rounded-md border border-blue-500/30 bg-blue-500/5 p-3 text-center">
                <div className="text-[11px] text-blue-300/90 font-semibold mb-1">
                  Esperando primer tip…
                </div>
                <div className="text-[10px] text-white/60 leading-tight">
                  Los tips se generan automáticamente cada ~60s durante la grabación,
                  o pulsa <span className="font-bold text-white">"Pedir tip ahora"</span> abajo.
                </div>
                <div className="text-[9px] text-white/40 mt-2">
                  Si no aparecen, verifica que la grabación esté activa.
                </div>
              </div>
            ) : (
              tipsHistory.map((t, idx) => {
                // v32.2: tarjetas opacas (rgba alpha 0.95) — antes 0.6 las hacía
                // ilegibles sobre fondo claro. Border 5px izquierdo color categoría.
                const tcat = categoryMeta(t.category);
                const tcolor = tcat.hex;
                const isLatest = idx === 0;
                const icon = tcat.icon ?? '💡';
                return (
                  <div
                    key={`${t.tip}-${idx}`}
                    className="tip-card-v32 rounded-md p-3"
                    style={{
                      background: `linear-gradient(135deg, ${tcolor}25 0%, rgba(10,10,15,0.95) 100%)`,
                      borderLeft: `5px solid ${tcolor}`,
                      borderTop: '1px solid rgba(255,255,255,0.08)',
                      borderRight: '1px solid rgba(255,255,255,0.08)',
                      borderBottom: '1px solid rgba(255,255,255,0.08)',
                      animation: isLatest ? 'tipSlideIn 200ms ease-out' : undefined,
                      boxShadow: isLatest ? `0 4px 20px ${tcolor}55` : undefined,
                    }}
                  >
                    <div className="flex items-center gap-2 mb-1.5">
                      <span style={{ fontSize: '24px', lineHeight: 1 }}>{icon}</span>
                      <span style={{ color: tcolor, fontSize: '15px', fontWeight: 700, letterSpacing: '0.5px' }}>
                        {tcat.label}
                      </span>
                      {isLatest && (
                        <span className="text-[8px] px-1.5 py-0.5 rounded bg-emerald-500/20 text-emerald-300 font-semibold ml-auto">
                          NUEVO
                        </span>
                      )}
                      <span className={`text-[8px] text-white/40 ${isLatest ? '' : 'ml-auto'}`}>
                        #{tipsHistory.length - idx}
                      </span>
                    </div>
                    <div className="text-[13px] text-white leading-snug">
                      {t.tip}
                    </div>
                  </div>
                );
              })
            )}
          </div>
          {/* Botón "Pedir tip ahora" — invoca directamente coach_request_simple_tip
              que ejecuta coach_simple_tick en backend. El tip aparece via polling
              de coach_get_recent_tips (próximo ciclo 3s). v31: una sola ruta. */}
{/* v32.2: el botón ahora ESPERA la respuesta del backend y muestra el
              motivo si devolvió None (sin transcript / dedup / parser rechazó).
              El polling cada 3s lo actualiza igual cuando entra un tip OK. */}
          <button
            onClick={async () => {
              setRequestingTip(true);
              setTipRequestStatus(null);
              try {
                const res = await invoke<unknown>('coach_request_simple_tip', {});
                if (res === null || res === undefined) {
                  setTipRequestStatus('Sin tip ahora — espera 30s o habla más.');
                } else {
                  setTipRequestStatus('Tip generado ✓');
                }
              } catch (e) {
                console.warn('[floating] coach_request_simple_tip failed:', e);
                setTipRequestStatus(`Error: ${String(e).slice(0, 80)}`);
              } finally {
                setRequestingTip(false);
                setTimeout(() => setTipRequestStatus(null), 6000);
              }
            }}
            disabled={requestingTip}
            className="mt-2 w-full flex items-center justify-center gap-1.5 px-3 py-2 rounded-md bg-[#485df4]/40 hover:bg-[#485df4]/60 disabled:opacity-50 disabled:cursor-not-allowed text-white text-xs font-bold uppercase tracking-wider transition active:scale-[0.98] flex-shrink-0"
            title="Pedir un tip nuevo manualmente"
          >
            {requestingTip ? (
              <>
                <span className="w-3.5 h-3.5 rounded-full border-2 border-white/30 border-t-white animate-spin" />
                Generando…
              </>
            ) : (
              <>
                <Zap className="w-3.5 h-3.5" /> Pedir tip ahora
              </>
            )}
          </button>
          {tipRequestStatus && (
            <div className="mt-1 text-[10px] text-center text-white/70 italic flex-shrink-0">
              {tipRequestStatus}
            </div>
          )}
        </div>
      )}

      {/* SECCIÓN: PREGUNTAS DEL CLIENTE */}
      {section === 'questions' && (
        <div className="flex-1 rounded-lg border border-white/10 bg-white/4 p-3 overflow-hidden flex flex-col min-h-0">
          {questions.length === 0 ? (
            <div className="flex-1 flex items-center justify-center text-white/55 text-xs italic text-center">
              Aún no se detectan preguntas del interlocutor.
            </div>
          ) : (
            <div className="flex-1 overflow-y-auto custom-scrollbar space-y-1.5">
              {questions
                .slice()
                .reverse()
                .map((q, i) => {
                  const ageSec = Math.max(0, (Date.now() - q.timestamp) / 1000);
                  const ageLabel =
                    ageSec < 60
                      ? `${Math.round(ageSec)}s`
                      : ageSec < 3600
                      ? `${Math.floor(ageSec / 60)}m`
                      : `${Math.floor(ageSec / 3600)}h`;
                  return (
                    <div
                      key={`${q.timestamp}-${i}`}
                      className="rounded-md bg-white/8 border border-white/8 px-2.5 py-2"
                    >
                      <div className="flex items-start gap-2">
                        <HelpCircle className="w-3 h-3 mt-0.5 flex-shrink-0 text-[#1bea9a]" />
                        <div className="flex-1 min-w-0">
                          <div className="text-[12px] text-white/95 leading-snug">{q.text}</div>
                          <div className="text-[9px] text-white/45 mt-0.5">hace {ageLabel}</div>
                        </div>
                      </div>
                    </div>
                  );
                })}
            </div>
          )}
        </div>
      )}
      <style>{`
        @keyframes tipSlideIn {
          from { opacity: 0; transform: translateX(20px); }
          to   { opacity: 1; transform: translateX(0); }
        }
        /* v32.3: pulso suave del chip WPM cuando entra zona "Lento" (>180 wpm).
           No invasivo: solo opacidad varía 0.7→1.0. */
        @keyframes wpmPulse {
          0%, 100% { opacity: 1; }
          50%      { opacity: 0.7; }
        }
      `}</style>
    </div>
  );
}
