'use client';

import React, { useEffect, useState, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import {
  Mic, Volume2, X, Minimize2, Maximize2, Sparkles, AlertTriangle,
  TrendingUp, MessageCircle, Clock, Activity, Timer,
} from 'lucide-react';

interface CoachTip {
  tip: string;
  category?: string;
  priority?: 'critical' | 'important' | 'soft' | 'high' | 'medium' | 'low';
  technique?: string;
  confidence?: number;
  tip_type?: string;
  timestamp?: number;
}

interface AudioLevels {
  mic_rms?: number;
  sys_rms?: number;
  micRms?: number;
  sysRms?: number;
  mic_peak?: number;
  sys_peak?: number;
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
  connectionScore?: number;
  connectionTrend?: 'up' | 'down' | 'flat' | 'stable' | 'rising' | 'falling';
}

function formatDuration(sec: number): string {
  const m = Math.floor(sec / 60);
  const s = Math.floor(sec % 60);
  return `${m}:${s.toString().padStart(2, '0')}`;
}

function priorityHex(p?: string): string {
  if (p === 'critical' || p === 'high') return '#ff0050';
  if (p === 'important' || p === 'medium') return '#f59e0b';
  return '#1bea9a';
}

function priorityLabel(p?: string): string {
  if (p === 'critical' || p === 'high') return 'Crítico';
  if (p === 'important' || p === 'medium') return 'Importante';
  return 'Sugerencia';
}

function healthColor(score: number): string {
  if (score >= 70) return '#1bea9a';
  if (score >= 40) return '#f59e0b';
  return '#ff0050';
}

/** Gauge circular grande para salud de la conversación. */
function HealthGauge({ score }: { score: number }) {
  const radius = 38;
  const stroke = 8;
  const circumference = 2 * Math.PI * radius;
  const offset = circumference - (score / 100) * circumference;
  const color = healthColor(score);
  return (
    <div className="relative w-24 h-24 flex-shrink-0">
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
      <div className="absolute inset-0 flex flex-col items-center justify-center">
        <div className="text-2xl font-bold tabular-nums" style={{ color }}>
          {Math.round(score)}
        </div>
        <div className="text-[9px] uppercase tracking-wider text-white/60 -mt-0.5">Salud</div>
      </div>
    </div>
  );
}

/** Barra de audio con label e icono. */
function AudioBar({
  label,
  icon,
  level,
  color,
}: {
  label: string;
  icon: React.ReactNode;
  level: number;
  color: string;
}) {
  const pct = Math.min(100, Math.max(0, level * 200));
  const active = level > 0.005;
  return (
    <div className="flex items-center gap-1.5">
      <div className={`flex items-center gap-1 text-[10px] uppercase tracking-wide ${active ? 'text-white/90' : 'text-white/40'}`}>
        <span className={active ? 'animate-pulse' : ''}>{icon}</span>
        <span className="font-semibold w-7">{label}</span>
      </div>
      <div className="flex-1 h-2 rounded-full bg-white/10 overflow-hidden relative">
        <div
          className="absolute inset-y-0 left-0 rounded-full transition-all duration-100"
          style={{ width: `${pct}%`, background: color }}
        />
      </div>
      <div className="text-[9px] tabular-nums text-white/60 w-7 text-right">
        {Math.round(pct)}%
      </div>
    </div>
  );
}

/** Tarjeta de métrica individual con label + valor grande. */
function MetricCard({ label, value, color, icon }: { label: string; value: string; color: string; icon: React.ReactNode }) {
  return (
    <div className="flex flex-col rounded-lg bg-white/5 border border-white/8 p-2 min-w-0">
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
  const [tip, setTip] = useState<CoachTip | null>(null);
  const [tipsCount, setTipsCount] = useState(0);
  const [audio, setAudio] = useState<AudioLevels>({});
  const [metrics, setMetrics] = useState<MeetingMetrics>({});
  const tipFlashRef = useRef(false);

  useEffect(() => {
    const unlisteners: Array<() => void> = [];

    // Único canal de tips: `coach-tip-update` emitido por coach_suggest backend.
    // Antes había un duplicado (`coach-suggestion`) que causaba contadores inflados.
    listen<CoachTip>('coach-tip-update', (e) => {
      setTip(e.payload);
      setTipsCount((c) => c + 1);
      tipFlashRef.current = true;
      setTimeout(() => { tipFlashRef.current = false; }, 600);
    }).then(u => unlisteners.push(u));

    listen<AudioLevels>('audio-levels', (e) => setAudio(e.payload)).then(u => unlisteners.push(u));
    listen<AudioLevels>('recording-audio-levels', (e) => setAudio(e.payload)).then(u => unlisteners.push(u));
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

  const micLevel = audio.mic_rms ?? audio.micRms ?? 0;
  const sysLevel = audio.sys_rms ?? audio.sysRms ?? 0;
  const health = metrics.health ?? metrics.connectionScore ?? 50;
  const wpm = metrics.wpm ?? 0;
  const duration = metrics.durationSec ?? 0;
  const tipColor = priorityHex(tip?.priority);

  if (compact) {
    return (
      <div
        className="h-screen w-screen flex flex-col p-2 select-none cursor-move"
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
              {tip?.priority ? priorityLabel(tip.priority) : 'Esperando'}
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
      {/* HEADER */}
      <div className="flex items-center justify-between mb-3 flex-shrink-0" data-tauri-drag-region>
        <div className="flex items-center gap-1.5 text-xs" data-tauri-drag-region>
          <div className="flex items-center gap-1.5">
            <div className="relative w-2 h-2">
              <div className="absolute inset-0 rounded-full bg-[#1bea9a] animate-ping opacity-75"/>
              <div className="absolute inset-0 rounded-full bg-[#1bea9a]"/>
            </div>
            <span className="font-bold tracking-wider text-white">MAITY COACH</span>
          </div>
          {tipsCount > 0 && (
            <span className="ml-1 text-[9px] px-1.5 py-0.5 rounded bg-[#485df4]/30 text-[#a8b3ff] font-semibold">
              {tipsCount} tip{tipsCount !== 1 ? 's' : ''}
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

      {/* GAUGE + MÉTRICAS */}
      <div className="flex items-center gap-3 mb-3 flex-shrink-0" data-tauri-drag-region>
        <HealthGauge score={health} />
        <div className="flex-1 grid grid-cols-2 gap-1.5 min-w-0">
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

      {/* AUDIO BARS */}
      <div className="space-y-1.5 mb-3 flex-shrink-0 px-1" data-tauri-drag-region>
        <AudioBar label="MIC" icon={<Mic className="w-3 h-3" />} level={micLevel} color="#485df4" />
        <AudioBar label="SIS" icon={<Volume2 className="w-3 h-3" />} level={sysLevel} color="#1bea9a" />
      </div>

      {/* TALK TIME SPLIT */}
      <div className="mb-3 flex-shrink-0" data-tauri-drag-region>
        <div className="flex items-center justify-between text-[9px] uppercase tracking-wider text-white/55 mb-1">
          <span>Tiempo de palabra</span>
          <span className="text-white/70 tabular-nums">
            {formatDuration(metrics.userTalkSec ?? 0)} · {formatDuration(metrics.interlocutorTalkSec ?? 0)}
          </span>
        </div>
        <div className="flex h-3 rounded-full overflow-hidden bg-white/8">
          <div
            className="flex items-center justify-center text-[9px] font-bold text-white transition-all duration-300"
            style={{
              width: `${Math.max(0, Math.min(100, metrics.userTalkPct ?? 0))}%`,
              background: '#485df4',
              minWidth: (metrics.userTalkPct ?? 0) > 8 ? undefined : 0,
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

      {/* TIP CARD */}
      <div
        className="flex-1 rounded-lg border p-3 overflow-hidden flex flex-col min-h-0"
        style={{
          background: tip ? `linear-gradient(135deg, ${tipColor}1a 0%, rgba(255,255,255,0.04) 100%)` : 'rgba(255,255,255,0.04)',
          borderColor: tip ? `${tipColor}55` : 'rgba(255,255,255,0.08)',
          transition: 'border-color 0.3s ease, background 0.3s ease',
        }}
      >
        <div className="flex items-center justify-between mb-2 flex-shrink-0">
          <div className="flex items-center gap-1.5">
            {tip ? (
              <>
                {(tip.priority === 'critical' || tip.priority === 'high') ? (
                  <AlertTriangle className="w-3.5 h-3.5" style={{ color: tipColor }} />
                ) : (
                  <Sparkles className="w-3.5 h-3.5" style={{ color: tipColor }} />
                )}
                <span className="text-[10px] font-bold uppercase tracking-wider" style={{ color: tipColor }}>
                  {priorityLabel(tip.priority)}
                </span>
              </>
            ) : (
              <>
                <MessageCircle className="w-3.5 h-3.5 text-white/50" />
                <span className="text-[10px] font-bold uppercase tracking-wider text-white/50">
                  Tip en vivo
                </span>
              </>
            )}
          </div>
          {tip?.category && (
            <span className="text-[9px] px-1.5 py-0.5 rounded bg-white/10 text-white/70 font-semibold">
              {tip.category}
            </span>
          )}
        </div>

        <div className="text-[13px] text-white/95 leading-relaxed overflow-y-auto custom-scrollbar flex-1">
          {tip?.tip || (
            <div className="text-white/55 text-xs italic">
              Esperando próxima sugerencia del coach. Habla con tu interlocutor — los tips llegan
              cada ~20s o cuando hay cambios relevantes.
            </div>
          )}
        </div>

        {tip?.technique && (
          <div className="mt-2 pt-2 border-t border-white/10 flex-shrink-0">
            <div className="text-[9px] uppercase tracking-wider text-white/50 mb-0.5">Técnica</div>
            <div className="text-[11px] text-white/80 italic">{tip.technique}</div>
          </div>
        )}
      </div>
    </div>
  );
}
