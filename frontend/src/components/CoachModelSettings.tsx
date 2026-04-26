'use client';

import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { Sparkles, RefreshCw, CheckCircle2, AlertCircle, Loader2, MessageSquare, BarChart3, Mic } from 'lucide-react';
import { useCoach } from '@/contexts/CoachContext';
import { toast } from 'sonner';

interface OllamaModel {
  name: string;
  id: string;
  size: string;
  modified: string;
}

interface CoachModelsConfig {
  tips_model: string;
  evaluation_model: string;
  chat_model: string;
}

type Purpose = 'tips' | 'evaluation' | 'chat';

const PURPOSE_META: Record<Purpose, { label: string; description: string; icon: React.ReactNode; suggested: string[] }> = {
  tips: {
    label: 'Tips en tiempo real',
    description: 'Modelo que genera sugerencias durante la reunión. Prioriza velocidad.',
    icon: <Mic className="w-4 h-4" />,
    suggested: ['phi3.5:latest', 'phi3.5:3.8b-mini-instruct-q4_K_M', 'gemma3:4b'],
  },
  evaluation: {
    label: 'Evaluación post-meeting',
    description: 'Modelo que produce análisis profundo (radar, gauge, recomendaciones). Prioriza calidad.',
    icon: <BarChart3 className="w-4 h-4" />,
    suggested: ['gemma3:4b', 'gemma4:e4b', 'qwen3:8b'],
  },
  chat: {
    label: 'Chat con reuniones',
    description: 'Modelo que responde preguntas sobre reuniones grabadas con citas timestamps.',
    icon: <MessageSquare className="w-4 h-4" />,
    suggested: ['gemma3:4b', 'gemma4:e4b', 'qwen3:8b'],
  },
};

const RECOMMENDED: Record<string, { tag: string; reason: string }> = {
  'phi3.5:3.8b-mini-instruct-q4_K_M': { tag: 'Recomendado', reason: 'MIT, ~2.3 GB, balance velocidad/calidad' },
  'gemma3:4b': { tag: 'Universal', reason: 'Apache 2.0, ~3 GB, corre en laptops 8GB RAM' },
  'gemma4:e4b': { tag: 'Calidad alta', reason: 'Apache 2.0, ~7 GB, requiere 16GB RAM' },
  'phi3.5:latest': { tag: 'Rápido', reason: 'Latencias <2s en CPU moderno' },
  'qwen3:8b': { tag: 'Multilingüe', reason: 'Mejor español que phi/gemma' },
  'nomic-embed-text': { tag: 'Embeddings', reason: 'Solo búsqueda semántica chat (no chat conversacional)' },
};

