'use client';

import React, { useState, useRef, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Bookmark, Star, Clock, DollarSign, CheckCircle, AlertTriangle, ArrowRight } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { useTranscripts } from '@/contexts/TranscriptContext';
import { useRecordingState } from '@/contexts/RecordingStateContext';
import { useCoach } from '@/contexts/CoachContext';

const CATEGORIES = [
  { id: 'important',   label: 'Importante',   icon: <Star className="w-3.5 h-3.5" />,          color: 'text-yellow-300' },
  { id: 'follow_up',   label: 'Seguimiento',  icon: <Clock className="w-3.5 h-3.5" />,         color: 'text-blue-300' },
  { id: 'pricing',     label: 'Precio',       icon: <DollarSign className="w-3.5 h-3.5" />,    color: 'text-green-300' },
  { id: 'decision',    label: 'Decisión',     icon: <CheckCircle className="w-3.5 h-3.5" />,   color: 'text-emerald-300' },
  { id: 'action_item', label: 'Acción',       icon: <ArrowRight className="w-3.5 h-3.5" />,    color: 'text-orange-300' },
  { id: 'risk',        label: 'Riesgo',       icon: <AlertTriangle className="w-3.5 h-3.5" />, color: 'text-red-300' },
];

export function BookmarkButton() {
  const { transcriptsRef, currentMeetingId } = useTranscripts();
  const { isRecording } = useRecordingState();
  const { metrics } = useCoach();
  const [isOpen, setIsOpen] = useState(false);
  const [toast, setToast] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  // Click outside → close
  useEffect(() => {
    if (!isOpen) return;
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setIsOpen(false);
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [isOpen]);

  // Auto-dismiss toast
  useEffect(() => {
    if (!toast) return;
    const t = setTimeout(() => setToast(null), 2000);
    return () => clearTimeout(t);
  }, [toast]);

  if (!isRecording || !currentMeetingId) return null;

  const handleBookmark = async (cat: typeof CATEGORIES[0]) => {
    setBusy(true);
    try {
      const all = transcriptsRef.current ?? [];
      const last = all.length > 0 ? ((all[all.length - 1] as any).text ?? '').slice(0, 200) : '';
      const sec = metrics.durationSec;

      await invoke('coach_add_bookmark', {
        recordingId: currentMeetingId,
        timestampSec: Math.max(0, sec),
        category: cat.id,
        segmentText: last || null,
      });
      setToast(`${cat.label} marcado`);
      setIsOpen(false);
    } catch (e) {
      setToast('Error al marcar');
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <div ref={ref} className="relative">
        <button
          onClick={() => setIsOpen(!isOpen)}
          disabled={busy}
          className="text-[10px] px-2 py-1 rounded bg-purple-600/20 text-purple-200 border border-purple-500/30 hover:bg-purple-600/30 disabled:opacity-40 transition flex items-center gap-1"
          title="Marcar momento importante"
        >
          <Bookmark className="w-3 h-3" />
          Marcar
        </button>

        <AnimatePresence>
          {isOpen && (
            <motion.div
              initial={{ opacity: 0, y: -2, scale: 0.95 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{ opacity: 0, y: -2, scale: 0.95 }}
              transition={{ duration: 0.12 }}
              className="absolute right-0 mt-1.5 w-36 rounded-md border border-gray-700/60 bg-gray-800/95 backdrop-blur-sm shadow-lg z-50 p-0.5"
            >
              {CATEGORIES.map((cat) => (
                <button
                  key={cat.id}
                  onClick={() => handleBookmark(cat)}
                  disabled={busy}
                  className="w-full flex items-center gap-2 px-2.5 py-1.5 rounded text-[11px] text-gray-200 hover:bg-gray-700/60 disabled:opacity-50 transition"
                >
                  <span className={cat.color}>{cat.icon}</span>
                  {cat.label}
                </button>
              ))}
            </motion.div>
          )}
        </AnimatePresence>
      </div>

      <AnimatePresence>
        {toast && (
          <motion.div
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 10 }}
            className="fixed bottom-6 right-6 px-3 py-2 rounded-lg bg-green-600/30 border border-green-500/40 text-green-200 text-xs font-medium shadow-lg z-50 flex items-center gap-1.5"
          >
            <CheckCircle className="w-3.5 h-3.5" />
            {toast}
          </motion.div>
        )}
      </AnimatePresence>
    </>
  );
}
