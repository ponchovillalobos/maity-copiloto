'use client';

import React, { useEffect, useRef, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useRouter } from 'next/navigation';
import { Sparkles, Send, X, Loader2, Quote, MessageCircleMore, History } from 'lucide-react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

interface GlobalChatCitation {
  segment_id: string;
  text: string;
  source_type?: string | null;
  audio_start_time?: number | null;
  audio_end_time?: number | null;
  score: number;
  meeting_id?: string;
  meeting_title?: string;
}

interface GlobalChatResponse {
  answer: string;
  citations: GlobalChatCitation[];
  model: string;
  latency_ms: number;
  matched_segments: number;
}

interface ChatTurn {
  role: 'user' | 'assistant';
  content: string;
  citations?: GlobalChatCitation[];
  timestamp: number;
}

const STORAGE_KEY = 'maity_global_chat_history_v1';
const MAX_HISTORY_TURNS = 50;

const SUGGESTED = [
  '¿Qué objeciones recurrentes han surgido?',
  '¿Cuáles son mis acuerdos pendientes?',
  '¿Qué temas se repiten más en mis reuniones?',
  '¿Qué clientes mencionaron precio?',
];

function formatTimestamp(seconds?: number | null): string {
  if (typeof seconds !== 'number' || Number.isNaN(seconds)) return '??:??';
  const total = Math.max(0, Math.floor(seconds));
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
}

