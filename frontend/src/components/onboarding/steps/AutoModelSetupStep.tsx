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
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { toast } from 'sonner';
import { logger } from '@/lib/logger';

const REQUIRED_MODELS = [
  {
    name: 'gemma3:4b',
    size: '~3 GB',
    description: 'Coach IA para tips y evaluación en vivo',
    displayName: 'Gemma 3 4B (Coach IA)',
  },
  {
    name: 'nomic-embed-text',
    size: '~270 MB',
    description: 'Embeddings para búsqueda semántica',
    displayName: 'Nomic Embed (Búsqueda)',
  },
];

interface ModelStatus {
  name: string;
  installed: boolean;
  downloading: boolean;
  progress: number;
}

export function AutoModelSetupStep() {
  const { goNext } = useOnboarding();
  const [isMac, setIsMac] = useState(false);
  const [ollamaRunning, setOllamaRunning] = useState<boolean | null>(null);
  const [modelStatuses, setModelStatuses] = useState<ModelStatus[]>(
    REQUIRED_MODELS.map((m) => ({
      name: m.name,
      installed: false,
      downloading: false,
      progress: 0,
    }))
  );
  const [isCheckingOllama, setIsCheckingOllama] = useState(true);
  const [showWhyExplanation, setShowWhyExplanation] = useState(false);
  const [isInstallingAll, setIsInstallingAll] = useState(false);

  // Detect platform
  useEffect(() => {
    const checkPlatform = async () => {
      try {
        const { platform } = await import('@tauri-apps/plugin-os');
        setIsMac(platform() === 'macos');
      } catch (e) {
        setIsMac(navigator.userAgent.includes('Mac'));
      }
    };
    checkPlatform();
  }, []);

  // Check if Ollama is running and list installed models
  const checkOllamaStatus = useCallback(async () => {
    setIsCheckingOllama(true);
    try {
      logger.debug('[AutoModelSetupStep] Checking Ollama status');
      const models = await invoke<{ name: string }[]>('get_ollama_models');
      logger.debug('[AutoModelSetupStep] Ollama is running, found models:', models);

      // Check which of our required models are installed
      const installedModelNames = models.map((m) => m.name);
      setModelStatuses((prev) =>
        prev.map((status) => ({
          ...status,
          installed: installedModelNames.some((name) =>
            name.includes(status.name)
          ),
        }))
      );

      setOllamaRunning(true);
    } catch (error) {
      logger.debug('[AutoModelSetupStep] Ollama not running or error:', error);
      setOllamaRunning(false);
    } finally {
      setIsCheckingOllama(false);
    }
  }, []);

  // Initial check + interval polling
  useEffect(() => {
    checkOllamaStatus();
    const interval = setInterval(checkOllamaStatus, 5000); // Check every 5s
    return () => clearInterval(interval);
  }, [checkOllamaStatus]);

  // Set up event listeners for download progress
  useEffect(() => {
    const unsubscribers: (() => void)[] = [];

    const setupListeners = async () => {
      try {
        // Progress listener
        const unlistenProgress = await listen<{
          modelName: string;
          progress: number;
        }>('ollama-model-download-progress', (event) => {
          const { modelName, progress } = event.payload;
          logger.debug(
            `[AutoModelSetupStep] Progress for ${modelName}: ${progress}%`
          );

          setModelStatuses((prev) =>
            prev.map((status) =>
              status.name === modelName
                ? { ...status, progress, downloading: true }
                : status
            )
          );
        });
        unsubscribers.push(unlistenProgress);

        // Completion listener
        const unlistenComplete = await listen<{ modelName: string }>(
          'ollama-model-download-complete',
          (event) => {
            const { modelName } = event.payload;
            logger.debug(`[AutoModelSetupStep] Download complete: ${modelName}`);

            setModelStatuses((prev) =>
              prev.map((status) =>
                status.name === modelName
                  ? {
                      ...status,
                      installed: true,
                      downloading: false,
                      progress: 100,
                    }
                  : status
              )
            );

            toast.success(`¡${modelName} descargado!`, {
              description: 'El modelo está listo para usar',
            });
          }
        );
        unsubscribers.push(unlistenComplete);

        // Error listener
        const unlistenError = await listen<{
          modelName: string;
          error: string;
        }>('ollama-model-download-error', (event) => {
          const { modelName, error } = event.payload;
          logger.error(
            `[AutoModelSetupStep] Download error for ${modelName}:`,
            error
          );

          setModelStatuses((prev) =>
            prev.map((status) =>
              status.name === modelName
                ? { ...status, downloading: false, progress: 0 }
                : status
            )
          );

          toast.error(`Error descargando ${modelName}`, {
            description: error,
          });
        });
        unsubscribers.push(unlistenError);
      } catch (e) {
        logger.error('[AutoModelSetupStep] Failed to setup listeners:', e);
      }
    };

    setupListeners();

    return () => {
      unsubscribers.forEach((fn) => fn());
    };
  }, []);

  // Download a single model
  const downloadModel = useCallback(async (modelName: string) => {
    try {
      logger.info(`[AutoModelSetupStep] Starting download of ${modelName}`);
      await invoke('pull_ollama_model', { modelName });
    } catch (error) {
      logger.error(`[AutoModelSetupStep] Failed to start download:`, error);
      toast.error(`Error descargando ${modelName}`, {
        description: String(error),
      });
    }
  }, []);

  // Download all missing models sequentially
  const downloadAllModels = useCallback(async () => {
    setIsInstallingAll(true);
    try {
      const toDownload = modelStatuses.filter((m) => !m.installed);

      for (const model of toDownload) {
        await downloadModel(model.name);
        // Wait a bit between starting downloads
        await new Promise((r) => setTimeout(r, 500));
      }
    } catch (error) {
      logger.error('[AutoModelSetupStep] Error in batch download:', error);
    } finally {
      setIsInstallingAll(false);
    }
  }, [modelStatuses, downloadModel]);

  // Handle continue
  const handleContinue = async () => {
    goNext();
  };

  // If still checking Ollama status
  if (isCheckingOllama) {
    return (
      <OnboardingContainer
        title="Preparando Configuración"
        description="Verificando si Ollama está instalado en tu equipo..."
        step={2}
        totalSteps={isMac ? 6 : 5}
      >
        <div className="flex flex-col items-center justify-center space-y-6 py-8">
          <div className="w-12 h-12 rounded-full bg-blue-100 dark:bg-blue-900 flex items-center justify-center animate-spin">
            <Loader2 className="w-6 h-6 text-[#3a4ac3]" />
          </div>
          <p className="text-center text-[#4a4a4c] dark:text-gray-300">
            Buscando Ollama...
          </p>
        </div>
      </OnboardingContainer>
    );
  }

  // If Ollama is not running
  if (!ollamaRunning) {
    return (
      <OnboardingContainer
        title="Ollama No Está Activo"
        description="Maity necesita Ollama para el Coach IA y búsqueda semántica"
        step={2}
        totalSteps={isMac ? 6 : 5}
      >
        <div className="flex flex-col items-center space-y-6">
          <div className="w-16 h-16 rounded-full bg-amber-100 dark:bg-amber-900 flex items-center justify-center">
            <AlertCircle className="w-8 h-8 text-amber-600 dark:text-amber-400" />
          </div>

          <div className="w-full max-w-md bg-amber-50 dark:bg-amber-950 border border-amber-200 dark:border-amber-800 rounded-lg p-4 space-y-3">
            <p className="text-sm text-amber-900 dark:text-amber-100">
              No pudimos conectar con Ollama en <code>http://localhost:11434</code>.
              Asegúrate que Ollama esté instalado y ejecutándose.
            </p>
            <p className="text-xs text-amber-800 dark:text-amber-200">
              Descárgalo desde <a
                href="https://ollama.ai"
                target="_blank"
                rel="noopener noreferrer"
                className="underline font-semibold hover:text-amber-700 dark:hover:text-amber-300"
              >
                ollama.ai
              </a>
            </p>
          </div>

          <div className="w-full max-w-xs space-y-3">
            <Button
              onClick={checkOllamaStatus}
              className="w-full h-11 bg-[#000000] hover:bg-[#1a1a1a] text-white flex items-center justify-center gap-2"
            >
              <RefreshCw className="w-4 h-4" />
              Verificar de Nuevo
            </Button>

            <Button
              onClick={handleContinue}
              variant="outline"
              className="w-full h-11"
            >
              Saltar y Configurar Después
            </Button>
          </div>

          <p className="text-xs text-center text-[#6a6a6d] dark:text-gray-500">
            Nota: Algunas funciones (Coach IA, búsqueda) no estarán disponibles sin Ollama.
          </p>
        </div>
      </OnboardingContainer>
    );
  }

  // Ollama is running - show model setup
  const allInstalled = modelStatuses.every((m) => m.installed);
  const anyDownloading = modelStatuses.some((m) => m.downloading);

  return (
    <OnboardingContainer
      title="Descargar Modelos Locales"
      description="Maity usa IA local para respetar tu privacidad. Estos modelos se descargan solo una vez."
      step={2}
      totalSteps={isMac ? 6 : 5}
    >
      <div className="flex flex-col items-center space-y-6">
        {/* Ollama Status */}
        <div className="w-full max-w-md bg-green-50 dark:bg-green-950 border border-green-200 dark:border-green-800 rounded-lg p-4 flex items-center gap-3">
          <CheckCircle2 className="w-5 h-5 text-green-600 dark:text-green-400 flex-shrink-0" />
          <p className="text-sm text-green-800 dark:text-green-100">
            ✓ Ollama está instalado y funcionando
          </p>
        </div>

        {/* Model Cards */}
        <div className="w-full max-w-md space-y-3">
          {modelStatuses.map((status) => {
            const modelConfig = REQUIRED_MODELS.find(
              (m) => m.name === status.name
            )!;

            return (
              <div
                key={status.name}
                className="bg-white dark:bg-gray-800 border border-[#e7e7e9] dark:border-gray-700 rounded-lg p-4 space-y-2"
              >
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-3 flex-1">
                    {status.installed && (
                      <CheckCircle2 className="w-5 h-5 text-green-600 dark:text-green-400 flex-shrink-0" />
                    )}
                    {status.downloading && (
                      <Loader2 className="w-5 h-5 text-[#3a4ac3] animate-spin flex-shrink-0" />
                    )}
                    {!status.installed && !status.downloading && (
                      <Download className="w-5 h-5 text-[#6a6a6d] dark:text-gray-400 flex-shrink-0" />
                    )}

                    <div className="flex-1">
                      <p className="font-medium text-[#000000] dark:text-white text-sm">
                        {modelConfig.displayName}
                      </p>
                      <p className="text-xs text-[#6a6a6d] dark:text-gray-400">
                        {modelConfig.size}
                      </p>
                    </div>
                  </div>

                  {status.installed && (
                    <span className="text-xs font-semibold text-green-600 dark:text-green-400 bg-green-100 dark:bg-green-900 px-2 py-1 rounded">
                      Listo
                    </span>
                  )}
                </div>

                {/* Progress Bar */}
                {status.downloading && (
                  <div className="space-y-1">
                    <div className="w-full bg-[#e7e7e9] dark:bg-gray-700 rounded-full h-2 overflow-hidden">
                      <div
                        className="bg-[#3a4ac3] h-full transition-all duration-300"
                        style={{ width: `${status.progress}%` }}
                      />
                    </div>
                    <p className="text-xs text-[#6a6a6d] dark:text-gray-400 text-right">
                      {status.progress}%
                    </p>
                  </div>
                )}

                {/* Download Button */}
                {!status.installed && !status.downloading && (
                  <Button
                    onClick={() => downloadModel(status.name)}
                    size="sm"
                    variant="outline"
                    className="w-full h-9 text-xs"
                  >
                    <Download className="w-3 h-3 mr-1" />
                    Descargar ({modelConfig.size})
                  </Button>
                )}
              </div>
            );
          })}
        </div>

        {/* Why Explanation */}
        <div className="w-full max-w-md border border-[#e7e7e9] dark:border-gray-700 rounded-lg overflow-hidden">
          <button
            onClick={() => setShowWhyExplanation(!showWhyExplanation)}
            className="w-full px-4 py-3 flex items-center justify-between hover:bg-[#f5f5f6] dark:hover:bg-gray-700 transition-colors"
          >
            <span className="flex items-center gap-2 text-sm font-medium text-[#000000] dark:text-white">
              <HelpCircle className="w-4 h-4" />
              ¿Por qué necesito esto?
            </span>
            {showWhyExplanation ? (
              <ChevronUp className="w-4 h-4" />
            ) : (
              <ChevronDown className="w-4 h-4" />
            )}
          </button>

          {showWhyExplanation && (
            <div className="border-t border-[#e7e7e9] dark:border-gray-700 px-4 py-3 bg-[#f5f5f6] dark:bg-gray-800 space-y-2">
              <p className="text-xs text-[#4a4a4c] dark:text-gray-300">
                <strong>Privacidad:</strong> Maity usa IA local para que tu audio nunca
                salga de tu computadora. Ningún servicio en la nube, cero API keys.
              </p>
              <p className="text-xs text-[#4a4a4c] dark:text-gray-300">
                <strong>Modelos:</strong> Estos modelos pesan ~3.3 GB total y se descargan
                solo una vez. Después funcionan completamente sin conexión a internet.
              </p>
            </div>
          )}
        </div>

        {/* CTA Buttons */}
        <div className="w-full max-w-xs space-y-3">
          {!allInstalled && (
            <Button
              onClick={downloadAllModels}
              disabled={isInstallingAll || anyDownloading}
              className="w-full h-11 bg-[#000000] hover:bg-[#1a1a1a] text-white"
            >
              {isInstallingAll || anyDownloading ? (
                <>
                  <Loader2 className="w-4 h-4 mr-2 animate-spin" />
                  Descargando...
                </>
              ) : (
                <>
                  <Download className="w-4 h-4 mr-2" />
                  Instalar Todos
                </>
              )}
            </Button>
          )}

          <Button
            onClick={handleContinue}
            disabled={anyDownloading}
            className={allInstalled ? 'w-full h-11 bg-[#000000] hover:bg-[#1a1a1a] text-white' : 'w-full h-11'}
            variant={allInstalled ? 'default' : 'outline'}
          >
            {allInstalled ? '¡Vamos!' : 'Saltar y Configurar Después'}
          </Button>
        </div>

        {!allInstalled && (
          <p className="text-xs text-center text-[#6a6a6d] dark:text-gray-500">
            Puedes descargar los modelos después desde Configuración si lo prefieres.
          </p>
        )}
      </div>
    </OnboardingContainer>
  );
}
