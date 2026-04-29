'use client';

import React, { useEffect, useMemo, useState } from 'react';
import {
  Radar,
  RadarChart,
  PolarGrid,
  PolarAngleAxis,
  PolarRadiusAxis,
  ResponsiveContainer,
  BarChart,
  Bar,
  XAxis,
  YAxis,
  Tooltip as ReTooltip,
  Cell,
} from 'recharts';
import {
  Sparkles, AlertTriangle, TrendingUp, MessageSquare, Download, CheckCircle,
  Users, Heart, Clock,
} from 'lucide-react';
import { ComplianceReportButton } from '../Compliance';
import { safeInvoke, quietInvoke } from '@/lib/safeInvoke';
import type { Transcript } from '@/types';

interface EvaluationProps {
  meetingId: string;
  transcripts: Transcript[];
  previousMeetingId?: string;
}

interface PostMeetingEvaluationResult {
  meeting_id: string;
  evaluation: MeetingEvaluation;
  model_used: string;
  prompt_version: string;
  latency_ms: number;
  created_at: string;
}

interface ContextoEval {
  relacion?: string;
  formalidad_observada?: string;
  brecha_formalidad?: string;
  objetivo_real_inferido?: string;
  alineacion_objetivo?: number;
  tipo_comunicacion?: string;
}

interface HablanteStats {
  palabras?: number;
  oraciones?: number;
  resumen?: string;
  claridad?: number;
  persuasion?: number;
  formalidad?: number;
  emociones?: { dominante?: string; valor?: number };
}

interface EmpatiaHablante {
  evaluable?: boolean;
  puntaje?: number;
  nivel?: string;
  tu_resultado?: string;
  reconocimiento_emocional?: number;
  escucha_activa?: number;
  tono_empatico?: number;
}

interface DimensionDetalle {
  puntaje?: number;
  nivel?: string;
  tu_resultado?: string;
  que_mide?: string;
}

interface TimelineSegment {
  tipo?: string;
  pct?: number;
}

interface TimelineMomento {
  nombre?: string;
  minuto?: number;
}

interface MeetingEvaluation {
  identificacion: { nombre_sesion: string; idioma?: string };
  historico: { tendencia_global?: number };
  contexto: ContextoEval;
  meta: { duracion_minutos: number; palabras_totales: number };
  resumen: {
    puntuacion_global: number;
    nivel: string;
    descripcion: string;
    fortaleza: string;
    fortaleza_hint?: string;
    mejorar: string;
    mejorar_hint?: string;
  };
  radiografia: {
    muletillas_total: number;
    muletillas_detalle: Record<string, number>;
    ratio_habla: number;
  };
  insights: Array<{ dato: string; por_que: string; sugerencia: string }>;
  patron: { actual: string; evolucion?: string; senales?: string[]; que_cambiaria: string };
  dimensiones: Record<string, DimensionDetalle>;
  por_hablante?: Record<string, HablanteStats>;
  empatia?: Record<string, EmpatiaHablante>;
  calidad_global: { puntaje: number; nivel: string };
  recomendaciones: Array<{ prioridad: number; titulo: string; texto_mejorado: string }>;
  visualizaciones: {
    gauge: { valor: number; label: string };
    radar_calidad: { labels: string[]; valores: number[] };
    muletillas_chart: { labels: string[]; valores: number[] };
    timeline_chart?: { segmentos?: TimelineSegment[]; momentos?: TimelineMomento[] };
  };
}

function buildTranscriptText(transcripts: Transcript[]): string {
  // Formato canónico que espera el prompt v5: "Speaker: texto del turno".
  // Mapeamos source_type → etiqueta legible para el LLM.
  return transcripts
    .map((t) => {
      const raw = (t.source_type || '').toLowerCase();
      const speaker =
        raw === 'user' || raw === 'mic' || raw === 'microphone'
          ? 'USUARIO'
          : raw === 'interlocutor' || raw === 'system' || raw === 'speaker'
          ? 'INTERLOCUTOR'
          : 'DESCONOCIDO';
      return `${speaker}: ${(t.text ?? '').trim()}`;
    })
    .filter((line) => line.length > line.indexOf(':') + 2)
    .join('\n');
}