export function GlobalChatDrawer() {
  const [open, setOpen] = useState(false);
  const [history, setHistory] = useState<ChatTurn[]>([]);
  const [input, setInput] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const router = useRouter();

  // Cargar historial persistido al abrir por primera vez.
  useEffect(() => {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (raw) {
        const parsed = JSON.parse(raw) as ChatTurn[];
        if (Array.isArray(parsed)) {
          setHistory(parsed.slice(-MAX_HISTORY_TURNS));
        }
      }
    } catch {
      // ignore corrupted storage
    }
  }, []);

  // Persistir historial cada vez que cambia.
  useEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(history.slice(-MAX_HISTORY_TURNS)));
    } catch {
      // ignore quota errors
    }
  }, [history]);

  // Listener para evento global "open-global-chat".
  useEffect(() => {
    const handler = () => setOpen(true);
    window.addEventListener('open-global-chat', handler);
    return () => window.removeEventListener('open-global-chat', handler);
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

  // Auto-scroll al fondo cuando llega mensaje nuevo.
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [history, loading, open]);

  const handleAsk = useCallback(
    async (question?: string) => {
      const q = (question ?? input).trim();
      if (!q || loading) return;
      setError(null);
      setInput('');
      setHistory((prev) => [
        ...prev,
        { role: 'user', content: q, timestamp: Date.now() },
      ]);
      setLoading(true);
      try {
        const res = await invoke<GlobalChatResponse>('chat_with_history', {
          query: q,
          topK: 8,
          chatModel: null,
          embedModel: null,
        });
        setHistory((prev) => [
          ...prev,
          {
            role: 'assistant',
            content: res.answer,
            citations: res.citations,
            timestamp: Date.now(),
          },
        ]);
      } catch (e) {
        setError(String(e));
        setHistory((prev) => [
          ...prev,
          { role: 'assistant', content: `Error: ${e}`, timestamp: Date.now() },
        ]);
      } finally {
        setLoading(false);
      }
    },
    [input, loading]
  );

  const handleClearHistory = () => {
    setHistory([]);
    try {
      localStorage.removeItem(STORAGE_KEY);
    } catch {
      /* ignore */
    }
  };

  const goToMeeting = (meetingId?: string) => {
    if (!meetingId) return;
    setOpen(false);
    router.push(`/meeting-details?id=${meetingId}`);
  };

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex justify-end" role="dialog" aria-label="Chat global con historial">
      <div
        className="absolute inset-0 bg-black/40 backdrop-blur-sm"
        onClick={() => setOpen(false)}
      />
      <aside className="relative w-full max-w-md h-full bg-white dark:bg-gray-900 border-l border-[#e7e7e9] dark:border-gray-700 flex flex-col shadow-2xl">
        <header className="flex-shrink-0 flex items-center justify-between px-4 py-3 border-b border-[#e7e7e9] dark:border-gray-700">
          <div className="flex items-center gap-2">
            <Sparkles className="w-5 h-5 text-[#485df4]" />
            <div>
              <div className="text-sm font-semibold text-[#3a3a3c] dark:text-gray-100">
                Chat con tu historial
              </div>
              <div className="text-[11px] text-[#6a6a6d] dark:text-gray-400">
                Pregunta sobre cualquier reunión grabada
              </div>
            </div>
          </div>
          <div className="flex items-center gap-1">
            <button
              onClick={handleClearHistory}
              className="p-1.5 rounded hover:bg-[#f5f5f6] dark:hover:bg-gray-800 text-[#6a6a6d] hover:text-[#3a3a3c] dark:hover:text-gray-200"
              title="Limpiar historial de chat"
              aria-label="Limpiar historial"
            >
              <History className="w-4 h-4" />
            </button>
            <button
              onClick={() => setOpen(false)}
              className="p-1.5 rounded hover:bg-[#f5f5f6] dark:hover:bg-gray-800 text-[#6a6a6d] hover:text-[#3a3a3c] dark:hover:text-gray-200"
              title="Cerrar"
              aria-label="Cerrar"
            >
              <X className="w-4 h-4" />
            </button>
          </div>
        </header>

        <div ref={scrollRef} className="flex-1 overflow-y-auto custom-scrollbar p-4 space-y-3">
          {history.length === 0 && !loading && (
            <div className="py-8 text-center">
              <MessageCircleMore className="w-10 h-10 text-[#485df4] mx-auto mb-3 opacity-70" />
              <div className="text-sm text-[#3a3a3c] dark:text-gray-200 font-medium mb-1">
                ¿Sobre qué quieres preguntar?
              </div>
              <div className="text-xs text-[#6a6a6d] dark:text-gray-400 mb-4">
                Las respuestas se basan en tus reuniones indexadas y citan literalmente con timestamps.
              </div>
              <div className="grid grid-cols-1 gap-2 max-w-sm mx-auto">
                {SUGGESTED.map((q) => (
                  <button
                    key={q}
                    onClick={() => handleAsk(q)}
                    className="text-left text-xs px-3 py-2 rounded-lg bg-[#f5f5f6] dark:bg-gray-800 border border-[#e7e7e9] dark:border-gray-700 hover:border-[#485df4] hover:text-[#3a4ac3] dark:hover:text-blue-300 transition-colors"
                  >
                    {q}
                  </button>
                ))}
              </div>
            </div>
          )}

          {history.map((turn, i) => (
            <div key={i} className={`flex ${turn.role === 'user' ? 'justify-end' : 'justify-start'}`}>
              <div
                className={`max-w-[88%] rounded-2xl px-4 py-3 ${
                  turn.role === 'user'
                    ? 'bg-[#485df4] text-white'
                    : 'bg-white dark:bg-gray-800 border border-[#e7e7e9] dark:border-gray-700 shadow-sm'
                }`}
              >
                {turn.role === 'user' ? (
                  <div className="text-[14px] leading-relaxed whitespace-pre-wrap text-white">
                    {turn.content}
                  </div>
                ) : (
                  <div className="text-[14px] leading-relaxed text-[#1f2025] dark:text-gray-100 chat-markdown">
                    <ReactMarkdown
                      remarkPlugins={[remarkGfm]}
                      components={{
                        p: ({ children }) => <p className="mb-2 last:mb-0">{children}</p>,
                        strong: ({ children }) => (
                          <strong className="font-semibold text-[#1f2025] dark:text-white">
                            {children}
                          </strong>
                        ),
                        em: ({ children }) => <em className="italic">{children}</em>,
                        ul: ({ children }) => (
                          <ul className="list-disc list-outside pl-5 my-2 space-y-1">{children}</ul>
                        ),
                        ol: ({ children }) => (
                          <ol className="list-decimal list-outside pl-5 my-2 space-y-1">{children}</ol>
                        ),
                        li: ({ children }) => <li className="leading-relaxed">{children}</li>,
                        h1: ({ children }) => (
                          <h1 className="text-base font-bold mt-2 mb-1">{children}</h1>
                        ),
                        h2: ({ children }) => (
                          <h2 className="text-sm font-bold mt-2 mb-1">{children}</h2>
                        ),
                        h3: ({ children }) => (
                          <h3 className="text-sm font-semibold mt-2 mb-1">{children}</h3>
                        ),
                        code: ({ children }) => (
                          <code className="px-1 py-0.5 rounded bg-[#f0f2fe] dark:bg-gray-900 text-[#3a4ac3] dark:text-blue-300 text-[13px] font-mono">
                            {children}
                          </code>
                        ),
                        blockquote: ({ children }) => (
                          <blockquote className="border-l-2 border-[#485df4] pl-3 my-2 italic text-[#4a4a4c] dark:text-gray-300">
                            {children}
                          </blockquote>
                        ),
                        a: ({ children, href }) => (
                          <a
                            href={href}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="text-[#485df4] dark:text-blue-400 underline"
                          >
                            {children}
                          </a>
                        ),
                      }}
                    >
                      {turn.content}
                    </ReactMarkdown>
                  </div>
                )}
                {turn.citations && turn.citations.length > 0 && (
                  <div className="mt-2.5 space-y-1.5">
                    {turn.citations.slice(0, 4).map((c) => (
                      <button
                        key={c.segment_id}
                        onClick={() => goToMeeting(c.meeting_id)}
                        className="w-full flex items-start gap-1.5 text-xs bg-white dark:bg-gray-900 hover:bg-[#f0f2fe] dark:hover:bg-gray-700 transition rounded-md p-2 text-left"
                        title={c.meeting_id ? 'Abrir reunión' : ''}
                      >
                        <Quote className="w-3 h-3 mt-0.5 flex-shrink-0 text-[#8a8a8d]" />
                        <div className="flex-1 min-w-0">
                          {c.meeting_title && (
                            <div className="text-[10px] font-semibold text-[#485df4] truncate">
                              {c.meeting_title}
                              <span className="font-mono ml-1.5 text-[#8a8a8d]">
                                [{formatTimestamp(c.audio_start_time)}]
                              </span>
                            </div>
                          )}
                          <div className="text-[#4a4a4c] dark:text-gray-300 line-clamp-2">{c.text}</div>
                        </div>
                      </button>
                    ))}
                  </div>
                )}
              </div>
            </div>
          ))}

          {loading && (
            <div className="flex items-center gap-2 text-sm text-[#6a6a6d] dark:text-gray-400">
              <Loader2 className="w-4 h-4 animate-spin" /> Buscando en tu historial…
            </div>
          )}

          {error && <div className="text-xs text-[#cc0040]">{error}</div>}
        </div>

        <footer className="flex-shrink-0 p-3 border-t border-[#e7e7e9] dark:border-gray-700 bg-white dark:bg-gray-900">
          <div className="flex items-end gap-2">
            <textarea
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && !e.shiftKey) {
                  e.preventDefault();
                  handleAsk();
                }
              }}
              placeholder="Pregunta sobre tu historial…"
              rows={1}
              disabled={loading}
              className="flex-1 resize-none rounded-lg border border-[#d0d0d3] dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-[#485df4]/40 focus:border-[#485df4] disabled:opacity-50"
            />
            <button
              onClick={() => handleAsk()}
              disabled={loading || !input.trim()}
              className="p-2 rounded-lg bg-[#485df4] text-white hover:bg-[#3a4ac3] disabled:opacity-50 disabled:cursor-not-allowed"
              aria-label="Enviar pregunta"
            >
              <Send className="w-4 h-4" />
            </button>
          </div>
          <div className="text-[10px] text-[#8a8a8d] mt-1.5 px-1">
            {history.length > 0 && `${history.length} mensajes guardados localmente · `}
            Esc para cerrar · Enter envía · Shift+Enter nueva línea
          </div>
        </footer>
      </aside>
    </div>
  );
}

export default GlobalChatDrawer;
