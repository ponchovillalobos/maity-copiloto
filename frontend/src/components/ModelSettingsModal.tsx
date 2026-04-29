'use client';

import { useEffect, useState } from 'react';
import { Button } from '@/components/ui/button';
import { Sparkles, CheckCircle, AlertCircle, Download, Loader2 } from 'lucide-react';
import { quietInvoke, safeInvoke } from '@/lib/safeInvoke';

/**
 * ModelConfig — interface conservada por compatibilidad con código que la importa,
 * pero la app ya NO permite cambiar de modelo. Siempre usa el runtime local
 * embebido (`builtin-ai`) con `gemma3:4b`.
 *
 * Los campos cloud (groq, claude, openai, openrouter, custom-openai) se mantienen
 * en el type union por compatibilidad con tipos persistidos en disco; el código
 * activo solo lee `provider: 'builtin-ai'`.
 */
export interface ModelConfig {
  provider: 'ollama' | 'groq' | 'claude' | 'openai' | 'openrouter' | 'builtin-ai' | 'custom-openai';
  model: string;
  whisperModel: string;
  apiKey?: string | null;
  ollamaEndpoint?: string | null;
  customOpenAIDisplayName?: string | null;
  customOpenAIEndpoint?: string | null;
  customOpenAIModel?: string | null;
  customOpenAIApiKey?: string | null;
  maxTokens?: number | null;
  temperature?: number | null;
  topP?: number | null;
}

interface ModelSettingsModalProps {
  /** Si controlado externamente (overlay): visible/no. Si omitido, modal siempre visible. */
  showModelSettings?: boolean;
  setShowModelSettings?: (show: boolean) => void;
  /** Callback opcional (no-op en versión local — se mantiene por compatibilidad). */
  onSave?: (config: ModelConfig) => void | Promise<void>;
  /** Config previa (ignorada — la app usa builtin-ai/gemma3:4b siempre). */
  modelConfig?: ModelConfig;
  /** Setter de config (ignorado — sin selector). */
  setModelConfig?: (config: ModelConfig | ((prev: ModelConfig) => ModelConfig)) => void;
  /** Skip fetch inicial (ignorado — no hay fetch). */
  skipInitialFetch?: boolean;
}

const DEFAULT_MODEL = 'gemma3:4b';

/**
 * Modal simplificado para usuarios no-técnicos. La app ya no expone selector
 * de modelos; este modal solo informa el estado de la IA local y ofrece descarga
 * si falta el modelo.
 */
export function ModelSettingsModal({ showModelSettings, setShowModelSettings }: ModelSettingsModalProps) {
  // Si NO se pasan props de visibilidad (formato legacy embebido en SettingsModal),
  // mostramos siempre el contenido. Si se pasan, respetamos la visibilidad externa.
  const isControlled = typeof showModelSettings === 'boolean';
  const visible = isControlled ? showModelSettings : true;
  const [ready, setReady] = useState<boolean | null>(null);
  const [downloading, setDownloading] = useState(false);

  useEffect(() => {
    if (!visible) return;
    let cancelled = false;
    (async () => {
      const isReady = await quietInvoke<boolean>('builtin_ai_is_model_ready', { modelName: DEFAULT_MODEL });
      if (!cancelled) setReady(!!isReady);
    })();
    return () => { cancelled = true; };
  }, [visible]);

  if (!visible) return null;

  const handleDownload = async () => {
    setDownloading(true);
    await safeInvoke('builtin_ai_download_model', { modelName: DEFAULT_MODEL }, 'No se pudo iniciar la descarga.');
    setDownloading(false);
  };

  const handleClose = () => {
    if (setShowModelSettings) setShowModelSettings(false);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm" onClick={handleClose}>
      <div
        className="w-full max-w-md rounded-xl border border-white/10 bg-gray-900 p-6 shadow-2xl text-gray-100"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-start gap-3 mb-4">
          <div className="rounded-full p-2 bg-blue-500/20 border border-blue-500/40">
            <Sparkles className="w-5 h-5 text-blue-300" />
          </div>
          <div className="flex-1">
            <h2 className="text-lg font-semibold text-gray-50">IA Local de Maity</h2>
            <p className="text-xs text-gray-400 mt-1">
              Modelo Gemma 3 (4B) corriendo 100% en tu equipo. Sin internet, sin terceros.
            </p>
          </div>
        </div>

        <div className="rounded-lg border border-white/10 bg-white/5 p-4 mb-4">
          {ready === null ? (
            <div className="flex items-center gap-2 text-sm text-gray-400">
              <Loader2 className="w-4 h-4 animate-spin" /> Verificando modelo…
            </div>
          ) : ready ? (
            <div className="flex items-center gap-2 text-sm text-emerald-300">
              <CheckCircle className="w-4 h-4" /> Modelo descargado y listo para usar.
            </div>
          ) : (
            <div className="space-y-3">
              <div className="flex items-center gap-2 text-sm text-amber-300">
                <AlertCircle className="w-4 h-4" /> Modelo no descargado todavía.
              </div>
              <Button
                onClick={handleDownload}
                disabled={downloading}
                className="w-full bg-blue-500 hover:bg-blue-600 text-white flex items-center justify-center gap-2"
                aria-label="Descargar modelo de IA"
              >
                <Download className="w-4 h-4" />
                {downloading ? 'Iniciando…' : 'Descargar modelo (~2.4 GB)'}
              </Button>
              <p className="text-xs text-gray-400">
                Solo se descarga una vez. Después no se necesita internet para los tips ni la evaluación.
              </p>
            </div>
          )}
        </div>

        <div className="flex justify-end">
          <Button onClick={handleClose} variant="outline" className="border-white/15 text-gray-200" aria-label="Cerrar">
            Cerrar
          </Button>
        </div>
      </div>
    </div>
  );
}

export default ModelSettingsModal;