function nivelColor(score: number): string {
  if (score >= 76) return '#1bea9a';
  if (score >= 46) return '#f59e0b';
  return '#ff0050';
}

const DIMENSION_LABELS: Record<string, string> = {
  claridad: 'Claridad',
  estructura: 'Estructura',
  persuasion: 'Persuasión',
  proposito: 'Propósito',
  empatia: 'Empatía',
  adaptacion: 'Adaptación',
};

function GaugeWidget({ valor, label }: { valor: number; label: string }) {
  const color = nivelColor(valor);
  const angle = (valor / 100) * 180;
  return (
    <div className="flex flex-col items-center">
      <div className="relative w-48 h-24">
        <svg viewBox="0 0 200 100" className="w-full h-full">
          <path d="M 10 100 A 90 90 0 0 1 190 100" fill="none" stroke="rgba(255,255,255,0.12)" strokeWidth="14" strokeLinecap="round" />
          <path
            d="M 10 100 A 90 90 0 0 1 190 100"
            fill="none"
            stroke={color}
            strokeWidth="14"
            strokeLinecap="round"
            strokeDasharray={`${(angle / 180) * 283} 283`}
          />
          <text x="100" y="85" textAnchor="middle" fontSize="32" fontWeight="700" fill={color}>
            {Math.round(valor)}
          </text>
        </svg>
      </div>
      <div className="text-sm font-medium text-gray-100 capitalize">{label}</div>
    </div>
  );
}

function RadarWidget({ labels, valores }: { labels: string[]; valores: number[] }) {
  const data = labels.map((l, i) => ({ dim: l, valor: valores[i] ?? 0 }));
  return (
    <div className="w-full h-64">
      <ResponsiveContainer>
        <RadarChart data={data} outerRadius="75%">
          <PolarGrid stroke="rgba(255,255,255,0.18)" />
          <PolarAngleAxis dataKey="dim" tick={{ fill: '#e5e7eb', fontSize: 11 }} />
          <PolarRadiusAxis angle={90} domain={[0, 100]} tick={{ fill: '#9ca3af', fontSize: 10 }} />
          <Radar dataKey="valor" stroke="#a8b3ff" fill="#485df4" fillOpacity={0.45} />
        </RadarChart>
      </ResponsiveContainer>
    </div>
  );
}

