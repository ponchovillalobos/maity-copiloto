'use client';

import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Sparkles, RefreshCw, CheckCircle2, AlertCircle, Loader2 } from 'lucide-react';
import { useCoach } from '@/contexts/CoachContext';
import { toast } from 'sonner';

interface OllamaModel {
  name: string;
  id: string;
  size: string;
  modified: string;
}

const RECOMMENDED: Record<string, { tag: string; reason: string }> = {
  'phi3.5:3.8b-mini-instruct-q4_K_M': { tag: 'Recomendado', reason: 'MIT, ~2.3 GB, balance velocidad/calidad' },
  'gemma4:e4b': { tag: 'Calidad alta', reason: 'Apache 2.0, abril 2026' },
  'phi3.5:latest': { tag: 'Rápido', reason: 'Latencias <2s en CPU moderno' },
  'qwen3:8b': { tag: 'Multilingüe', reason: 'Mejor español que phi/gemma' },
};

/**
 * CoachModelSettings — selector de modelo Ollama para el copiloto IA (Wave C1).
 *
 * Inspirado en Director (LLM provider abstraction): el provider Coach está
 * fijado en Ollama por privacidad, pero el usuario puede elegir cualquier
 * modelo que tenga descargado localmente.
 */
export function CoachModelSettings() {
  const { model: currentModel, setModel } = useCoach();
  const [models, setModels] = useState<OllamaModel[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState<string | null>(null);

  const fetchModels = async () => {
    setLoading(true);
    setError(null);
    try {
      const list = await invoke<OllamaModel[]>('get_ollama_models', { endpoint: null });
      setModels(list);
    } catch (e) {
      setError(typeof e === 'string' ? e : `${e}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchModels();
  }, []);

  const handleSelect = async (name: string) => {
    if (name === currentModel) return;
    setSaving(name);
    try {
      await setModel(name);
      toast.success(`Coach IA usará: ${name}`);
    } catch (e) {
      toast.error(`No se pudo cambiar modelo: ${e}`);
    } finally {
      setSaving(null);
    }
  };

  return (
    <div className="space-y-6 max-w-3xl">
      <div className="flex items-start gap-3">
        <div className="p-2 rounded-lg bg-blue-100 dark:bg-blue-900/30 flex-shrink-0">
          <Sparkles className="w-5 h-5 text-blue-600 dark:text-blue-300" />
        </div>
        <div className="flex-1 min-w-0">
          <h2 className="text-lg font-semibold text-gray-900 dark:text-gray-100">Modelo del Coach IA</h2>
          <p className="text-sm text-gray-600 dark:text-gray-400 mt-0.5">
            El copiloto siempre usa Ollama local (privacidad). Elige qué modelo usar.
            Modelos pequeños son más rápidos; modelos grandes dan tips más sutiles.
          </p>
        </div>
        <button
          type="button"
          onClick={fetchModels}
          disabled={loading}
          className="px-3 py-1.5 text-xs rounded-md border border-gray-300 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800 transition disabled:opacity-50"
          title="Recargar lista de modelos Ollama"
        >
          <RefreshCw className={`w-3.5 h-3.5 inline-block mr-1 ${loading ? 'animate-spin' : ''}`} />
          Refrescar
        </button>
      </div>

      <div className="rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900 overflow-hidden">
        <div className="px-4 py-2.5 bg-gray-50 dark:bg-gray-800/50 border-b border-gray-200 dark:border-gray-700">
          <div className="text-xs font-medium text-gray-700 dark:text-gray-300">
            Modelo activo: <code className="text-blue-600 dark:text-blue-400">{currentModel || '(no configurado)'}</code>
          </div>
        </div>

        {error && (
          <div className="px-4 py-6 flex items-center gap-3 text-sm text-amber-700 dark:text-amber-300 bg-amber-50 dark:bg-amber-900/20">
            <AlertCircle className="w-5 h-5 flex-shrink-0" />
            <div>
              <div className="font-medium">No se pudieron cargar los modelos</div>
              <div className="text-xs opacity-80 mt-0.5">{error}</div>
              <div className="text-xs mt-1">Verifica que Ollama esté corriendo: <code>ollama serve</code></div>
            </div>
          </div>
        )}

        {!error && loading && models.length === 0 && (
          <div className="px-4 py-8 flex items-center justify-center gap-2 text-sm text-gray-500">
            <Loader2 className="w-4 h-4 animate-spin" />
            Consultando Ollama…
          </div>
        )}

        {!error && !loading && models.length === 0 && (
          <div className="px-4 py-6 text-sm text-gray-600 dark:text-gray-400">
            No tienes modelos Ollama instalados. Descarga uno con: <code>ollama pull phi3.5</code>
          </div>
        )}

        {models.length > 0 && (
          <div className="divide-y divide-gray-200 dark:divide-gray-800">
            {models.map((m) => {
              const isActive = m.name === currentModel;
              const meta = RECOMMENDED[m.name];
              const isSaving = saving === m.name;
              return (
                <button
                  key={m.id}
                  type="button"
                  onClick={() => handleSelect(m.name)}
                  disabled={isSaving}
                  className={`w-full flex items-center gap-3 px-4 py-3 text-left transition ${
                    isActive
                      ? 'bg-blue-50 dark:bg-blue-950/30'
                      : 'hover:bg-gray-50 dark:hover:bg-gray-800/40'
                  }`}
                >
                  <div className="flex-shrink-0 w-5">
                    {isSaving ? (
                      <Loader2 className="w-4 h-4 animate-spin text-blue-500" />
                    ) : isActive ? (
                      <CheckCircle2 className="w-5 h-5 text-blue-500" />
                    ) : (
                      <span className="block w-4 h-4 rounded-full border-2 border-gray-300 dark:border-gray-600" />
                    )}
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 flex-wrap">
                      <span className="font-mono text-sm text-gray-900 dark:text-gray-100 truncate">
                        {m.name}
                      </span>
                      {meta && (
                        <span className="text-[10px] font-semibold px-1.5 py-0.5 rounded bg-emerald-100 dark:bg-emerald-900/40 text-emerald-700 dark:text-emerald-300">
                          {meta.tag}
                        </span>
                      )}
                    </div>
                    <div className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
                      {meta?.reason ?? `${m.size} · modificado ${m.modified}`}
                    </div>
                  </div>
                </button>
              );
            })}
          </div>
        )}
      </div>

      <div className="text-[11px] text-gray-500 dark:text-gray-400 leading-relaxed">
        <strong>Nota privacidad:</strong> el Coach IA solo permite Ollama local — no se envía
        contexto de la reunión a APIs externas (Claude, OpenAI, Groq). Para cambiar de provider
        de resumen LLM (sí soporta cloud), usa la pestaña <em>Resumen</em>.
      </div>
    </div>
  );
}
