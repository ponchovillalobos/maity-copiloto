'use client';

import React, { useEffect, useState } from 'react';
import { CheckCircle, AlertCircle, Loader2, Sparkles } from 'lucide-react';
import { isOllamaRunning, listOllamaModels } from '@/lib/ollama';

/**
 * Panel informativo (read-only) sobre la IA local activa.
 * Reemplaza el antiguo selector de modelos para que el usuario no-técnico
 * NO tenga que elegir nada — la app trae todo preconfigurado.
 */
export function AIInfoPanel() {
  const [status, setStatus] = useState<'checking' | 'ok' | 'down'>('checking');
  const [models, setModels] = useState<number>(0);

  useEffect(() => {
    let cancelled = false;
    const check = async () => {
      const running = await isOllamaRunning();
      if (cancelled) return;
      if (!running) {
        setStatus('down');
        setModels(0);
        return;
      }
      const list = await listOllamaModels();
      if (cancelled) return;
      setStatus('ok');
      setModels(list.length);
    };
    check();
    const id = setInterval(check, 30_000);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, []);

  const StatusIcon =
    status === 'checking' ? Loader2 : status === 'ok' ? CheckCircle : AlertCircle;
  const statusColor =
    status === 'checking' ? 'text-gray-400' : status === 'ok' ? 'text-emerald-400' : 'text-rose-400';
  const statusText =
    status === 'checking' ? 'Verificando…' : status === 'ok' ? 'Activa' : 'No disponible';

  return (
    <div className="space-y-6 text-gray-100">
      <div className="rounded-xl border border-white/10 bg-white/5 p-6">
        <div className="flex items-start gap-4">
          <div className="rounded-full p-3 bg-[#485df4]/20 border border-[#485df4]/40">
            <Sparkles className="w-6 h-6 text-[#a8b3ff]" />
          </div>
          <div className="flex-1">
            <div className="flex items-center gap-2">
              <h3 className="text-base font-semibold text-gray-50">IA Local</h3>
              <span className="flex items-center gap-1 text-xs">
                <StatusIcon className={`w-3.5 h-3.5 ${statusColor} ${status === 'checking' ? 'animate-spin' : ''}`} />
                <span className={statusColor}>{statusText}</span>
              </span>
            </div>
            <p className="text-sm text-gray-300 mt-2 leading-relaxed">
              Maity usa modelos de IA que corren <strong className="text-gray-100">100% en tu computadora</strong>.
              Tus conversaciones nunca se envían a servidores externos. No requieres seleccionar
              modelos ni configurar APIs.
            </p>
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        <div className="rounded-lg border border-white/10 bg-white/5 p-4">
          <div className="text-[10px] uppercase tracking-wider text-gray-400 mb-1">Tips en vivo</div>
          <div className="text-sm font-semibold text-gray-50">Gemma 3 (4B)</div>
          <div className="text-xs text-gray-400 mt-1">Sugerencias rápidas mientras hablas.</div>
        </div>
        <div className="rounded-lg border border-white/10 bg-white/5 p-4">
          <div className="text-[10px] uppercase tracking-wider text-gray-400 mb-1">Evaluación post-meeting</div>
          <div className="text-sm font-semibold text-gray-50">Gemma 3 (4B)</div>
          <div className="text-xs text-gray-400 mt-1">Análisis profundo al terminar la reunión.</div>
        </div>
        <div className="rounded-lg border border-white/10 bg-white/5 p-4">
          <div className="text-[10px] uppercase tracking-wider text-gray-400 mb-1">Búsqueda semántica</div>
          <div className="text-sm font-semibold text-gray-50">Nomic Embed</div>
          <div className="text-xs text-gray-400 mt-1">Encuentra momentos de tus reuniones.</div>
        </div>
        <div className="rounded-lg border border-white/10 bg-white/5 p-4">
          <div className="text-[10px] uppercase tracking-wider text-gray-400 mb-1">Transcripción</div>
          <div className="text-sm font-semibold text-gray-50">Parakeet TDT</div>
          <div className="text-xs text-gray-400 mt-1">3.45% WER en español. Sin internet.</div>
        </div>
      </div>

      {status === 'down' && (
        <div className="rounded-lg border border-rose-500/40 bg-rose-500/10 p-4 text-sm">
          <div className="font-semibold text-rose-200 mb-1">Ollama no está corriendo</div>
          <div className="text-rose-100/80 text-xs leading-relaxed">
            La IA local necesita el servicio Ollama. Descárgalo gratis en{' '}
            <a href="https://ollama.com" target="_blank" rel="noreferrer" className="underline">ollama.com</a>{' '}
            e instálalo. Maity arranca el servicio automáticamente cuando está disponible.
          </div>
        </div>
      )}

      {status === 'ok' && (
        <div className="text-xs text-gray-400">
          {models} modelo{models !== 1 ? 's' : ''} detectado{models !== 1 ? 's' : ''} en Ollama local.
        </div>
      )}
    </div>
  );
}

export default AIInfoPanel;
