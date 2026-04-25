'use client';

import React, { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { Mic, Volume2, X, Minimize2, Maximize2, Sparkles } from 'lucide-react';

interface CoachTip {
  text: string;
  category?: string;
  priority?: 'high' | 'medium' | 'low';
}

interface AudioLevels {
  micRms: number;
  sysRms: number;
}

interface MeetingMetrics {
  health?: number;
  wpm?: number;
  durationSec?: number;
}

function formatDuration(sec: number): string {
  const m = Math.floor(sec / 60);
  const s = Math.floor(sec % 60);
  return `${m}:${s.toString().padStart(2, '0')}`;
}

function priorityColor(p?: string): string {
  if (p === 'high') return '#ff0050';
  if (p === 'medium') return '#f59e0b';
  return '#1bea9a';
}

export default function FloatingPage() {
  const [compact, setCompact] = useState(false);
  const [tip, setTip] = useState<CoachTip | null>(null);
  const [audio, setAudio] = useState<AudioLevels>({ micRms: 0, sysRms: 0 });
  const [metrics, setMetrics] = useState<MeetingMetrics>({});

  useEffect(() => {
    const unlisteners: Array<() => void> = [];

    listen<CoachTip>('coach-tip-update', (e) => setTip(e.payload)).then(u => unlisteners.push(u));
    listen<{ tip: string; category?: string; priority?: 'high' | 'medium' | 'low' }>('coach-suggestion', (e) => {
      setTip({ text: e.payload.tip, category: e.payload.category, priority: e.payload.priority });
    }).then(u => unlisteners.push(u));
    listen<AudioLevels>('audio-levels', (e) => setAudio(e.payload)).then(u => unlisteners.push(u));
    listen<MeetingMetrics>('meeting-metrics', (e) => setMetrics(e.payload)).then(u => unlisteners.push(u));

    return () => unlisteners.forEach(u => u());
  }, []);

  const handleClose = async () => {
    try {
      await invoke('close_floating_coach');
    } catch (e) {
      console.error(e);
    }
  };

  const handleToggleCompact = async () => {
    const next = !compact;
    setCompact(next);
    try {
      await invoke('floating_toggle_compact', { compact: next });
    } catch (e) {
      console.error(e);
    }
  };

  const health = metrics.health ?? 0;
  const healthColor = health >= 70 ? '#1bea9a' : health >= 40 ? '#f59e0b' : '#ff0050';

  if (compact) {
    return (
      <div
        className="h-screen w-screen flex flex-col p-2 select-none"
        style={{
          background: 'rgba(20, 20, 28, 0.78)',
          backdropFilter: 'blur(16px)',
          WebkitBackdropFilter: 'blur(16px)',
          border: '1px solid rgba(255,255,255,0.08)',
          borderRadius: 12,
        }}
        data-tauri-drag-region
      >
        <div className="flex items-center justify-between mb-1" data-tauri-drag-region>
          <div className="flex items-center gap-1.5 text-[10px] uppercase tracking-wider text-white/60" data-tauri-drag-region>
            <Sparkles className="w-3 h-3" /> Maity
          </div>
          <div className="flex items-center gap-0.5">
            <button onClick={handleToggleCompact} className="p-0.5 hover:bg-white/10 rounded text-white/70">
              <Maximize2 className="w-3 h-3" />
            </button>
            <button onClick={handleClose} className="p-0.5 hover:bg-white/10 rounded text-white/70">
              <X className="w-3 h-3" />
            </button>
          </div>
        </div>
        <div className="flex-1 flex items-center gap-2" data-tauri-drag-region>
          <div className="text-2xl font-bold leading-none" style={{ color: healthColor }}>
            {Math.round(health)}
          </div>
          <div className="text-[10px] text-white/80 line-clamp-2 leading-tight">
            {tip?.text || 'Esperando…'}
          </div>
        </div>
      </div>
    );
  }

  return (
    <div
      className="h-screen w-screen flex flex-col p-3 select-none text-white"
      style={{
        background: 'rgba(20, 20, 28, 0.82)',
        backdropFilter: 'blur(20px)',
        WebkitBackdropFilter: 'blur(20px)',
        border: '1px solid rgba(255,255,255,0.1)',
        borderRadius: 14,
      }}
      data-tauri-drag-region
    >
      <div className="flex items-center justify-between mb-3" data-tauri-drag-region>
        <div className="flex items-center gap-2 text-xs uppercase tracking-wider text-white/70" data-tauri-drag-region>
          <Sparkles className="w-3.5 h-3.5 text-[#485df4]" /> Coach Maity
        </div>
        <div className="flex items-center gap-1">
          <button
            onClick={handleToggleCompact}
            className="p-1 hover:bg-white/10 rounded text-white/70"
            title="Modo compacto"
          >
            <Minimize2 className="w-3.5 h-3.5" />
          </button>
          <button
            onClick={handleClose}
            className="p-1 hover:bg-white/10 rounded text-white/70"
            title="Cerrar"
          >
            <X className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      <div className="grid grid-cols-3 gap-2 mb-3" data-tauri-drag-region>
        <div className="rounded-lg bg-white/5 border border-white/5 p-2">
          <div className="text-[9px] uppercase text-white/50">Salud</div>
          <div className="text-lg font-bold mt-0.5" style={{ color: healthColor }}>
            {Math.round(health)}
          </div>
        </div>
        <div className="rounded-lg bg-white/5 border border-white/5 p-2">
          <div className="text-[9px] uppercase text-white/50">WPM</div>
          <div className="text-lg font-bold mt-0.5 text-white">{Math.round(metrics.wpm ?? 0)}</div>
        </div>
        <div className="rounded-lg bg-white/5 border border-white/5 p-2">
          <div className="text-[9px] uppercase text-white/50">Tiempo</div>
          <div className="text-lg font-bold mt-0.5 text-white">{formatDuration(metrics.durationSec ?? 0)}</div>
        </div>
      </div>

      <div className="flex-1 rounded-lg bg-white/5 border border-white/5 p-3 overflow-hidden flex flex-col">
        <div className="flex items-center gap-1.5 text-[10px] uppercase text-white/50 mb-1.5">
          <span
            className="inline-block w-2 h-2 rounded-full"
            style={{ background: priorityColor(tip?.priority) }}
          />
          Tip en vivo
          {tip?.category && <span className="ml-auto text-white/40">{tip.category}</span>}
        </div>
        <div className="text-[13px] text-white/95 leading-snug overflow-y-auto custom-scrollbar">
          {tip?.text || 'Esperando próxima sugerencia del coach…'}
        </div>
      </div>

      <div className="mt-2 flex items-center gap-2 text-[10px] text-white/60" data-tauri-drag-region>
        <div className="flex items-center gap-1 flex-1">
          <Mic className="w-3 h-3" />
          <div className="flex-1 h-1 rounded bg-white/10 overflow-hidden">
            <div
              className="h-full bg-[#485df4] transition-all duration-150"
              style={{ width: `${Math.min(100, audio.micRms * 200)}%` }}
            />
          </div>
        </div>
        <div className="flex items-center gap-1 flex-1">
          <Volume2 className="w-3 h-3" />
          <div className="flex-1 h-1 rounded bg-white/10 overflow-hidden">
            <div
              className="h-full bg-[#1bea9a] transition-all duration-150"
              style={{ width: `${Math.min(100, audio.sysRms * 200)}%` }}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
