'use client';

import React, { useEffect, useState, useCallback } from 'react';
import {
  Download,
  CheckCircle2,
  AlertCircle,
  Loader2,
  RefreshCw,
  HelpCircle,
  ChevronDown,
  ChevronUp,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import { OnboardingContainer } from '../OnboardingContainer';
import { useOnboarding } from '@/contexts/OnboardingContext';
import { listen } from '@tauri-apps/api/event';
import { toast } from 'sonner';
import { logger } from '@/lib/logger';
import { safeInvoke, quietInvoke } from '@/lib/safeInvoke';

/**
 * AutoModelSetupStep — Descarga el modelo de IA local embebido (NO requiere Ollama).
 *
 * Maity usa un runtime LLM nativo (`llama-helper` con `llama.cpp`) que se
 * empaqueta dentro del instalador. Este paso solo descarga el archivo GGUF
 * del modelo, que se guarda en `app_data/models/summary/`.
 *
 * Modelo recomendado: Gemma 3 4B Q4_K_M (~2.4 GB) — corre en CPU sin GPU
 * con 4 GB RAM.
 */

interface ModelOption {
  /** Identificador interno del modelo. */
  name: string;
  /** Tamaño legible para el usuario. */
  size: string;
  /** Nombre que ve el usuario. */
  displayName: string;
  /** Descripción corta de para qué sirve. */
  description: string;
  /** Recomendado por defecto. */
  recommended: boolean;
}

const MODEL_OPTIONS: ModelOption[] = [
  {
    name: 'gemma3:4b',
    size: '~2.4 GB',
    displayName: 'Gemma 3 4B (Recomendado)',
    description: 'Mejor calidad de tips y evaluación. Requiere 4 GB RAM libres.',
    recommended: true,
  },
  {
    name: 'gemma3:1b',
    size: '~1 GB',
    displayName: 'Gemma 3 1B (Ligero)',
    description: 'Más rápido y para equipos con poca RAM. Calidad básica.',
    recommended: false,
  },
];

interface ModelStatus {
  name: string;
  installed: boolean;
  downloading: boolean;
  /** Progreso 0-100. */
  progress: number;
  /** MB descargados (informativo). */
  downloadedMb: number;
  /** Tamaño total en MB (informativo). */
  totalMb: number;
  /** Timestamp del primer evento de progreso (para calcular ETA). */
  startedAt?: number;
}

/** Calcula ETA en formato "X min" basado en velocidad de descarga real. */
function formatEta(status: ModelStatus | undefined): string {
  if (!status || !status.startedAt || status.progress <= 0 || status.progress >= 100) return '';
  const elapsedSec = (Date.now() - status.startedAt) / 1000;
  if (elapsedSec < 5) return 'calculando…';
  const totalEstimatedSec = (elapsedSec / status.progress) * 100;
  const remainingSec = Math.max(0, totalEstimatedSec - elapsedSec);
  if (remainingSec < 60) return `~${Math.ceil(remainingSec)}s restantes`;
  const remainingMin = Math.ceil(remainingSec / 60);
  return `~${remainingMin} min restantes`;
}

interface DownloadProgressEvent {
  model: string;
  progress: number;
  downloaded_bytes?: number;
  total_bytes?: number;
}

interface ModelInfoResponse {
  name: string;
  status: 'NotDownloaded' | 'Downloading' | 'Downloaded' | { Downloaded: unknown } | { Downloading: unknown };
}

export function AutoModelSetupStep() {
  const { goNext } = useOnboarding();
  const [isMac, setIsMac] = useState(false);
  const [selectedModel, setSelectedModel] = useState<string>('gemma3:4b');
  const [modelStatuses, setModelStatuses] = useState<Record<string, ModelStatus>>(
    Object.fromEntries(
      MODEL_OPTIONS.map((m) => [
        m.name,
        { name: m.name, installed: false, downloading: false, progress: 0, downloadedMb: 0, totalMb: 0 },
      ]),
    ),
  );
  const [showWhyExplanation, setShowWhyExplanation] = useState(false);
  const [isCheckingStatus, setIsCheckingStatus] = useState(true);

  // Detect platform
  useEffect(() => {
    const checkPlatform = async () => {
      try {
        const { platform } = await import('@tauri-apps/plugin-os');
        setIsMac(platform() === 'macos');
      } catch {
        setIsMac(navigator.userAgent.includes('Mac'));
      }
    };
    checkPlatform();
  }, []);

  /** Verifica el estado actual de cada modelo en disco. */
  const checkAllModelStatuses = useCallback(async () => {
    setIsCheckingStatus(true);
    for (const opt of MODEL_OPTIONS) {
      const info = await quietInvoke<ModelInfoResponse | null>('builtin_ai_get_model_info', {
        modelName: opt.name,
      });
      const isDownloaded =
        info?.status === 'Downloaded' ||
        (typeof info?.status === 'object' && info?.status !== null && 'Downloaded' in info.status);
      setModelStatuses((prev) => ({
        ...prev,
        [opt.name]: {
          ...prev[opt.name],
          installed: !!isDownloaded,
        },
      }));
    }
    setIsCheckingStatus(false);
  }, []);

  useEffect(() => {
    checkAllModelStatuses();
  }, [checkAllModelStatuses]);

  // Listen download events emitted from backend.
  useEffect(() => {
    const unsubs: Array<() => void> = [];
    (async () => {
      const u1 = await listen<DownloadProgressEvent>('builtin-ai-download-progress', (event) => {
        const { model, progress, downloaded_bytes, total_bytes } = event.payload;
        setModelStatuses((prev) => ({
          ...prev,
          [model]: {
            ...prev[model],
            downloading: progress < 100,
            installed: progress >= 100,
            progress: Math.round(progress),
            downloadedMb: downloaded_bytes ? Math.round(downloaded_bytes / (1024 * 1024)) : prev[model]?.downloadedMb ?? 0,
            totalMb: total_bytes ? Math.round(total_bytes / (1024 * 1024)) : prev[model]?.totalMb ?? 0,
            startedAt: prev[model]?.startedAt ?? Date.now(),
          },
        }));
      });
      unsubs.push(u1);

      const u2 = await listen<{ model: string }>('builtin-ai-download-complete', (event) => {
        const { model } = event.payload;
        setModelStatuses((prev) => ({
          ...prev,
          [model]: { ...prev[model], installed: true, downloading: false, progress: 100 },
        }));
        toast.success('Modelo de IA descargado', {
          description: 'Listo para usar — no necesitas internet para evaluar reuniones.',
        });
      });
      unsubs.push(u2);

      const u3 = await listen<{ model: string; error: string }>('builtin-ai-download-error', (event) => {
        const { model, error } = event.payload;
        logger.error(`[AutoModelSetup] Download error for ${model}:`, error);
        setModelStatuses((prev) => ({
          ...prev,
          [model]: { ...prev[model], downloading: false, progress: 0 },
        }));
        toast.error('Error descargando modelo', { description: error });
      });
      unsubs.push(u3);
    })();
    return () => {
      unsubs.forEach((u) => u());
    };
  }, []);

  /** Inicia la descarga del modelo seleccionado. */
  const startDownload = useCallback(async (modelName: string) => {
    setModelStatuses((prev) => ({
      ...prev,
      [modelName]: { ...prev[modelName], downloading: true, progress: 0 },
    }));
    await safeInvoke(
      'builtin_ai_download_model',
      { modelName },
      'No se pudo iniciar la descarga del modelo. Verifica tu conexión a internet.',
    );
  }, []);

  const handleContinue = () => goNext();

  if (isCheckingStatus) {
    return (
      <OnboardingContainer
        title="Preparando IA local"
        description="Verificando si el modelo ya está en tu equipo..."
        step={2}
        totalSteps={isMac ? 6 : 5}
      >
        <div className="flex flex-col items-center justify-center space-y-6 py-8">
          <Loader2 className="w-10 h-10 text-blue-300 animate-spin" />
          <p className="text-center text-gray-200">Buscando modelo de IA local…</p>
        </div>
      </OnboardingContainer>
    );
  }

  const selected = MODEL_OPTIONS.find((m) => m.name === selectedModel)!;
  const status = modelStatuses[selectedModel];
  const isDownloaded = status?.installed ?? false;
  const isDownloading = status?.downloading ?? false;
  const canContinue = isDownloaded;

  return (
    <OnboardingContainer
      title="Inteligencia Artificial Local"
      description="Maity descarga UN modelo de IA que correrá 100% en tu equipo. Sin internet ni servidores externos."
      step={2}
      totalSteps={isMac ? 6 : 5}
    >
      <div className="space-y-6">
        {/* Selector de modelo */}
        <div className="space-y-3">
          {MODEL_OPTIONS.map((opt) => {
            const s = modelStatuses[opt.name];
            const isSelected = selectedModel === opt.name;
            return (
              <label
                key={opt.name}
                className={`flex items-start gap-3 p-4 rounded-lg border cursor-pointer transition-colors ${
                  isSelected
                    ? 'border-blue-400 bg-blue-500/10'
                    : 'border-white/10 bg-white/5 hover:border-white/20'
                }`}
              >
                <input
                  type="radio"
                  name="model"
                  value={opt.name}
                  checked={isSelected}
                  onChange={() => setSelectedModel(opt.name)}
                  className="mt-1"
                  aria-label={`Seleccionar ${opt.displayName}`}
                />
                <div className="flex-1">
                  <div className="flex items-center gap-2 flex-wrap">
                    <span className="font-semibold text-gray-50">{opt.displayName}</span>
                    {opt.recommended && (
                      <span className="text-[10px] uppercase tracking-wider px-2 py-0.5 rounded-full bg-emerald-500/20 text-emerald-300 border border-emerald-500/40">
                        Recomendado
                      </span>
                    )}
                    <span className="text-xs text-gray-400">{opt.size}</span>
                  </div>
                  <div className="text-xs text-gray-300 mt-1">{opt.description}</div>
                  {s?.installed && (
                    <div className="flex items-center gap-1.5 mt-2 text-xs text-emerald-400">
                      <CheckCircle2 className="w-3.5 h-3.5" /> Instalado
                    </div>
                  )}
                </div>
              </label>
            );
          })}
        </div>

        {/* Estado actual del seleccionado */}
        {isDownloading && status && (
          <div className="rounded-lg border border-blue-400/40 bg-blue-500/10 p-4 space-y-3">
            <div className="flex items-center justify-between text-sm">
              <span className="font-semibold text-gray-100 flex items-center gap-2">
                <Download className="w-4 h-4 animate-pulse" />
                Descargando {selected.displayName}…
              </span>
              <span className="tabular-nums text-gray-300">
                {status.progress}%
                {status.totalMb > 0 && ` · ${status.downloadedMb}/${status.totalMb} MB`}
              </span>
            </div>
            <div className="h-2 rounded-full bg-white/10 overflow-hidden">
              <div
                className="h-full bg-blue-400 transition-all duration-300"
                style={{ width: `${status.progress}%` }}
              />
            </div>
            <div className="flex items-center justify-between text-xs">
              <span className="text-gray-400">
                Esta descarga ocurre solo una vez. El modelo se guarda en tu equipo.
              </span>
              <span className="text-blue-300 font-medium">{formatEta(status)}</span>
            </div>
          </div>
        )}

        {/* Acciones */}
        <div className="space-y-3">
          {!isDownloaded && !isDownloading && (
            <Button
              onClick={() => startDownload(selectedModel)}
              className="w-full h-11 bg-blue-500 hover:bg-blue-600 text-white flex items-center justify-center gap-2"
              aria-label={`Descargar ${selected.displayName}`}
            >
              <Download className="w-4 h-4" />
              Descargar {selected.displayName}
            </Button>
          )}

          {isDownloaded && (
            <Button
              onClick={handleContinue}
              className="w-full h-11 bg-emerald-500 hover:bg-emerald-600 text-white flex items-center justify-center gap-2"
              aria-label="Continuar"
            >
              <CheckCircle2 className="w-4 h-4" />
              Continuar
            </Button>
          )}

          <Button
            onClick={checkAllModelStatuses}
            variant="outline"
            className="w-full h-9 text-xs flex items-center justify-center gap-2"
            aria-label="Verificar estado"
          >
            <RefreshCw className="w-3.5 h-3.5" />
            Verificar estado
          </Button>

          <Button
            onClick={handleContinue}
            variant="ghost"
            className="w-full h-9 text-xs text-gray-400 hover:text-white"
            aria-label="Saltar configuración"
          >
            Saltar (la app funcionará sin IA hasta que descargues el modelo)
          </Button>
        </div>

        {/* Por qué? */}
        <div className="rounded-lg border border-white/10 bg-white/5">
          <button
            onClick={() => setShowWhyExplanation(!showWhyExplanation)}
            className="w-full flex items-center justify-between p-3 text-xs text-gray-300 hover:text-white"
            aria-label="Mostrar por qué se necesita el modelo"
          >
            <span className="flex items-center gap-2">
              <HelpCircle className="w-3.5 h-3.5" />
              ¿Por qué necesito esto?
            </span>
            {showWhyExplanation ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
          </button>
          {showWhyExplanation && (
            <div className="px-3 pb-3 space-y-2 text-xs text-gray-300 leading-relaxed">
              <p>
                <strong className="text-gray-100">Privacidad total.</strong> El modelo corre en tu computadora.
                Tus reuniones nunca se envían a servidores externos.
              </p>
              <p>
                <strong className="text-gray-100">Sin dependencias.</strong> No necesitas instalar Ollama
                ni cualquier otro software adicional. Maity trae todo lo que necesita.
              </p>
              <p>
                <strong className="text-gray-100">Sin tarjeta de video.</strong> Funciona en CPU. Probado en
                laptops con 4 GB de RAM.
              </p>
              <p className="text-amber-300">
                <AlertCircle className="w-3 h-3 inline mr-1" />
                La primera descarga toma 5-10 minutos según tu conexión. Después no hay que repetirla.
              </p>
            </div>
          )}
        </div>
      </div>
    </OnboardingContainer>
  );
}