function MuletillasChart({ labels, valores }: { labels: string[]; valores: number[] }) {
  const data = labels.map((l, i) => ({ name: l, count: valores[i] ?? 0 }));
  if (data.length === 0) {
    return <div className="text-sm text-gray-400">Sin muletillas detectadas.</div>;
  }
  return (
    <div className="w-full h-56">
      <ResponsiveContainer>
        <BarChart data={data} layout="vertical" margin={{ left: 12, right: 12 }}>
          <XAxis type="number" tick={{ fontSize: 11, fill: '#e5e7eb' }} />
          <YAxis dataKey="name" type="category" tick={{ fontSize: 11, fill: '#e5e7eb' }} width={80} />
          <ReTooltip
            contentStyle={{ background: 'rgba(15,16,24,0.95)', border: '1px solid rgba(255,255,255,0.12)', color: '#fff' }}
            cursor={{ fill: 'rgba(72, 93, 244, 0.15)' }}
          />
          <Bar dataKey="count" radius={[0, 4, 4, 0]}>
            {data.map((_, i) => (
              <Cell key={i} fill="#485df4" />
            ))}
          </Bar>
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
}

function TimelineChart({ segmentos, momentos }: { segmentos?: TimelineSegment[]; momentos?: TimelineMomento[] }) {
  const segs = segmentos ?? [];
  const moms = momentos ?? [];
  if (segs.length === 0) return null;
  const palette = ['#485df4', '#1bea9a', '#f59e0b', '#a8b3ff', '#fb7185'];
  return (
    <div className="space-y-2">
      <div className="flex h-4 rounded-md overflow-hidden bg-white/5 border border-white/10">
        {segs.map((s, i) => (
          <div
            key={i}
            title={`${s.tipo ?? '—'}: ${Math.round(s.pct ?? 0)}%`}
            className="flex items-center justify-center text-[10px] font-medium text-white"
            style={{ width: `${Math.max(0, Math.min(100, s.pct ?? 0))}%`, background: palette[i % palette.length] }}
          >
            {(s.pct ?? 0) > 12 ? `${s.tipo} ${Math.round(s.pct ?? 0)}%` : ''}
          </div>
        ))}
      </div>
      {moms.length > 0 && (
        <ul className="text-xs text-gray-300 space-y-1">
          {moms.map((m, i) => (
            <li key={i} className="flex items-center gap-2">
              <Clock className="w-3 h-3 text-gray-400" />
              <span className="font-mono text-gray-400">m{m.minuto ?? 0}</span>
              <span>{m.nombre ?? '—'}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function HablantesCards({ hablantes }: { hablantes: Record<string, HablanteStats> }) {
  const entries = Object.entries(hablantes);
  if (entries.length === 0) return null;
  return (
    <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
      {entries.map(([nombre, stats]) => (
        <div key={nombre} className="rounded-lg bg-white/5 border border-white/10 p-4 space-y-2">
          <div className="flex items-center justify-between">
            <div className="font-semibold text-gray-100">{nombre}</div>
            {stats.emociones?.dominante && (
              <span className="text-[10px] uppercase tracking-wider text-gray-400">
                {stats.emociones.dominante}
              </span>
            )}
          </div>
          {stats.resumen && <div className="text-xs text-gray-300">{stats.resumen}</div>}
          <div className="grid grid-cols-3 gap-2 text-center text-[10px] uppercase tracking-wider text-gray-400">
            <div>
              <div className="text-base font-bold tabular-nums" style={{ color: nivelColor(stats.claridad ?? 0) }}>
                {Math.round(stats.claridad ?? 0)}
              </div>
              <div>Claridad</div>
            </div>
            <div>
              <div className="text-base font-bold tabular-nums" style={{ color: nivelColor(stats.persuasion ?? 0) }}>
                {Math.round(stats.persuasion ?? 0)}
              </div>
              <div>Persuasión</div>
            </div>
            <div>
              <div className="text-base font-bold tabular-nums" style={{ color: nivelColor(stats.formalidad ?? 0) }}>
                {Math.round(stats.formalidad ?? 0)}
              </div>
              <div>Formalidad</div>
            </div>
          </div>
          <div className="text-[10px] text-gray-400">
            {stats.palabras ?? 0} palabras · {stats.oraciones ?? 0} oraciones
          </div>
        </div>
      ))}
    </div>
  );
}

function EmpatiaCards({ empatia }: { empatia: Record<string, EmpatiaHablante> }) {
  const entries = Object.entries(empatia).filter(([, e]) => e.evaluable !== false);
  if (entries.length === 0) return null;
  return (
    <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
      {entries.map(([nombre, e]) => (
        <div key={nombre} className="rounded-lg bg-white/5 border border-white/10 p-4 space-y-2">
          <div className="flex items-center justify-between">
            <div className="font-semibold text-gray-100">{nombre}</div>
            <div
              className="text-2xl font-bold tabular-nums"
              style={{ color: nivelColor(e.puntaje ?? 0) }}
            >
              {Math.round(e.puntaje ?? 0)}
            </div>
          </div>
          {e.tu_resultado && <div className="text-xs text-gray-300">{e.tu_resultado}</div>}
          <div className="space-y-1.5 mt-2">
            {[
              { label: 'Reconocimiento emocional', value: e.reconocimiento_emocional ?? 0 },
              { label: 'Escucha activa', value: e.escucha_activa ?? 0 },
              { label: 'Tono empático', value: e.tono_empatico ?? 0 },
            ].map((row, i) => (
              <div key={i}>
                <div className="flex justify-between text-[10px] text-gray-400 mb-0.5">
                  <span>{row.label}</span>
                  <span className="tabular-nums">{Math.round(row.value)}</span>
                </div>
                <div className="h-1.5 rounded-full bg-white/8 overflow-hidden">
                  <div
                    className="h-full rounded-full transition-all duration-500"
                    style={{ width: `${row.value}%`, background: nivelColor(row.value) }}
                  />
                </div>
              </div>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}

function ContextoBadges({ ctx }: { ctx: ContextoEval }) {
  const items: Array<{ label: string; value: string }> = [];
  if (ctx.tipo_comunicacion) items.push({ label: 'Tipo', value: ctx.tipo_comunicacion });
  if (ctx.relacion) items.push({ label: 'Relación', value: ctx.relacion });
  if (ctx.formalidad_observada) items.push({ label: 'Formalidad', value: ctx.formalidad_observada });
  if (ctx.brecha_formalidad && ctx.brecha_formalidad !== 'ninguna')
    items.push({ label: 'Brecha formal.', value: ctx.brecha_formalidad });
  if (typeof ctx.alineacion_objetivo === 'number')
    items.push({ label: 'Alineación obj.', value: `${Math.round(ctx.alineacion_objetivo * 100)}%` });
  if (items.length === 0) return null;
  return (
    <div className="flex flex-wrap gap-2">
      {items.map((it, i) => (
        <span key={i} className="text-[10px] uppercase tracking-wider px-2.5 py-1 rounded-full bg-white/8 border border-white/10 text-gray-200">
          <span className="text-gray-400 mr-1">{it.label}:</span>
          <span className="font-semibold capitalize">{it.value}</span>
        </span>
      ))}
    </div>
  );
}

function DimensionesGrid({ dimensiones }: { dimensiones: Record<string, DimensionDetalle> }) {
  const order = ['claridad', 'estructura', 'persuasion', 'proposito', 'empatia', 'adaptacion'];
  const items = order
    .map((k) => ({ key: k, label: DIMENSION_LABELS[k] ?? k, det: dimensiones?.[k] }))
    .filter((x) => x.det && typeof x.det.puntaje === 'number');
  if (items.length === 0) return null;
  return (
    <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
      {items.map(({ key, label, det }) => (
        <div key={key} className="rounded-lg bg-white/5 border border-white/10 p-3 space-y-1.5">
          <div className="flex items-center justify-between">
            <span className="text-[10px] uppercase tracking-wider text-gray-400">{label}</span>
            <span
              className="text-lg font-bold tabular-nums"
              style={{ color: nivelColor(det!.puntaje ?? 0) }}
            >
              {Math.round(det!.puntaje ?? 0)}
            </span>
          </div>
          {det!.nivel && (
            <div className="text-[10px] capitalize text-gray-300">{det!.nivel.replace(/_/g, ' ')}</div>
          )}
          {det!.tu_resultado && (
            <div className="text-xs text-gray-200 leading-snug">{det!.tu_resultado}</div>
          )}
        </div>
      ))}
    </div>
  );
}

function SkeletonCard() {
  return (
    <div className="rounded-xl border border-white/10 bg-white/5 p-5 animate-pulse space-y-3">
      <div className="h-3 w-24 rounded bg-white/10" />
      <div className="h-32 rounded bg-white/8" />
      <div className="h-3 w-3/4 rounded bg-white/10" />
    </div>
  );
}

export function EvaluationPanel({ meetingId, transcripts, previousMeetingId }: EvaluationProps) {
  const [result, setResult] = useState<PostMeetingEvaluationResult | null>(null);
  const [loading, setLoading] = useState<boolean>(true);
  const [generating, setGenerating] = useState<boolean>(false);
  const [exporting, setExporting] = useState<boolean>(false);
  const [exportSuccess, setExportSuccess] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    (async () => {
      const existing = await quietInvoke<PostMeetingEvaluationResult | null>('coach_get_post_meeting_evaluation', {
        meetingId,
      });
      if (!cancelled) {
        setResult(existing ?? null);
        setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [meetingId]);

  const transcriptText = useMemo(() => buildTranscriptText(transcripts), [transcripts]);
  const canGenerate = transcriptText.trim().length >= 100;

  const handleGenerate = async () => {
    setGenerating(true);
    setError(null);
    if (!canGenerate) {
      setError('Esta reunión es muy corta para generar evaluación. Necesitas al menos 100 caracteres de transcript.');
      setGenerating(false);
      return;
    }
    try {
      const res = await safeInvoke<PostMeetingEvaluationResult>(
        'coach_evaluate_post_meeting',
        {
          meetingId,
          transcript: transcriptText,
          previousSessionId: previousMeetingId ?? null,
          evaluationModel: null,
        },
        'No se pudo generar la evaluación. Verifica que el modelo de IA esté descargado (ve a Configuración → IA Local).',
      );
      if (res) setResult(res);
      else {
        // Toast ya mostrado por safeInvoke
        if (typeof globalThis !== 'undefined' && globalThis.window) {
          globalThis.window.dispatchEvent(new CustomEvent('verify-ollama-status'));
        }
      }
    } finally {
      setGenerating(false);
    }
  };

  const handleExportPdf = async () => {
    setExporting(true);
    setError(null);
    setExportSuccess(null);
    const pdfPath = await safeInvoke<string>(
      'export_evaluation_pdf',
      { meetingId, outputPath: null },
      'No se pudo exportar el PDF.',
    );
    if (pdfPath) {
      setExportSuccess(pdfPath);
      setTimeout(() => setExportSuccess(null), 5000);
    }
    setExporting(false);
  };

  const handleOpenFolder = async () => {
    if (!exportSuccess) return;
    await safeInvoke('show_in_folder', { path: exportSuccess }, 'No se pudo abrir la carpeta.');
  };

  if (loading) {
    return (
      <div className="p-6 space-y-4">
        <div className="h-6 w-48 rounded bg-white/10 animate-pulse" />
        <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
          <SkeletonCard />
          <SkeletonCard />
        </div>
        <SkeletonCard />
      </div>
    );
  }

  if (!result) {
    return (
      <div className="p-6 flex flex-col items-center text-center gap-4">
        <div className="rounded-full p-4 bg-white/8 border border-white/10">
          <Sparkles className="w-10 h-10 text-[#a8b3ff]" />
        </div>
        <h3 className="text-lg font-semibold text-gray-100">Aún no hay evaluación</h3>
        <p className="text-sm text-gray-300 max-w-md">
          Genera el análisis profundo localmente. Tarda ~30-60s. Todo el procesamiento ocurre en tu equipo
          — el contenido nunca sale de tu computadora.
        </p>
        {!canGenerate && (
          <div className="flex items-center gap-2 text-xs text-[#ff5e85]">
            <AlertTriangle className="w-4 h-4" /> Transcripción muy corta (mínimo 100 caracteres).
          </div>
        )}
        <button
          onClick={handleGenerate}
          disabled={!canGenerate || generating}
          className="px-5 py-2.5 rounded-lg bg-[#485df4] text-white font-medium shadow-sm hover:bg-[#3a4ac3] disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {generating ? 'Analizando…' : 'Generar evaluación'}
        </button>
        {error && <div className="text-xs text-[#ff5e85] max-w-md">{error}</div>}
      </div>
    );
  }

  const ev = result.evaluation;
  const gauge = ev.visualizaciones?.gauge ?? { valor: ev.resumen.puntuacion_global, label: ev.resumen.nivel };
  const radar = ev.visualizaciones?.radar_calidad ?? { labels: [], valores: [] };
  const muletillas = ev.visualizaciones?.muletillas_chart ?? { labels: [], valores: [] };
  const timeline = ev.visualizaciones?.timeline_chart;
  const hablantes = ev.por_hablante ?? {};
  const empatia = ev.empatia ?? {};
  const contexto = ev.contexto ?? {};

  return (
    <div className="p-6 space-y-6 overflow-y-auto custom-scrollbar h-full text-gray-100">
      <div className="flex items-start justify-between gap-4 flex-wrap">
        <div className="min-w-0 flex-1">
          <div className="text-xs uppercase tracking-wider text-gray-400">Evaluación post-meeting</div>
          <h2 className="text-xl font-semibold text-gray-50 mt-0.5">
            {ev.identificacion?.nombre_sesion || 'Reunión'}
          </h2>
          <div className="text-sm text-gray-300 mt-1 max-w-2xl leading-relaxed">{ev.resumen.descripcion}</div>
          <div className="mt-3"><ContextoBadges ctx={contexto} /></div>
        </div>
        <div className="flex gap-2 flex-wrap">
          <ComplianceReportButton meetingId={meetingId} />
          <button
            onClick={handleExportPdf}
            disabled={exporting}
            title="Exportar evaluación como PDF"
            aria-label="Exportar PDF"
            className="px-3 py-1.5 text-xs rounded-md border border-white/15 hover:bg-white/8 text-gray-100 disabled:opacity-50 flex items-center gap-1.5"
          >
            <Download className="w-3.5 h-3.5" />
            {exporting ? 'Exportando…' : 'PDF'}
          </button>
          <button
            onClick={handleGenerate}
            disabled={generating}
            aria-label="Re-evaluar"
            className="px-3 py-1.5 text-xs rounded-md border border-white/15 hover:bg-white/8 text-gray-100 disabled:opacity-50"
          >
            {generating ? 'Re-evaluando…' : 'Re-evaluar'}
          </button>
        </div>
      </div>

      {exportSuccess && (
        <div className="flex items-center gap-2 p-3 bg-emerald-500/10 border border-emerald-500/40 rounded-lg text-sm">
          <CheckCircle className="w-4 h-4 text-emerald-400" />
          <span className="flex-1 text-emerald-200">PDF guardado correctamente</span>
          <button
            onClick={handleOpenFolder}
            className="text-xs text-emerald-300 underline hover:font-semibold"
          >
            Abrir carpeta
          </button>
        </div>
      )}

      {error && (
        <div className="flex items-center gap-2 p-3 bg-rose-500/10 border border-rose-500/40 rounded-lg text-sm">
          <AlertTriangle className="w-4 h-4 text-rose-400" />
          <span className="flex-1 text-rose-200">{error}</span>
        </div>
      )}

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        <div className="bg-white/5 rounded-xl border border-white/10 p-5 flex flex-col items-center">
          <div className="text-xs uppercase tracking-wider text-gray-400 mb-2">Puntuación global</div>
          <GaugeWidget valor={gauge.valor} label={gauge.label} />
          {typeof ev.historico?.tendencia_global === 'number' && (
            <div className="mt-3 flex items-center gap-1.5 text-xs">
              <TrendingUp className="w-4 h-4" />
              <span className={ev.historico.tendencia_global >= 0 ? 'text-emerald-400' : 'text-rose-400'}>
                {ev.historico.tendencia_global >= 0 ? '+' : ''}{ev.historico.tendencia_global.toFixed(1)} vs sesión anterior
              </span>
            </div>
          )}
        </div>

        <div className="bg-white/5 rounded-xl border border-white/10 p-5">
          <div className="text-xs uppercase tracking-wider text-gray-400 mb-2">Radar 6 dimensiones</div>
          {radar.labels.length > 0 ? (
            <RadarWidget labels={radar.labels} valores={radar.valores} />
          ) : (
            <div className="text-sm text-gray-400">Sin radar disponible.</div>
          )}
        </div>
      </div>

      <DimensionesGrid dimensiones={ev.dimensiones ?? {}} />

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        <div className="bg-white/5 rounded-xl border border-white/10 p-5">
          <div className="text-xs uppercase tracking-wider text-gray-400 mb-2">Muletillas detectadas</div>
          <MuletillasChart labels={muletillas.labels} valores={muletillas.valores} />
        </div>

        <div className="bg-white/5 rounded-xl border border-white/10 p-5">
          <div className="text-xs uppercase tracking-wider text-gray-400 mb-2">Resumen ejecutivo</div>
          <div className="space-y-3 text-sm text-gray-100">
            <div>
              <span className="font-semibold text-emerald-400">Fortaleza:</span>{' '}
              <span className="capitalize">{ev.resumen.fortaleza || '—'}</span>
              {ev.resumen.fortaleza_hint && (
                <div className="text-xs text-gray-300 mt-0.5">{ev.resumen.fortaleza_hint}</div>
              )}
            </div>
            <div>
              <span className="font-semibold text-rose-400">A mejorar:</span>{' '}
              <span className="capitalize">{ev.resumen.mejorar || '—'}</span>
              {ev.resumen.mejorar_hint && (
                <div className="text-xs text-gray-300 mt-0.5">{ev.resumen.mejorar_hint}</div>
              )}
            </div>
            <div className="pt-2 border-t border-white/10">
              <span className="font-semibold text-gray-100">Patrón actual:</span>{' '}
              <span className="text-gray-200">{ev.patron?.actual || '—'}</span>
            </div>
            {ev.patron?.evolucion && (
              <div>
                <span className="font-semibold text-gray-100">Evolución sugerida:</span>{' '}
                <span className="text-gray-200">{ev.patron.evolucion}</span>
              </div>
            )}
            <div>
              <span className="font-semibold text-gray-100">Qué cambiar:</span>{' '}
              <span className="text-gray-200">{ev.patron?.que_cambiaria || '—'}</span>
            </div>
          </div>
        </div>
      </div>

      {timeline?.segmentos && timeline.segmentos.length > 0 && (
        <div className="bg-white/5 rounded-xl border border-white/10 p-5">
          <div className="text-xs uppercase tracking-wider text-gray-400 mb-3 flex items-center gap-2">
            <Clock className="w-4 h-4" /> Línea de tiempo
          </div>
          <TimelineChart segmentos={timeline.segmentos} momentos={timeline.momentos} />
        </div>
      )}

      {Object.keys(hablantes).length > 0 && (
        <div className="bg-white/5 rounded-xl border border-white/10 p-5">
          <div className="text-xs uppercase tracking-wider text-gray-400 mb-3 flex items-center gap-2">
            <Users className="w-4 h-4" /> Por hablante
          </div>
          <HablantesCards hablantes={hablantes} />
        </div>
      )}

      {Object.keys(empatia).length > 0 && (
        <div className="bg-white/5 rounded-xl border border-white/10 p-5">
          <div className="text-xs uppercase tracking-wider text-gray-400 mb-3 flex items-center gap-2">
            <Heart className="w-4 h-4" /> Empatía
          </div>
          <EmpatiaCards empatia={empatia} />
        </div>
      )}

      {ev.insights && ev.insights.length > 0 && (
        <div className="bg-white/5 rounded-xl border border-white/10 p-5">
          <div className="text-xs uppercase tracking-wider text-gray-400 mb-3 flex items-center gap-2">
            <MessageSquare className="w-4 h-4" /> Insights
          </div>
          <div className="space-y-3">
            {ev.insights.map((ins, i) => (
              <div key={i} className="border-l-2 border-[#485df4] pl-3 text-sm">
                <div className="font-medium text-gray-100">{ins.dato}</div>
                <div className="text-xs text-gray-400 mt-0.5">{ins.por_que}</div>
                <div className="text-xs text-blue-300 mt-1">→ {ins.sugerencia}</div>
              </div>
            ))}
          </div>
        </div>
      )}

      {ev.recomendaciones && ev.recomendaciones.length > 0 && (
        <div className="bg-white/5 rounded-xl border border-white/10 p-5">
          <div className="text-xs uppercase tracking-wider text-gray-400 mb-3">Recomendaciones priorizadas</div>
          <div className="space-y-3">
            {ev.recomendaciones
              .slice()
              .sort((a, b) => a.prioridad - b.prioridad)
              .map((r, i) => (
                <div key={i} className="rounded-lg bg-white/8 border border-white/10 p-3">
                  <div className="text-[10px] font-bold tracking-wider text-blue-300">PRIORIDAD {r.prioridad}</div>
                  <div className="font-semibold text-gray-100 mt-1">{r.titulo}</div>
                  <div className="text-sm text-gray-200 mt-1 leading-relaxed">{r.texto_mejorado}</div>
                </div>
              ))}
          </div>
        </div>
      )}

      <div className="text-xs text-gray-400 flex items-center justify-between pt-2 border-t border-white/10">
        <span>Modelo local · prompt {result.prompt_version}</span>
        <span>Latencia: {(result.latency_ms / 1000).toFixed(1)}s</span>
      </div>
    </div>
  );
}

export default EvaluationPanel;
