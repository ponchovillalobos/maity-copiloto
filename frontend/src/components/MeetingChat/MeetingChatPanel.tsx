'use client';

import React, { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Send, Sparkles, Quote, Loader2 } from 'lucide-react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

interface MeetingChatCitation {
  segment_id: string;
  text: string;
  source_type?: string | null;
  audio_start_time?: number | null;
  audio_end_time?: number | null;
  score: number;
}

interface MeetingChatResponse {
  answer: string;
  citations: MeetingChatCitation[];
  model: string;
  latency_ms: number;
  matched_segments: number;
}

interface ChatTurn {
  role: 'user' | 'assistant';
  content: string;
  citations?: MeetingChatCitation[];
}

const SUGGESTED_QUESTIONS = [
  '¿Qué acuerdos se alcanzaron?',
  '¿Qué objeciones surgieron?',
  '¿Cuáles fueron las próximas acciones?',
  '¿Qué temas quedaron pendientes?',
];

function formatTimestamp(seconds?: number | null): string {
  if (typeof seconds !== 'number' || Number.isNaN(seconds)) return '??:??';
  const total = Math.max(0, Math.floor(seconds));
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
}

interface Props {
  meetingId: string;
}

export function MeetingChatPanel({ meetingId }: Props) {
  const [history, setHistory] = useState<ChatTurn[]>([]);
  const [input, setInput] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [history, loading]);

  const handleAsk = async (question?: string) => {
    const q = (question ?? input).trim();
    if (!q || loading) return;
    setError(null);
    setInput('');
    const userTurn: ChatTurn = { role: 'user', content: q };
    setHistory(prev => [...prev, userTurn]);
    setLoading(true);
    try {
      const res = await invoke<MeetingChatResponse>('chat_with_meeting', {
        meetingId,
        query: q,
        topK: 5,
        chatModel: null,
        embedModel: null,
      });
      setHistory(prev => [
        ...prev,
        { role: 'assistant', content: res.answer, citations: res.citations },
      ]);
    } catch (e) {
      setError(String(e));
      setHistory(prev => [
        ...prev,
        { role: 'assistant', content: `Error: ${e}`, citations: [] },
      ]);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="flex flex-col h-full bg-[#f5f5f6] dark:bg-gray-900">
      <div className="flex-shrink-0 px-5 py-3 border-b border-[#e7e7e9] dark:border-gray-700 flex items-center gap-2">
        <Sparkles className="w-4 h-4 text-[#485df4]" />
        <h3 className="text-sm font-semibold text-[#3a3a3c] dark:text-gray-100">
          Chat con la reunión
        </h3>
      </div>

      <div ref={scrollRef} className="flex-1 overflow-y-auto custom-scrollbar p-5 space-y-4">
        {history.length === 0 && !loading && (
          <div className="text-center py-10">
            <div className="text-sm text-[#6a6a6d] dark:text-gray-400 mb-4">
              Pregunta sobre lo que se dijo. Las respuestas citan literalmente con timestamps.
            </div>
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 max-w-md mx-auto">
              {SUGGESTED_QUESTIONS.map(q => (
                <button
                  key={q}
                  onClick={() => handleAsk(q)}
                  className="text-left text-xs px-3 py-2 rounded-lg bg-white dark:bg-gray-800 border border-[#e7e7e9] dark:border-gray-700 hover:border-[#485df4] hover:text-[#3a4ac3] dark:hover:text-blue-300 transition-colors"
                >
                  {q}
                </button>
              ))}
            </div>
          </div>
        )}

        {history.map((turn, i) => (
          <div
            key={i}
            className={`flex ${turn.role === 'user' ? 'justify-end' : 'justify-start'}`}
          >
            <div
              className={`max-w-[85%] rounded-2xl px-4 py-3 ${
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
                <div className="text-[14px] leading-relaxed text-[#1f2025] dark:text-gray-100">
                  <ReactMarkdown
                    remarkPlugins={[remarkGfm]}
                    components={{
                      p: ({ children }) => <p className="mb-2 last:mb-0">{children}</p>,
                      strong: ({ children }) => (
                        <strong className="font-semibold text-[#1f2025] dark:text-white">{children}</strong>
                      ),
                      ul: ({ children }) => (
                        <ul className="list-disc list-outside pl-5 my-2 space-y-1">{children}</ul>
                      ),
                      ol: ({ children }) => (
                        <ol className="list-decimal list-outside pl-5 my-2 space-y-1">{children}</ol>
                      ),
                      li: ({ children }) => <li className="leading-relaxed">{children}</li>,
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
                    }}
                  >
                    {turn.content}
                  </ReactMarkdown>
                </div>
              )}
              {turn.citations && turn.citations.length > 0 && (
                <div className="mt-3 space-y-1.5">
                  {turn.citations.slice(0, 3).map(c => (
                    <div
                      key={c.segment_id}
                      className="flex items-start gap-1.5 text-xs bg-[#f5f5f6] dark:bg-gray-900 rounded-md p-2"
                    >
                      <Quote className="w-3 h-3 mt-0.5 flex-shrink-0 text-[#8a8a8d]" />
                      <div className="flex-1 min-w-0">
                        <span className="font-mono text-[#485df4] mr-1.5">
                          [{formatTimestamp(c.audio_start_time)}]
                        </span>
                        <span className="text-[#4a4a4c] dark:text-gray-300">{c.text}</span>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          </div>
        ))}

        {loading && (
          <div className="flex items-center gap-2 text-sm text-[#6a6a6d] dark:text-gray-400">
            <Loader2 className="w-4 h-4 animate-spin" /> Consultando la reunión…
          </div>
        )}

        {error && <div className="text-xs text-[#cc0040]">{error}</div>}
      </div>

      <div className="flex-shrink-0 p-3 border-t border-[#e7e7e9] dark:border-gray-700 bg-white dark:bg-gray-800">
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
            placeholder="Pregunta sobre la reunión…"
            rows={1}
            disabled={loading}
            className="flex-1 resize-none rounded-lg border border-[#d0d0d3] dark:border-gray-600 dark:bg-gray-900 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-[#485df4]/40 focus:border-[#485df4] disabled:opacity-50"
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
      </div>
    </div>
  );
}

export default MeetingChatPanel;
