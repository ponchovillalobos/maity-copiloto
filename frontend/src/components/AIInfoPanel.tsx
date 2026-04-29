'use client';

import React, { useEffect, useState } from 'react';
import { CheckCircle, AlertCircle, Loader2, Sparkles, Download } from 'lucide-react';
import { quietInvoke, safeInvoke } from '@/lib/safeInvoke';

/**
 * Panel informativo (read-only) sobre la IA local activa.
 * NO requiere Ollama. Reporta estado del runtime embebido (`llama-helper` +
 * modelo GGUF descargado durante el wizard).
 */
export function AIInfoPanel() {
  const [status, setStatus] = useState<'checking' | 'ready' | 'missing'>('checking');
  const [modelName, setModelName] = useState<string>('gemma3:4b');
  const [downloading, setDownloading] = useState<boolean>(false);

  useEffect(() => {
    let cancelled = false;
    const check = async () => {
      const ready = await quietInvoke<boolean>('builtin_ai_is_model_ready', { modelName });
      if (cancelled) return;
      setStatus(ready ? 'ready' : 'missing');
    };
    check();
    const id = setInterval(check, 30_000);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, [modelName]);

  const handleDownload = async () => {
    setDownloading(true);
    await safeInvoke(
      'builtin_ai_download_model',
      { modelName },
      'No se pudo iniciar la descarga. Verifica tu conexión a internet.',
    );
    setDownloading(false);
    // El estado se actualiza por el polling del effect.
  };

  const StatusIcon = status === 'checking' ? Loader2 : status === 'ready' ? CheckCircle : AlertCircle;
  const statusColor = status === 'checking' ? 'text-gray-400' : status === 'ready' ? 'text-emerald-400' : 'text-amber-400';
  const statusText =
    status === 'checking' ? 'Verificando…' : status === 'ready' ? 'Listo' : 'Falta descargar';

  return (
    <div className="space-y-6 text-gray-100">
      <div className="rounded-xl border border-white/10 bg-white/5 p-6">
        <div className="flex items-start gap-4">
          <div className="rounded-full p-3 bg-[#485df4]/20 border border-[#485df4]/40">
            <Sparkles className="w-6 h-6 text-[#a8b3ff]" />
          </div>
          <div className="flex-1">
            <div className="flex items-center gap-2">
              <h3 className="text-base font-semibold text-gray-50">IA Local Embebida</h3>
              <span className="flex items-center gap-1 text-xs">
                <StatusIcon className={`w-3.5 h-3.5 ${statusColor} ${status === 'checking' ? 'animate-spin' : ''}`} />
                <span className={statusColor}>{statusText}</span>
              </span>
            </div>
            <p className="text-sm text-gray-300 mt-2 leading-relaxed">
              Maity usa un motor de IA <strong className="text-gray-100">100% local</strong> que viene
              empaquetado con la app. No necesita Ollama, internet, ni servidores externos. Las
              conversaciones nunca salen de tu computadora.
            </p>
          </div>
        </div>
      </div>

      {status === 'missing' && (
        <div className="rounded-lg border border-amber-500/40 bg-amber-500/10 p-4 space-y-3">
          <div className="flex items-start gap-2 text-sm">
            <AlertCircle className="w-4 h-4 text-amber-400 mt-0.5 flex-shrink-0" />
            <div className="flex-1">
              <div className="font-semibold text-amber-100">Falta descargar el modelo de IA</div>
              <div className="text-amber-100/80 text-xs leading-relaxed mt-1">
                El modelo Gemma 3 4B (~2.4 GB) se descarga una sola vez. Después funciona sin
                internet.
              </div>
            </div>
          </div>
          <button
            onClick={handleDownload}
            disabled={downloading}
            className="w-full px-4 py-2 rounded-md bg-amber-500 hover:bg-amber-600 text-white text-sm font-medium flex items-center justify-center gap-2 disabled:opacity-50"
          >
            <Download className="w-4 h-4" />
            {downloading ? 'Iniciando descarga…' : 'Descargar modelo ahora'}
          </button>
        </div>
      )}

      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        <div className="rounded-lg border border-white/10 bg-white/5 p-4">
          <div className="text-[10px] uppercase tracking-wider text-gray-400 mb-1">Tips en vivo</div>
          <div className="text-sm font-semibold text-gray-50">Gemma 3 4B</div>
          <div className="text-xs text-gray-400 mt-1">Sugerencias rápidas mientras hablas.</div>
        </div>
        <div className="rounded-lg border border-white/10 bg-white/5 p-4">
          <div className="text-[10px] uppercase tracking-wider text-gray-400 mb-1">Evaluación post-meeting</div>
          <div className="text-sm font-semibold text-gray-50">Gemma 3 4B</div>
          <div className="text-xs text-gray-400 mt-1">Análisis profundo al terminar la reunión.</div>
        </div>
        <div className="rounded-lg border border-white/10 bg-white/5 p-4">
          <div className="text-[10px] uppercase tracking-wider text-gray-400 mb-1">Transcripción</div>
          <div className="text-sm font-semibold text-gray-50">Parakeet TDT</div>
          <div className="text-xs text-gray-400 mt-1">3.45% WER en español. Sin internet.</div>
        </div>
        <div className="rounded-lg border border-white/10 bg-white/5 p-4">
          <div className="text-[10px] uppercase tracking-wider text-gray-400 mb-1">Runtime</div>
          <div className="text-sm font-semibold text-gray-50">llama.cpp (CPU)</div>
          <div className="text-xs text-gray-400 mt-1">Optimizado para 4 GB RAM sin GPU.</div>
        </div>
      </div>
    </div>
  );
}

export default AIInfoPanel;
