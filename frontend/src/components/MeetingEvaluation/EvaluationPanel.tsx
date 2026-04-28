'use client';

import React, { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
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
import { Sparkles, AlertTriangle, TrendingUp, MessageSquare, Download, CheckCircle } from 'lucide-react';
import { ComplianceReportButton } from '../Compliance';
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

interface MeetingEvaluation {
  identificacion: { nombre_sesion: string; idioma?: string };
  historico: { tendencia_global?: number };
  contexto: { relacion: string; objetivo_real_inferido: string; alineacion_objetivo: number };
  meta: { duracion_minutos: number; palabras_totales: number };
  resumen: { puntuacion_global: number; nivel: string; descripcion: string; fortaleza: string; mejorar: string };
  radiografia: {
    muletillas_total: number;
    muletillas_detalle: Record<string, number>;
    ratio_habla: number;
  };
  insights: Array<{ dato: string; por_que: string; sugerencia: string }>;
  patron: { actual: string; que_cambiaria: string };
  dimensiones: Record<string, { puntaje: number; nivel: string; tu_resultado: string } | { puntaje?: number }>;
  calidad_global: { puntaje: number; nivel: string };
  recomendaciones: Array<{ prioridad: number; titulo: string; texto_mejorado: string }>;
  visualizaciones: {
    gauge: { valor: number; label: string };
    radar_calidad: { labels: string[]; valores: number[] };
    muletillas_chart: { labels: string[]; valores: number[] };
  };
}

function buildTranscriptText(transcripts: Transcript[]): string {
  return transcripts
    .map(t => {
      const speaker = t.source_type || 'desconocido';
      return `[${speaker}] ${t.text}`;
    })
    .join('\n');
}

function nivelColor(score: number): string {
  if (score >= 76) return '#1bea9a';
  if (score >= 46) return '#f59e0b';
  return '#ff0050';
}

function GaugeWidget({ valor, label }: { valor: number; label: string }) {
  const color = nivelColor(valor);
  const angle = (valor / 100) * 180;
  return (
    <div className="flex flex-col items-center">
      <div className="relative w-48 h-24">
        <svg viewBox="0 0 200 100" className="w-full h-full">
          <path d="M 10 100 A 90 90 0 0 1 190 100" fill="none" stroke="#e7e7e9" strokeWidth="14" strokeLinecap="round" />
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
      <div className="text-sm font-medium text-[#3a3a3c] dark:text-gray-200 capitalize">{label}</div>
    </div>
  );
}

function RadarWidget({ labels, valores }: { labels: string[]; valores: number[] }) {
  const data = labels.map((l, i) => ({ dim: l, valor: valores[i] ?? 0 }));
  return (
    <div className="w-full h-64">
      <ResponsiveContainer>
        <RadarChart data={data} outerRadius="75%">
          <PolarGrid stroke="#d0d0d3" />
          <PolarAngleAxis dataKey="dim" tick={{ fill: '#4a4a4c', fontSize: 11 }} />
          <PolarRadiusAxis angle={90} domain={[0, 100]} tick={{ fill: '#8a8a8d', fontSize: 10 }} />
          <Radar dataKey="valor" stroke="#485df4" fill="#485df4" fillOpacity={0.35} />
        </RadarChart>
      </ResponsiveContainer>
    </div>
  );
}

function MuletillasChart({ labels, valores }: { labels: string[]; valores: number[] }) {
  const data = labels.map((l, i) => ({ name: l, count: valores[i] ?? 0 }));
  if (data.length === 0) {
    return <div className="text-sm text-[#6a6a6d]">Sin muletillas detectadas.</div>;
  }
  return (
    <div className="w-full h-56">
      <ResponsiveContainer>
        <BarChart data={data} layout="vertical" margin={{ left: 12, right: 12 }}>
          <XAxis type="number" tick={{ fontSize: 11, fill: '#4a4a4c' }} />
          <YAxis dataKey="name" type="category" tick={{ fontSize: 11, fill: '#4a4a4c' }} width={80} />
          <ReTooltip />
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
      try {
        const existing = await invoke<PostMeetingEvaluationResult | null>('coach_get_post_meeting_evaluation', {
          meetingId,
        });
        if (!cancelled) {
          setResult(existing ?? null);
          setLoading(false);
        }
      } catch (e) {
        if (!cancelled) {
          setError(String(e));
          setLoading(false);
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [meetingId]);

  const handleGenerate = async () => {
    setGenerating(true);
    setError(null);
    try {
      const transcriptText = buildTranscriptText(transcripts);
      if (!canGenerate) {
        setError('Esta reunión es muy corta para generar evaluación. Necesitas al menos 100 caracteres de transcript.');
        setGenerating(false);
        return;
      }
      const res = await invoke<PostMeetingEvaluationResult>('coach_evaluate_post_meeting', {
        meetingId,
        transcript: transcriptText,
        previousSessionId: previousMeetingId ?? null,
        evaluationModel: null,
      });
      setResult(res);
    } catch (e) {
      const errorMsg = String(e);
      if (errorMsg.toLowerCase().includes('ollama') || errorMsg.toLowerCase().includes('modelo')) {
        setError('El modelo de evaluación (gemma3:4b) no está disponible. Verifica que Ollama esté corriendo.');
        // Dispara evento para OllamaStatus widget
        if (typeof globalThis !== 'undefined' && globalThis.window) {
          globalThis.window.dispatchEvent(new CustomEvent('verify-ollama-status'));
        }
      } else if (errorMsg.toLowerCase().includes('transcripción') || errorMsg.toLowerCase().includes('transcript')) {
        setError('Esta reunión es muy corta para generar evaluación. Necesitas al menos 100 caracteres de transcript.');
      } else {
        setError(errorMsg);
      }
    } finally {
      setGenerating(false);
    }
  };

  const handleExportPdf = async () => {
    setExporting(true);
    setError(null);
    setExportSuccess(null);
    try {
      const pdfPath = await invoke<string>('export_evaluation_pdf', {
        meetingId,
        outputPath: null,
      });
      setExportSuccess(pdfPath);
      // Auto-dismiss success message después de 5 segundos
      setTimeout(() => setExportSuccess(null), 5000);
    } catch (e) {
      setError(String(e));
    } finally {
      setExporting(false);
    }
  };

  const handleOpenFolder = async () => {
    if (!exportSuccess) return;
    try {
      await invoke('show_in_folder', { path: exportSuccess });
    } catch (e) {
      setError(String(e));
    }
  };

  const transcriptText = useMemo(() => buildTranscriptText(transcripts), [transcripts]);
  const canGenerate = transcriptText.trim().length >= 100;

  if (loading) {
    return (
      <div className="p-6 text-sm text-[#6a6a6d] dark:text-gray-400">
        Cargando evaluación…
      </div>
    );
  }

  if (!result) {
    return (
      <div className="p-6 flex flex-col items-center text-center gap-4">
        <Sparkles className="w-10 h-10 text-[#485df4]" />
        <h3 className="text-lg font-semibold text-[#3a3a3c] dark:text-gray-200">Aún no hay evaluación</h3>
        <p className="text-sm text-[#6a6a6d] dark:text-gray-400 max-w-md">
          Genera el análisis profundo localmente con Ollama. Tarda ~30-60s. Por defecto usa
          <code className="mx-1 px-1.5 py-0.5 rounded bg-[#f5f5f6] dark:bg-gray-700 text-xs">gemma3:4b</code>
          (~3GB, corre en laptops 8GB RAM).
        </p>
        {!canGenerate && (
          <div className="flex items-center gap-2 text-xs text-[#cc0040]">
            <AlertTriangle className="w-4 h-4" /> Transcripción muy corta (mínimo 100 caracteres).
          </div>
        )}
        <button
          onClick={handleGenerate}
          disabled={!canGenerate || generating}
          className="px-5 py-2.5 rounded-lg bg-[#485df4] text-white font-medium shadow-sm hover:bg-[#3a4ac3] disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {generating ? 'Analizando con Gemma 4…' : 'Generar evaluación'}
        </button>
        {error && <div className="text-xs text-[#cc0040] max-w-md">{error}</div>}
      </div>
    );
  }

  const ev = result.evaluation;
  const gauge = ev.visualizaciones?.gauge ?? { valor: ev.resumen.puntuacion_global, label: ev.resumen.nivel };
  const radar = ev.visualizaciones?.radar_calidad ?? { labels: [], valores: [] };
  const muletillas = ev.visualizaciones?.muletillas_chart ?? { labels: [], valores: [] };

  return (
    <div className="p-6 space-y-6 overflow-y-auto custom-scrollbar h-full">
      <div className="flex items-start justify-between gap-4">
        <div>
          <div className="text-xs uppercase tracking-wider text-[#8a8a8d]">Evaluación post-meeting</div>
          <h2 className="text-xl font-semibold text-[#3a3a3c] dark:text-gray-100">
            {ev.identificacion?.nombre_sesion || 'Reunión'}
          </h2>
          <div className="text-sm text-[#6a6a6d] mt-1">{ev.resumen.descripcion}</div>
        </div>
        <div className="flex gap-2 flex-wrap">
          <ComplianceReportButton meetingId={meetingId} />
          <button
            onClick={handleExportPdf}
            disabled={exporting}
            title="Exportar evaluación como PDF"
            className="px-3 py-1.5 text-xs rounded-md border border-[#d0d0d3] hover:bg-[#f5f5f6] dark:border-gray-600 dark:hover:bg-gray-800 disabled:opacity-50 flex items-center gap-1.5"
          >
            <Download className="w-3.5 h-3.5" />
            {exporting ? 'Exportando…' : 'PDF'}
          </button>
          <button
            onClick={handleGenerate}
            disabled={generating}
            className="px-3 py-1.5 text-xs rounded-md border border-[#d0d0d3] hover:bg-[#f5f5f6] dark:border-gray-600 dark:hover:bg-gray-800 disabled:opacity-50"
          >
            {generating ? 'Re-evaluando…' : 'Re-evaluar'}
          </button>
        </div>
      </div>

      {exportSuccess && (
        <div className="flex items-center gap-2 p-3 bg-[#f0fdf4] dark:bg-green-900/20 border border-[#1bea9a] rounded-lg text-sm">
          <CheckCircle className="w-4 h-4 text-[#1bea9a]" />
          <span className="flex-1 text-[#1bea9a]">PDF guardado correctamente</span>
          <button
            onClick={handleOpenFolder}
            className="text-xs text-[#1bea9a] underline hover:font-semibold"
          >
            Abrir carpeta
          </button>
        </div>
      )}

      {error && (
        <div className="flex items-center gap-2 p-3 bg-[#fdf2f8] dark:bg-red-900/20 border border-[#ff0050] rounded-lg text-sm">
          <AlertTriangle className="w-4 h-4 text-[#ff0050]" />
          <span className="flex-1 text-[#ff0050]">{error}</span>
        </div>
      )}

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        <div className="bg-white dark:bg-gray-800 rounded-xl border border-[#e7e7e9] dark:border-gray-700 p-5 flex flex-col items-center">
          <div className="text-xs uppercase tracking-wider text-[#8a8a8d] mb-2">Puntuación global</div>
          <GaugeWidget valor={gauge.valor} label={gauge.label} />
          {typeof ev.historico?.tendencia_global === 'number' && (
            <div className="mt-3 flex items-center gap-1.5 text-xs">
              <TrendingUp className="w-4 h-4" />
              <span className={ev.historico.tendencia_global >= 0 ? 'text-[#1bea9a]' : 'text-[#ff0050]'}>
                {ev.historico.tendencia_global >= 0 ? '+' : ''}{ev.historico.tendencia_global.toFixed(1)} vs sesión anterior
              </span>
            </div>
          )}
        </div>

        <div className="bg-white dark:bg-gray-800 rounded-xl border border-[#e7e7e9] dark:border-gray-700 p-5">
          <div className="text-xs uppercase tracking-wider text-[#8a8a8d] mb-2">Radar 6 dimensiones</div>
          {radar.labels.length > 0 ? (
            <RadarWidget labels={radar.labels} valores={radar.valores} />
          ) : (
            <div className="text-sm text-[#6a6a6d]">Sin radar disponible.</div>
          )}
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
        <div className="bg-white dark:bg-gray-800 rounded-xl border border-[#e7e7e9] dark:border-gray-700 p-5">
          <div className="text-xs uppercase tracking-wider text-[#8a8a8d] mb-2">Muletillas detectadas</div>
          <MuletillasChart labels={muletillas.labels} valores={muletillas.valores} />
        </div>

        <div className="bg-white dark:bg-gray-800 rounded-xl border border-[#e7e7e9] dark:border-gray-700 p-5">
          <div className="text-xs uppercase tracking-wider text-[#8a8a8d] mb-2">Resumen ejecutivo</div>
          <div className="space-y-3 text-sm">
            <div>
              <span className="font-semibold text-[#1bea9a]">Fortaleza:</span> {ev.resumen.fortaleza || '—'}
            </div>
            <div>
              <span className="font-semibold text-[#ff0050]">A mejorar:</span> {ev.resumen.mejorar || '—'}
            </div>
            <div>
              <span className="font-semibold text-[#3a3a3c] dark:text-gray-200">Patrón actual:</span>{' '}
              {ev.patron?.actual || '—'}
            </div>
            <div>
              <span className="font-semibold text-[#3a3a3c] dark:text-gray-200">Qué cambiar:</span>{' '}
              {ev.patron?.que_cambiaria || '—'}
            </div>
          </div>
        </div>
      </div>

      {ev.insights && ev.insights.length > 0 && (
        <div className="bg-white dark:bg-gray-800 rounded-xl border border-[#e7e7e9] dark:border-gray-700 p-5">
          <div className="text-xs uppercase tracking-wider text-[#8a8a8d] mb-3 flex items-center gap-2">
            <MessageSquare className="w-4 h-4" /> Insights
          </div>
          <div className="space-y-3">
            {ev.insights.map((ins, i) => (
              <div key={i} className="border-l-2 border-[#485df4] pl-3 text-sm">
                <div className="font-medium text-[#3a3a3c] dark:text-gray-200">{ins.dato}</div>
                <div className="text-xs text-[#6a6a6d] mt-0.5">{ins.por_que}</div>
                <div className="text-xs text-[#3a4ac3] dark:text-blue-300 mt-1">→ {ins.sugerencia}</div>
              </div>
            ))}
          </div>
        </div>
      )}

      {ev.recomendaciones && ev.recomendaciones.length > 0 && (
        <div className="bg-white dark:bg-gray-800 rounded-xl border border-[#e7e7e9] dark:border-gray-700 p-5">
          <div className="text-xs uppercase tracking-wider text-[#8a8a8d] mb-3">Recomendaciones priorizadas</div>
          <div className="space-y-3">
            {ev.recomendaciones
              .slice()
              .sort((a, b) => a.prioridad - b.prioridad)
              .map((r, i) => (
                <div key={i} className="rounded-lg bg-[#f5f5f6] dark:bg-gray-900 p-3">
                  <div className="text-xs font-bold text-[#485df4]">PRIORIDAD {r.prioridad}</div>
                  <div className="font-semibold text-[#3a3a3c] dark:text-gray-200 mt-1">{r.titulo}</div>
                  <div className="text-sm text-[#4a4a4c] dark:text-gray-300 mt-1">{r.texto_mejorado}</div>
                </div>
              ))}
          </div>
        </div>
      )}

      <div className="text-xs text-[#8a8a8d] flex items-center justify-between pt-2 border-t border-[#e7e7e9] dark:border-gray-700">
        <span>Modelo: {result.model_used} · prompt {result.prompt_version}</span>
        <span>Latencia: {(result.latency_ms / 1000).toFixed(1)}s</span>
      </div>
    </div>
  );
}

export default EvaluationPanel;
