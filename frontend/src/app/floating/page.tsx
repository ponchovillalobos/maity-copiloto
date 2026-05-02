'use client';

import React, { useEffect, useState } from 'react';
import { listen, emit } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import {
  X, Minimize2, Maximize2, Sparkles, AlertTriangle,
  MessageCircle, Activity, Timer, ChevronLeft, ChevronRight, HelpCircle, Zap,
} from 'lucide-react';
import { categoryMeta, priorityMeta } from '@/components/Coach/tipMeta';

interface CoachTip {
  tip: string;
  category?: string;
  priority?: 'critical' | 'important' | 'soft' | 'high' | 'medium' | 'low';
  confidence?: number;
  tip_type?: string;
  timestamp?: number;
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

function HealthGauge({ score }: { score: number }) {
  const radius = 38;
  const stroke = 8;
  const circumference = 2 * Math.PI * radius;
  const offset = circumference - (score / 100) * circumference;
  const color = healthColor(score);
  return (
    <div className="relative w-24 h-24 flex-shrink-0" data-tauri-drag-region>
      <svg className="w-full h-full -rotate-90" viewBox="0 0 100 100">
        <circle cx="50" cy="50" r={radius} stroke="rgba(255,255,255,0.12)" strokeWidth={stroke} fill="none" />
        <circle
          cx="50"
          cy="50"
          r={radius}
          stroke={color}
          strokeWidth={stroke}
          fill="none"
          strokeLinecap="round"
          strokeDasharray={circumference}
          strokeDashoffset={offset}
          style={{ transition: 'stroke-dashoffset 0.5s ease' }}
        />
      </svg>
      <div className="absolute inset-0 flex flex-col items-center justify-center pointer-events-none">
        <div className="text-2xl font-bold tabular-nums" style={{ color }}>
          {Math.round(score)}
        </div>
        <div className="text-[9px] uppercase tracking-wider text-white/60 -mt-0.5">Salud</div>
      </div>
    </div>
  );
}

function MetricCard({ label, value, color, icon }: { label: string; value: string; color: string; icon: React.ReactNode }) {
  return (
    <div
      className="flex flex-col rounded-lg bg-white/5 border border-white/8 p-2 min-w-0"
      data-tauri-drag-region
    >
      <div className="flex items-center gap-1 text-[9px] uppercase tracking-wider text-white/55">
        {icon}
        <span>{label}</span>
      </div>
      <div className="text-base font-bold mt-0.5 tabular-nums" style={{ color }}>
        {value}
      </div>
    </div>
  );
}

export default function FloatingPage() {
  const [compact, setCompact] = useState(false);
  const [tipsHistory, setTipsHistory] = useState<CoachTip[]>([]);
  const [tipIndex, setTipIndex] = useState(0); // 0 = most recent
  const [metrics, setMetrics] = useState<MeetingMetrics>({});
  const [section, setSection] = useState<Section>('tip');
  const [requestingTip, setRequestingTip] = useState(false);

  useEffect(() => {
    const unlisteners: Array<() => void> = [];

    listen<CoachTip>('coach-tip-update', (e) => {
      setTipsHistory((prev) => [e.payload, ...prev].slice(0, 50));
      setTipIndex(0);
      setRequestingTip(false); // Tip llegó — quita el spinner.
    }).then(u => unlisteners.push(u));

    // Limpia tips cuando la sesión termina (grabación stop / nueva reunión).
    listen('coach-tips-clear', () => {
      setTipsHistory([]);
      setTipIndex(0);
    }).then(u => unlisteners.push(u));

    listen<MeetingMetrics>('meeting-metrics', (e) => setMetrics(e.payload)).then(u => unlisteners.push(u));

    return () => unlisteners.forEach(u => u());
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
        background: 'rgba(15, 16, 24, 0.92)',
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

      <div className="flex items-center gap-3 mb-3 flex-shrink-0" data-tauri-drag-region>
        <HealthGauge score={health} />
        <div className="flex-1 grid grid-cols-2 gap-1.5 min-w-0" data-tauri-drag-region>
          <MetricCard
            label="WPM"
            value={Math.round(wpm).toString()}
            color={wpm > 180 ? '#f59e0b' : wpm > 0 ? '#a8b3ff' : 'rgba(255,255,255,0.4)'}
            icon={<Activity className="w-2.5 h-2.5" />}
          />
          <MetricCard
            label="Tiempo"
            value={formatDuration(duration)}
            color="#a8b3ff"
            icon={<Timer className="w-2.5 h-2.5" />}
          />
        </div>
      </div>

<div className="mb-3 flex-shrink-0" data-tauri-drag-region>
        <div className="flex items-center justify-between text-[9px] uppercase tracking-wider text-white/55 mb-1" data-tauri-drag-region>
          <span>Tiempo de palabra</span>
          <span className="text-white/70 tabular-nums">
            {formatDuration(metrics.userTalkSec ?? 0)} · {formatDuration(metrics.interlocutorTalkSec ?? 0)}
          </span>
        </div>
        <div className="flex h-3 rounded-full overflow-hidden bg-white/8" data-tauri-drag-region>
          <div
            className="flex items-center justify-center text-[9px] font-bold text-white transition-all duration-300"
            style={{
              width: `${Math.max(0, Math.min(100, metrics.userTalkPct ?? 0))}%`,
              background: '#485df4',
            }}
          >
            {(metrics.userTalkPct ?? 0) > 18 ? `Tú ${Math.round(metrics.userTalkPct ?? 0)}%` : ''}
          </div>
          <div
            className="flex items-center justify-center text-[9px] font-bold text-white transition-all duration-300"
            style={{
              width: `${Math.max(0, Math.min(100, 100 - (metrics.userTalkPct ?? 0)))}%`,
              background: '#1bea9a',
            }}
          >
            {100 - (metrics.userTalkPct ?? 0) > 18
              ? `Otro ${Math.round(100 - (metrics.userTalkPct ?? 0))}%`
              : ''}
          </div>
        </div>
      </div>

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
            {tipsHistory.length === 0 ? (
              <div className="text-white/55 text-xs italic text-center py-6">
                {requestingTip
                  ? 'Generando primer tip… puede tardar 4-10 segundos.'
                  : 'Sin tips aún. Empezá grabación o presioná "Pedir tip ahora".'}
              </div>
            ) : (
              tipsHistory.map((t, idx) => {
                const tprio = priorityMeta(t.priority);
                const tcat = categoryMeta(t.category);
                const tcolor = tprio.hex;
                const isLatest = idx === 0;
                return (
                  <div
                    key={`${t.tip}-${idx}`}
                    className="rounded-md border p-2"
                    style={{
                      background: `linear-gradient(135deg, ${tcolor}1a 0%, rgba(255,255,255,0.04) 100%)`,
                      borderColor: `${tcolor}55`,
                      borderWidth: isLatest ? '2px' : '1px',
                    }}
                  >
                    <div className="flex items-center gap-1.5 mb-1">
                      {tprio.label === 'Crítico'
                        ? <AlertTriangle className="w-3 h-3" style={{ color: tcolor }} />
                        : <Sparkles className="w-3 h-3" style={{ color: tcolor }} />}
                      <span className="text-[9px] font-bold uppercase tracking-wider" style={{ color: tcolor }}>
                        {tprio.label}
                      </span>
                      {t.category && (
                        <span className="text-[8px] px-1.5 py-0.5 rounded bg-white/10 text-white/60 font-semibold">
                          {tcat.label}
                        </span>
                      )}
                      {isLatest && (
                        <span className="text-[8px] px-1.5 py-0.5 rounded bg-emerald-500/20 text-emerald-300 font-semibold ml-auto">
                          NUEVO
                        </span>
                      )}
                      <span className={`text-[8px] text-white/40 ${isLatest ? '' : 'ml-auto'}`}>
                        #{tipsHistory.length - idx}
                      </span>
                    </div>
                    <div className="text-[12px] text-white/95 leading-snug font-medium">
                      {t.tip}
                    </div>
                  </div>
                );
              })
            )}
          </div>
          {/* Botón "Pedir tip ahora" — con estado generando. */}
          <button
            onClick={() => {
              setRequestingTip(true);
              void emit('coach-request-tip', { source: 'floating' });
              // Auto-reset estado tras 30s o cuando llegue tip nuevo (manejado en useEffect).
              setTimeout(() => setRequestingTip(false), 30000);
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
    </div>
  );
}