function ModelSelectorRow({
  model,
  isActive,
  isSaving,
  onSelect,
}: {
  model: OllamaModel;
  isActive: boolean;
  isSaving: boolean;
  onSelect: () => void;
}) {
  const meta = RECOMMENDED[model.name];
  return (
    <button
      type="button"
      onClick={onSelect}
      disabled={isSaving}
      className={`w-full flex items-center gap-3 px-4 py-2.5 text-left transition ${
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
          <span className="font-mono text-sm text-gray-900 dark:text-gray-100 truncate">{model.name}</span>
          {meta && (
            <span className="text-[10px] font-semibold px-1.5 py-0.5 rounded bg-emerald-100 dark:bg-emerald-900/40 text-emerald-700 dark:text-emerald-300">
              {meta.tag}
            </span>
          )}
        </div>
        <div className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
          {meta?.reason ?? `${model.size} · modificado ${model.modified}`}
        </div>
      </div>
    </button>
  );
}

function PurposeSection({
  purpose,
  models,
  current,
  onChange,
}: {
  purpose: Purpose;
  models: OllamaModel[];
  current: string;
  onChange: (model: string) => Promise<void>;
}) {
  const [saving, setSaving] = useState<string | null>(null);
  const meta = PURPOSE_META[purpose];

  const handleSelect = async (name: string) => {
    if (name === current) return;
    setSaving(name);
    try {
      await onChange(name);
      toast.success(`${meta.label}: ${name}`);
    } catch (e) {
      toast.error(`No se pudo cambiar modelo: ${e}`);
    } finally {
      setSaving(null);
    }
  };

  return (
    <div className="rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900 overflow-hidden">
      <div className="px-4 py-2.5 bg-gray-50 dark:bg-gray-800/50 border-b border-gray-200 dark:border-gray-700">
        <div className="flex items-center gap-2">
          <span className="text-blue-600 dark:text-blue-300">{meta.icon}</span>
          <span className="text-sm font-semibold text-gray-900 dark:text-gray-100">{meta.label}</span>
          <span className="ml-auto text-xs text-gray-500 dark:text-gray-400">
            Activo: <code className="text-blue-600 dark:text-blue-400">{current || '(no configurado)'}</code>
          </span>
        </div>
        <div className="text-xs text-gray-500 dark:text-gray-400 mt-1">{meta.description}</div>
      </div>
      <div className="divide-y divide-gray-200 dark:divide-gray-800">
        {models.map((m) => (
          <ModelSelectorRow
            key={`${purpose}-${m.id}`}
            model={m}
            isActive={m.name === current}
            isSaving={saving === m.name}
            onSelect={() => handleSelect(m.name)}
          />
        ))}
      </div>
    </div>
  );
}

/**
 * CoachModelSettings — selector de 3 modelos Ollama para el copiloto IA.
 * Cada propósito (tips/evaluation/chat) puede tener un modelo distinto.
 * Provider fijo en Ollama por privacidad.
 */
export function CoachModelSettings() {
  const { setModel: setTipsModelInContext } = useCoach();
  const [models, setModels] = useState<OllamaModel[]>([]);
  const [config, setConfig] = useState<CoachModelsConfig>({
    tips_model: '',
    evaluation_model: '',
    chat_model: '',
  });
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchAll = async () => {
    setLoading(true);
    setError(null);
    try {
      const [list, cfg] = await Promise.all([
        invoke<OllamaModel[]>('get_ollama_models', { endpoint: null }),
        invoke<CoachModelsConfig>('coach_get_models'),
      ]);
      setModels(list);
      setConfig(cfg);
    } catch (e) {
      setError(typeof e === 'string' ? e : `${e}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchAll();
  }, []);

  const handleChange = async (purpose: Purpose, model: string) => {
    await invoke('coach_set_model_for_purpose', { purpose, model });
    setConfig(prev => ({
      ...prev,
      [`${purpose}_model`]: model,
    }));
    if (purpose === 'tips') {
      await setTipsModelInContext(model);
    }
  };

  return (
    <div className="space-y-6 max-w-3xl">
      <div className="flex items-start gap-3">
        <div className="p-2 rounded-lg bg-blue-100 dark:bg-blue-900/30 flex-shrink-0">
          <Sparkles className="w-5 h-5 text-blue-600 dark:text-blue-300" />
        </div>
        <div className="flex-1 min-w-0">
          <h2 className="text-lg font-semibold text-gray-900 dark:text-gray-100">Modelos del Coach IA</h2>
          <p className="text-sm text-gray-600 dark:text-gray-400 mt-0.5">
            El copiloto siempre usa Ollama local (privacidad). Configura un modelo distinto para cada propósito —
            puedes priorizar velocidad en tips y calidad en evaluación.
          </p>
        </div>
        <button
          type="button"
          onClick={fetchAll}
          disabled={loading}
          className="px-3 py-1.5 text-xs rounded-md border border-gray-300 dark:border-gray-700 hover:bg-gray-50 dark:hover:bg-gray-800 transition disabled:opacity-50"
          title="Recargar lista de modelos Ollama"
        >
          <RefreshCw className={`w-3.5 h-3.5 inline-block mr-1 ${loading ? 'animate-spin' : ''}`} />
          Refrescar
        </button>
      </div>

      {error && (
        <div className="px-4 py-6 flex items-center gap-3 text-sm text-amber-700 dark:text-amber-300 bg-amber-50 dark:bg-amber-900/20 rounded-lg">
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
          No tienes modelos Ollama instalados. Descarga uno con: <code>ollama pull gemma3:4b</code>
        </div>
      )}

      {models.length > 0 && (
        <>
          <PurposeSection
            purpose="tips"
            models={models}
            current={config.tips_model}
            onChange={(m) => handleChange('tips', m)}
          />
          <PurposeSection
            purpose="evaluation"
            models={models}
            current={config.evaluation_model}
            onChange={(m) => handleChange('evaluation', m)}
          />
          <PurposeSection
            purpose="chat"
            models={models}
            current={config.chat_model}
            onChange={(m) => handleChange('chat', m)}
          />
        </>
      )}

      <div className="text-[11px] text-gray-500 dark:text-gray-400 leading-relaxed">
        <strong>Nota privacidad:</strong> el Coach IA solo permite Ollama local — no se envía
        contexto de la reunión a APIs externas (Claude, OpenAI, Groq). Para cambiar de provider
        de resumen LLM (sí soporta cloud), usa la pestaña <em>Resumen</em>.
        <br />
        <strong>Chat con reuniones</strong> requiere también <code>nomic-embed-text</code> (~270 MB)
        para los embeddings vectoriales.
      </div>
    </div>
  );
}
