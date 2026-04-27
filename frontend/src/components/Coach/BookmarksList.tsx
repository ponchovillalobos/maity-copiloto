'use client';

import React, { useEffect, useState, useCallback } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { X, Trash2, Star, Clock, DollarSign, CheckCircle, AlertTriangle, ArrowRight } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { useTranscripts } from '@/contexts/TranscriptContext';
import { toast } from 'sonner';

interface Bookmark {
  id: number;
  recording_id: string;
  timestamp_sec: number;
  category: string;
  note: string | null;
  segment_text: string | null;
  created_at: string;
}

const CATEGORY_MAP: Record<string, { label: string; icon: React.ReactNode; color: string }> = {
  important: {
    label: 'Importante',
    icon: <Star className="w-3.5 h-3.5" />,
    color: 'text-yellow-300',
  },
  follow_up: {
    label: 'Seguimiento',
    icon: <Clock className="w-3.5 h-3.5" />,
    color: 'text-blue-300',
  },
  pricing: {
    label: 'Precio',
    icon: <DollarSign className="w-3.5 h-3.5" />,
    color: 'text-green-300',
  },
  decision: {
    label: 'Decisión',
    icon: <CheckCircle className="w-3.5 h-3.5" />,
    color: 'text-emerald-300',
  },
  action_item: {
    label: 'Acción',
    icon: <ArrowRight className="w-3.5 h-3.5" />,
    color: 'text-orange-300',
  },
  risk: {
    label: 'Riesgo',
    icon: <AlertTriangle className="w-3.5 h-3.5" />,
    color: 'text-red-300',
  },
};

function formatTimestamp(seconds: number): string {
  if (typeof seconds !== 'number' || Number.isNaN(seconds)) return '00:00';
  const total = Math.max(0, Math.floor(seconds));
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}`;
}

export function BookmarksList() {
  const { currentMeetingId } = useTranscripts();
  const [open, setOpen] = useState(false);
  const [bookmarks, setBookmarks] = useState<Bookmark[]>([]);
  const [loading, setLoading] = useState(false);

  // Listener para evento global "open-bookmarks-list"
  useEffect(() => {
    const handler = () => setOpen(true);
    window.addEventListener('open-bookmarks-list', handler);
    return () => window.removeEventListener('open-bookmarks-list', handler);
  }, []);

  // Cargar bookmarks cuando se abre el drawer
  useEffect(() => {
    if (!open || !currentMeetingId) {
      return;
    }
    loadBookmarks();
  }, [open, currentMeetingId]);

  // Escape cierra drawer
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setOpen(false);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open]);

  const loadBookmarks = useCallback(async () => {
    if (!currentMeetingId) return;
    setLoading(true);
    try {
      const result = await invoke<Bookmark[]>('coach_get_bookmarks', {
        recordingId: currentMeetingId,
      });
      setBookmarks(result);
    } catch (e) {
      toast.error('Error cargando bookmarks');
    } finally {
      setLoading(false);
    }
  }, [currentMeetingId]);

  const handleDelete = async (id: number) => {
    try {
      await invoke('coach_delete_bookmark', { id });
      setBookmarks((prev) => prev.filter((b) => b.id !== id));
      toast.success('Bookmark eliminado');
    } catch (e) {
      toast.error('Error eliminando bookmark');
    }
  };

  if (!open) return null;

  return (
    <AnimatePresence>
      {open && (
        <>
          {/* Overlay oscuro */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            onClick={() => setOpen(false)}
            className="fixed inset-0 bg-black/40 backdrop-blur-sm z-40"
          />

          {/* Drawer lado derecho */}
          <motion.div
            initial={{ x: 400, opacity: 0 }}
            animate={{ x: 0, opacity: 1 }}
            exit={{ x: 400, opacity: 0 }}
            transition={{ type: 'spring', damping: 20, stiffness: 300 }}
            className="fixed right-0 top-0 h-screen w-96 bg-gray-900/95 backdrop-blur-lg border-l border-gray-700/50 shadow-2xl z-50 flex flex-col"
          >
            {/* Header */}
            <div className="flex items-center justify-between p-4 border-b border-gray-700/30">
              <div className="flex items-center gap-2">
                <div className="p-1.5 rounded bg-purple-600/20 border border-purple-500/30">
                  <Star className="w-4 h-4 text-purple-300" />
                </div>
                <h2 className="text-sm font-semibold text-gray-100">Bookmarks guardados</h2>
              </div>
              <button
                onClick={() => setOpen(false)}
                className="p-1 rounded hover:bg-gray-800/50 transition"
                aria-label="Cerrar"
              >
                <X className="w-4 h-4 text-gray-400" />
              </button>
            </div>

            {/* Content */}
            <div className="flex-1 overflow-y-auto">
              {loading ? (
                <div className="flex items-center justify-center h-full">
                  <div className="text-xs text-gray-400">Cargando...</div>
                </div>
              ) : bookmarks.length === 0 ? (
                <div className="flex flex-col items-center justify-center h-full px-4 text-center">
                  <div className="p-3 rounded-full bg-gray-800/50 mb-3">
                    <Star className="w-5 h-5 text-gray-500" />
                  </div>
                  <p className="text-xs text-gray-400 font-medium">No hay bookmarks</p>
                  <p className="text-xs text-gray-600 mt-1">
                    Usa Ctrl+B durante una grabación para marcar acuerdos
                  </p>
                </div>
              ) : (
                <div className="space-y-2 p-3">
                  {bookmarks.map((bookmark) => {
                    const cat = CATEGORY_MAP[bookmark.category] || CATEGORY_MAP.important;
                    return (
                      <motion.div
                        key={bookmark.id}
                        initial={{ opacity: 0, y: 10 }}
                        animate={{ opacity: 1, y: 0 }}
                        className="group rounded-lg border border-gray-700/40 bg-gray-800/30 hover:bg-gray-800/50 p-3 transition"
                      >
                        <div className="flex items-start gap-2 mb-2">
                          <div className={`flex-shrink-0 mt-0.5 ${cat.color}`}>{cat.icon}</div>
                          <div className="flex-1 min-w-0">
                            <p className="text-xs font-semibold text-gray-200">{cat.label}</p>
                            <p className="text-[10px] text-gray-500">
                              {formatTimestamp(bookmark.timestamp_sec)}
                            </p>
                          </div>
                          <button
                            onClick={() => handleDelete(bookmark.id)}
                            className="flex-shrink-0 p-1 rounded opacity-0 group-hover:opacity-100 hover:bg-red-600/20 text-red-400 transition"
                            aria-label="Eliminar"
                          >
                            <Trash2 className="w-3.5 h-3.5" />
                          </button>
                        </div>

                        {bookmark.segment_text && (
                          <p className="text-[10px] text-gray-300 line-clamp-2 mb-2 italic">
                            "{bookmark.segment_text}"
                          </p>
                        )}

                        {bookmark.note && (
                          <p className="text-[10px] text-gray-400 bg-gray-900/40 rounded px-2 py-1">
                            {bookmark.note}
                          </p>
                        )}
                      </motion.div>
                    );
                  })}
                </div>
              )}
            </div>
          </motion.div>
        </>
      )}
    </AnimatePresence>
  );
}
