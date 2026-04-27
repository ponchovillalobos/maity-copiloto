'use client';

import React, { useEffect, useRef, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useRouter } from 'next/navigation';
import {
  BookOpen,
  Send,
  X,
  Loader2,
  Copy,
  Check,
  AlertCircle,
  Lightbulb,
  TrendingUp,
} from 'lucide-react';

interface PlaybookCita {
  meeting_title: string;
  quote: string;
  speaker: string;
}

interface PlaybookScript {
  contexto: string;
  respuesta_recomendada: string;
  fuente_meeting: string;
}

interface PlaybookInsight {
  patron_principal: string;
  frecuencia: string;
  citas_clave: PlaybookCita[];
  scripts_validados: PlaybookScript[];
  recomendaciones: string[];
  anti_patrones: string[];
}

interface PlaybookResult {
  query: string;
  insight: PlaybookInsight;
  meetings_analyzed: number;
  model: string;
  latency_ms: number;
}

const SUGGESTED_QUERIES = [
  'Objeciones de precio recurrentes',
  'Cómo respondieron clientes a nuestra propuesta',
  'Patterns en reuniones cerradas',
  'Dudas técnicas frecuentes',
];

const STORAGE_KEY = 'maity_playbook_last_query';

export function PlaybookDrawer() {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState('');
  const [result, setResult] = useState<PlaybookResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const router = useRouter();

  // Cargar query persistida al abrir.
  useEffect(() => {
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved) setQuery(saved);
    } catch {
      // ignore
    }
  }, []);

  // Persistir query cuando cambia.
  useEffect(() => {
    try {
      if (query) localStorage.setItem(STORAGE_KEY, query);
    } catch {
      // ignore
    }
  }, [query]);

  // Listener para evento global "open-playbook".
  useEffect(() => {
    const handler = () => setOpen(true);
    window.addEventListener('open-playbook', handler);
    return () => window.removeEventListener('open-playbook', handler);
  }, []);

  // Escape cierra drawer.
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setOpen(false);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open]);

  // Auto-scroll al fondo cuando llega resultado.
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [result, loading, open]);

  const handleAnalyze = useCallback(
    async (q?: string) => {
      const question = (q ?? query).trim();
      if (!question || loading) return;
      setError(null);
      setLoading(true);
      try {
        const res = await invoke<PlaybookResult>('generate_playbook', {
          query: question,
          topK: 15,
          chatModel: null,
          embedModel: null,
        });
        setResult(res);
      } catch (e) {
        setError(String(e));
        setResult(null);
      } finally {
        setLoading(false);
      }
    },
    [query, loading]
  );

  const copyToClipboard = (text: string, index: number) => {
    navigator.clipboard.writeText(text);
    setCopiedIndex(index);
    setTimeout(() => setCopiedIndex(null), 2000);
  };

  const goToMeeting = (title?: string) => {
    if (!title) return;
    setOpen(false);
    // Search para meeting por título
    router.push(`/?search=${encodeURIComponent(title)}`);
  };

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex justify-end" role="dialog" aria-label="Playbook cross-prospect">
      <div
        className="absolute inset-0 bg-black/40 backdrop-blur-sm"
        onClick={() => setOpen(false)}
      />
      <aside className="relative w-full max-w-2xl h-full bg-white dark:bg-gray-900 border-l border-[#e7e7e9] dark:border-gray-700 flex flex-col shadow-2xl">
        <header className="flex-shrink-0 flex items-center justify-between px-4 py-3 border-b border-[#e7e7e9] dark:border-gray-700">
          <div className="flex items-center gap-2">
            <BookOpen className="w-5 h-5 text-[#485df4]" />
            <div>
              <div className="text-sm font-semibold text-[#3a3a3c] dark:text-gray-100">
                Playbook Cross-Prospect
              </div>
              <div className="text-[11px] text-[#6a6a6d] dark:text-gray-400">
                Patrones a través de todas tus reuniones
              </div>
            </div>
          </div>
          <button
            onClick={() => setOpen(false)}
            className="p-1.5 rounded hover:bg-[#f5f5f6] dark:hover:bg-gray-800 text-[#6a6a6d] hover:text-[#3a3a3c] dark:hover:text-gray-200"
            title="Cerrar"
            aria-label="Cerrar"
          >
            <X className="w-4 h-4" />
          </button>
        </header>

        <div ref={scrollRef} className="flex-1 overflow-y-auto custom-scrollbar p-4 space-y-4">
          {!result && !loading && (
            <div className="py-8 text-center">
              <BookOpen className="w-10 h-10 text-[#485df4] mx-auto mb-3 opacity-70" />
              <div className="text-sm text-[#3a3a3c] dark:text-gray-200 font-medium mb-1">
                ¿Qué patrón quieres analizar?
              </div>
              <div className="text-xs text-[#6a6a6d] dark:text-gray-400 mb-4">
                Busca patrones recurrentes a través de todas tus reuniones grabadas.
              </div>
              <div className="space-y-2 max-w-sm mx-auto">
                {SUGGESTED_QUERIES.map((q) => (
                  <button
                    key={q}
                    onClick={() => {
                      setQuery(q);
                      setTimeout(() => handleAnalyze(q), 100);
                    }}
                    className="w-full text-left text-xs px-3 py-2 rounded-lg bg-[#f5f5f6] dark:bg-gray-800 border border-[#e7e7e9] dark:border-gray-700 hover:border-[#485df4] hover:text-[#3a4ac3] dark:hover:text-blue-300 transition-colors"
                  >
                    {q}
                  </button>
                ))}
              </div>
            </div>
          )}

          {loading && (
            <div className="py-8 flex flex-col items-center">
              <Loader2 className="w-6 h-6 animate-spin text-[#485df4] mb-2" />
              <div className="text-sm text-[#6a6a6d] dark:text-gray-400">
                Buscando en tus reuniones…
              </div>
            </div>
          )}

          {error && (
            <div className="bg-red-50 dark:bg-red-900/20 border border-[#cc0040] rounded-lg p-3 flex gap-2">
              <AlertCircle className="w-4 h-4 text-[#cc0040] flex-shrink-0 mt-0.5" />
              <div className="text-xs text-[#cc0040]">{error}</div>
            </div>
          )}

          {result && !loading && (
            <div className="space-y-4">
              {/* Patrón Principal */}
              <div className="bg-gradient-to-br from-[#485df4]/10 to-[#3a4ac3]/5 border border-[#485df4]/20 rounded-lg p-4">
                <div className="flex items-start gap-3">
                  <TrendingUp className="w-5 h-5 text-[#485df4] flex-shrink-0 mt-0.5" />
                  <div className="flex-1">
                    <div className="text-xs font-semibold text-[#8a8a8d] mb-1">PATRÓN PRINCIPAL</div>
                    <div className="text-sm font-medium text-[#1f2025] dark:text-gray-100 mb-2">
                      {result.insight.patron_principal}
                    </div>
                    <div className="inline-block bg-[#485df4] text-white text-[10px] font-semibold px-2.5 py-1 rounded-full">
                      {result.insight.frecuencia}
                    </div>
                  </div>
                </div>
              </div>

              {/* Citas Clave */}
              {result.insight.citas_clave.length > 0 && (
                <div>
                  <div className="text-xs font-semibold text-[#8a8a8d] mb-2">CITAS CLAVE ({result.insight.citas_clave.length})</div>
                  <div className="space-y-2">
                    {result.insight.citas_clave.map((cita, idx) => (
                      <button
                        key={idx}
                        onClick={() => goToMeeting(cita.meeting_title)}
                        className="w-full text-left bg-white dark:bg-gray-800 border border-[#e7e7e9] dark:border-gray-700 rounded-lg p-3 hover:border-[#485df4] hover:bg-[#f0f2fe] dark:hover:bg-gray-700/50 transition-colors"
                      >
                        <div className="flex items-start gap-2">
                          <div className="text-[10px] font-semibold text-[#485df4] bg-[#f0f2fe] dark:bg-blue-900/30 px-2 py-0.5 rounded">
                            {cita.speaker === 'USUARIO' ? 'TÚ' : 'CLIENTE'}
                          </div>
                          <div className="flex-1">
                            <div className="text-[10px] text-[#8a8a8d] mb-1 truncate">
                              {cita.meeting_title}
                            </div>
                            <div className="text-xs text-[#1f2025] dark:text-gray-200 italic line-clamp-2">
                              "{cita.quote}"
                            </div>
                          </div>
                        </div>
                      </button>
                    ))}
                  </div>
                </div>
              )}

              {/* Scripts Validados */}
              {result.insight.scripts_validados.length > 0 && (
                <div>
                  <div className="text-xs font-semibold text-[#8a8a8d] mb-2">SCRIPTS VALIDADOS ({result.insight.scripts_validados.length})</div>
                  <div className="space-y-2">
                    {result.insight.scripts_validados.map((script, idx) => (
                      <div
                        key={idx}
                        className="bg-[#f5f5f6] dark:bg-gray-800 border border-[#e7e7e9] dark:border-gray-700 rounded-lg p-3"
                      >
                        <div className="text-[10px] text-[#6a6a6d] mb-1.5 font-semibold">
                          Contexto: {script.contexto}
                        </div>
                        <div className="bg-white dark:bg-gray-900 rounded p-2 mb-2 border border-[#e7e7e9] dark:border-gray-600">
                          <div className="text-xs text-[#1f2025] dark:text-gray-200 italic mb-2">
                            "{script.respuesta_recomendada}"
                          </div>
                          <div className="text-[10px] text-[#8a8a8d]">
                            Fuente: {script.fuente_meeting}
                          </div>
                        </div>
                        <button
                          onClick={() => copyToClipboard(script.respuesta_recomendada, idx)}
                          className="flex items-center gap-1.5 text-[10px] px-2 py-1 rounded bg-[#485df4] text-white hover:bg-[#3a4ac3] transition-colors"
                        >
                          {copiedIndex === idx ? (
                            <>
                              <Check className="w-3 h-3" /> Copiado
                            </>
                          ) : (
                            <>
                              <Copy className="w-3 h-3" /> Copiar
                            </>
                          )}
                        </button>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* Recomendaciones */}
              {result.insight.recomendaciones.length > 0 && (
                <div>
                  <div className="text-xs font-semibold text-[#8a8a8d] mb-2 flex items-center gap-1.5">
                    <Lightbulb className="w-4 h-4" /> RECOMENDACIONES
                  </div>
                  <div className="bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg p-3 space-y-1.5">
                    {result.insight.recomendaciones.map((rec, idx) => (
                      <div key={idx} className="flex items-start gap-2">
                        <div className="w-1.5 h-1.5 rounded-full bg-green-600 mt-1.5 flex-shrink-0" />
                        <div className="text-xs text-green-900 dark:text-green-200">{rec}</div>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* Anti-patrones */}
              {result.insight.anti_patrones.length > 0 && (
                <div>
                  <div className="text-xs font-semibold text-[#8a8a8d] mb-2 flex items-center gap-1.5">
                    <AlertCircle className="w-4 h-4" /> ANTI-PATRONES
                  </div>
                  <div className="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-3 space-y-1.5">
                    {result.insight.anti_patrones.map((anti, idx) => (
                      <div key={idx} className="flex items-start gap-2">
                        <div className="w-1.5 h-1.5 rounded-full bg-red-600 mt-1.5 flex-shrink-0" />
                        <div className="text-xs text-red-900 dark:text-red-200">{anti}</div>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* Metadata */}
              <div className="text-[10px] text-[#8a8a8d] border-t border-[#e7e7e9] dark:border-gray-700 pt-2">
                Analizado {result.meetings_analyzed} reunión{result.meetings_analyzed !== 1 ? 'es' : ''} · Modelo: {result.model} · {result.latency_ms}ms
              </div>
            </div>
          )}
        </div>

        <footer className="flex-shrink-0 p-3 border-t border-[#e7e7e9] dark:border-gray-700 bg-white dark:bg-gray-900 space-y-2">
          <div className="flex items-end gap-2">
            <textarea
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && !e.shiftKey) {
                  e.preventDefault();
                  handleAnalyze();
                }
              }}
              placeholder="¿Qué patrón quieres analizar?"
              rows={2}
              disabled={loading}
              className="flex-1 resize-none rounded-lg border border-[#d0d0d3] dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-[#485df4]/40 focus:border-[#485df4] disabled:opacity-50"
            />
            <button
              onClick={() => handleAnalyze()}
              disabled={loading || !query.trim()}
              className="p-2 rounded-lg bg-[#485df4] text-white hover:bg-[#3a4ac3] disabled:opacity-50 disabled:cursor-not-allowed flex-shrink-0"
              aria-label="Analizar patrón"
            >
              <Send className="w-4 h-4" />
            </button>
          </div>
          <div className="text-[10px] text-[#8a8a8d] px-1">
            Esc para cerrar · Enter analiza · Shift+Enter nueva línea
          </div>
        </footer>
      </aside>
    </div>
  );
}

export default PlaybookDrawer;
