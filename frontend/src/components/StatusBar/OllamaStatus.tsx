'use client';

import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { AlertCircle, CheckCircle, Loader2, RotateCw, HelpCircle } from 'lucide-react';
import { cn } from '@/lib/utils';

interface OllamaModelEntry {
  name: string;
  digest: string;
  size: number;
  modified_at: string;
}

type Status = 'checking' | 'ready' | 'no-models' | 'offline';

interface OllamaStatusState {
  status: Status;
  models: OllamaModelEntry[];
  lastCheck: number;
  error?: string;
}

/**
 * OllamaStatus — Widget permanente en esquina superior-derecha
 * Verifica estado de Ollama cada 30s y muestra pill con indicador visual
 */
export function OllamaStatus() {
  const [state, setState] = useState<OllamaStatusState>({
    status: 'checking',
    models: [],
    lastCheck: Date.now(),
  });
  const [showPopover, setShowPopover] = useState(false);
  const popoverRef = useRef<HTMLDivElement>(null);
  const pollingRef = useRef<NodeJS.Timeout>();

  // Ejecuta verificación de Ollama
  const checkOllama = async () => {
    try {
      const models = await invoke('get_ollama_models', { endpoint: null }) as OllamaModelEntry[];

      // Busca modelo gemma3 (cualquier variante)
      const hasGemmaModel = models.some(m => m.name.startsWith('gemma3:'));

      setState(prev => ({
        ...prev,
        status: hasGemmaModel ? 'ready' : 'no-models',
        models,
        lastCheck: Date.now(),
      }));
    } catch (error) {
      setState(prev => ({
        ...prev,
        status: 'offline',
        models: [],
        lastCheck: Date.now(),
        error: error instanceof Error ? error.message : String(error),
      }));
    }
  };

  // Setup polling inicial + listener de evento
  useEffect(() => {
    checkOllama();
    pollingRef.current = setInterval(checkOllama, 30000);

    // Escucha evento de recheck (disparado por componentes de error)
    const handleVerifyRequest = () => {
      checkOllama();
    };
    window.addEventListener('verify-ollama-status', handleVerifyRequest);

    return () => {
      if (pollingRef.current) clearInterval(pollingRef.current);
      window.removeEventListener('verify-ollama-status', handleVerifyRequest);
    };
  }, []);

  // Cierra popover al hacer click fuera
  useEffect(() => {
    if (!showPopover) return;

    const handleClickOutside = (e: MouseEvent) => {
      if (popoverRef.current && !popoverRef.current.contains(e.target as Node)) {
        setShowPopover(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [showPopover]);

  const formatLastCheck = () => {
    const seconds = Math.floor((Date.now() - state.lastCheck) / 1000);
    if (seconds < 60) return `hace ${seconds}s`;
    const minutes = Math.floor(seconds / 60);
    return `hace ${minutes}m`;
  };

  const statusConfig = {
    checking: {
      icon: <Loader2 className="w-4 h-4 animate-spin text-gray-400" />,
      label: 'Verificando...',
      pill: 'bg-gray-500/20 border-gray-500/30 text-gray-300',
    },
    ready: {
      icon: <CheckCircle className="w-4 h-4 text-green-400 animate-pulse" />,
      label: `Ollama listo · ${state.models.length} modelos`,
      pill: 'bg-green-500/20 border-green-500/30 text-green-300',
    },
    'no-models': {
      icon: <AlertCircle className="w-4 h-4 text-amber-400 animate-pulse" style={{ animationDuration: '1.5s' }} />,
      label: 'Ollama corriendo · sin modelos',
      pill: 'bg-amber-500/20 border-amber-500/30 text-amber-300',
    },
    offline: {
      icon: <AlertCircle className="w-4 h-4 text-red-400" />,
      label: 'Ollama no detectado',
      pill: 'bg-red-500/20 border-red-500/30 text-red-300',
    },
  };

  const config = statusConfig[state.status];

  return (
    <div className="fixed top-3 right-3 z-30 font-sans">
      {/* Pill button */}
      <button
        onClick={() => setShowPopover(!showPopover)}
        className={cn(
          'flex items-center gap-2 px-3 py-1.5 rounded-full border text-xs font-medium',
          'transition-all duration-200 hover:shadow-md',
          'backdrop-blur-sm',
          config.pill
        )}
        aria-label="Ollama status"
      >
        {config.icon}
        <span className="hidden sm:inline">{config.label}</span>
      </button>

      {/* Popover */}
      {showPopover && (
        <div
          ref={popoverRef}
          className={cn(
            'absolute top-full right-0 mt-2 w-72',
            'rounded-lg border border-gray-700/50 bg-gray-900/95 backdrop-blur-sm',
            'shadow-xl p-4 space-y-3',
            'text-xs text-gray-300 z-40'
          )}
        >
          {/* Header */}
          <div className="space-y-1">
            <div className="flex items-center justify-between">
              <div className="font-semibold text-gray-100">Estado de Ollama</div>
              <div className="text-gray-400">{formatLastCheck()}</div>
            </div>
            <div className={cn(
              'text-xs font-medium',
              state.status === 'ready' && 'text-green-300',
              state.status === 'no-models' && 'text-amber-300',
              state.status === 'offline' && 'text-red-300',
              state.status === 'checking' && 'text-gray-400'
            )}>
              {config.label}
            </div>
          </div>

          {/* Modelos instalados */}
          {state.models.length > 0 && (
            <div className="space-y-2 pt-2 border-t border-gray-700/30">
              <div className="font-medium text-gray-200">Modelos instalados</div>
              <div className="space-y-1 max-h-24 overflow-y-auto">
                {state.models.slice(0, 5).map(m => (
                  <div key={m.name} className="text-xs text-gray-400">
                    {m.name}
                  </div>
                ))}
              </div>
              {state.models.length > 5 && (
                <div className="text-xs text-gray-500">
                  +{state.models.length - 5} más
                </div>
              )}
            </div>
          )}

          {/* Acciones */}
          <div className="flex gap-2 pt-2 border-t border-gray-700/30">
            <button
              onClick={() => {
                checkOllama();
                // No cierra el popover, permite ver resultado
              }}
              className={cn(
                'flex-1 flex items-center justify-center gap-1 px-2 py-1.5',
                'rounded bg-gray-800/50 hover:bg-gray-700/50 border border-gray-600/30',
                'text-xs font-medium text-gray-300 transition-colors'
              )}
            >
              <RotateCw className="w-3 h-3" />
              Verificar ahora
            </button>
          </div>

          {/* Help section si Ollama está offline */}
          {state.status === 'offline' && (
            <div className="space-y-2 pt-2 border-t border-gray-700/30">
              <div className="flex items-start gap-2 text-xs text-gray-300">
                <HelpCircle className="w-3 h-3 mt-0.5 flex-shrink-0 text-amber-400" />
                <div className="space-y-1.5 flex-1">
                  <div className="font-medium">¿Cómo arreglar?</div>
                  <ol className="list-decimal list-inside space-y-1 text-gray-400">
                    <li>Descarga Ollama: ollama.ai</li>
                    <li>Abre terminal: <code className="bg-gray-800 px-1 rounded text-gray-300">ollama serve</code></li>
                    <li>Reinicia la app</li>
                  </ol>
                </div>
              </div>
            </div>
          )}

          {/* Help section si no hay modelos */}
          {state.status === 'no-models' && (
            <div className="space-y-2 pt-2 border-t border-gray-700/30">
              <button
                onClick={() => {
                  window.dispatchEvent(new CustomEvent('open-settings', { detail: { tab: 'models' } }));
                  setShowPopover(false);
                }}
                className={cn(
                  'w-full px-2 py-1.5 rounded',
                  'bg-amber-600/20 hover:bg-amber-600/30 border border-amber-600/50',
                  'text-xs font-medium text-amber-200 transition-colors'
                )}
              >
                Descargar gemma3:4b
              </button>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
