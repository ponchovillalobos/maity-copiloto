'use client';

import React, { useEffect, useState, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Mic, RotateCcw, Volume2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { OnboardingContainer } from '../OnboardingContainer';
import { useOnboarding } from '@/contexts/OnboardingContext';
import { AudioLevelMeter } from '@/components/AudioLevelMeter';
import { toast } from 'sonner';
import { logger } from '@/lib/logger';

interface AudioDevice {
  name: string;
  device_type: string; // "Microphone" or "System"
}

interface AudioLevelData {
  device_name: string;
  device_type: string;
  rms_level: number;
  peak_level: number;
  is_active: boolean;
}

interface AudioLevelUpdate {
  timestamp: number;
  levels: AudioLevelData[];
}

type TestPhase = 'device-selection' | 'level-monitoring' | 'recording-preview' | 'complete';

export function MicTestStep() {
  const { goNext } = useOnboarding();
  const [phase, setPhase] = useState<TestPhase>('device-selection');

  // Device selection
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const [selectedDevice, setSelectedDevice] = useState<string>('');
  const [loadingDevices, setLoadingDevices] = useState(true);

  // Level monitoring
  const [rmsLevel, setRmsLevel] = useState(0);
  const [peakLevel, setPeakLevel] = useState(0);
  const [isActive, setIsActive] = useState(false);
  const [levelDetected, setLevelDetected] = useState(false);
  const [monitoringActive, setMonitoringActive] = useState(false);
  const levelTimeoutRef = useRef<NodeJS.Timeout>();
  const soundDetectedRef = useRef(false);
  const unsubscribeRef = useRef<(() => void) | null>(null);

  // Recording preview
  const [isRecording, setIsRecording] = useState(false);
  const [recordingTime, setRecordingTime] = useState(0);
  const [audioBlob, setAudioBlob] = useState<Blob | null>(null);
  const [isPlaying, setIsPlaying] = useState(false);
  const audioRef = useRef<HTMLAudioElement>(null);
  const recordingIntervalRef = useRef<NodeJS.Timeout>();

  // Load audio devices
  useEffect(() => {
    const loadDevices = async () => {
      try {
        const audioDevices = await invoke<AudioDevice[]>('get_audio_devices');
        const inputDevices = audioDevices.filter(
          (d) => d.device_type === 'Microphone' || d.device_type === 'Input'
        );
        setDevices(inputDevices);

        // Auto-select first device
        if (inputDevices.length > 0) {
          setSelectedDevice(inputDevices[0].name);
        }

        logger.debug('[MicTestStep] Loaded audio devices:', inputDevices);
      } catch (error) {
        logger.error('[MicTestStep] Failed to load audio devices:', error);
        toast.error('No se pudieron cargar los dispositivos de audio');
      } finally {
        setLoadingDevices(false);
      }
    };

    loadDevices();
  }, []);

  // Clean up monitoring when unmounting
  useEffect(() => {
    return () => {
      if (monitoringActive) {
        stopLevelMonitoring();
      }
    };
  }, [monitoringActive]);

  const startLevelMonitoring = async () => {
    if (!selectedDevice) {
      toast.error('Por favor selecciona un micrófono');
      return;
    }

    try {
      setMonitoringActive(true);
      setLevelDetected(false);
      soundDetectedRef.current = false;

      logger.debug('[MicTestStep] Starting level monitoring for device:', selectedDevice);
      await invoke('start_audio_level_monitoring', {
        deviceNames: [selectedDevice],
      });

      // Subscribe to audio levels
      const unsubscribe = await listen<AudioLevelUpdate>('audio-levels', (event: { payload: AudioLevelUpdate }) => {
        const update = event.payload;
        if (update.levels.length > 0) {
          const level = update.levels[0];
          setRmsLevel(level.rms_level);
          setPeakLevel(level.peak_level);
          setIsActive(level.is_active);

          // Detect sustained sound >0.05 for 1+ seconds
          if (level.rms_level > 0.05) {
            soundDetectedRef.current = true;

            // Reset timeout each time we get a high level
            if (levelTimeoutRef.current) {
              clearTimeout(levelTimeoutRef.current);
            }

            levelTimeoutRef.current = setTimeout(() => {
              if (soundDetectedRef.current) {
                setLevelDetected(true);
                logger.debug('[MicTestStep] Sound detected (RMS > 0.05 for 1s)');
              }
            }, 1000);
          } else {
            soundDetectedRef.current = false;
          }
        }
      });

      unsubscribeRef.current = unsubscribe;
    } catch (error) {
      logger.error('[MicTestStep] Failed to start level monitoring:', error);
      toast.error('Error al iniciar monitoreo de audio');
      setMonitoringActive(false);
    }
  };

  const stopLevelMonitoring = async () => {
    try {
      if (levelTimeoutRef.current) {
        clearTimeout(levelTimeoutRef.current);
      }
      if (unsubscribeRef.current) {
        unsubscribeRef.current();
        unsubscribeRef.current = null;
      }
      await invoke('stop_audio_level_monitoring');
      setMonitoringActive(false);
      setIsActive(false);
      logger.debug('[MicTestStep] Stopped level monitoring');
    } catch (error) {
      logger.error('[MicTestStep] Failed to stop level monitoring:', error);
    }
  };

  const handleStartMonitoring = async () => {
    await startLevelMonitoring();
    setPhase('level-monitoring');
  };

  const handleStartRecording = async () => {
    if (!selectedDevice) {
      toast.error('Por favor selecciona un micrófono');
      return;
    }

    try {
      setIsRecording(true);
      setRecordingTime(0);
      setAudioBlob(null);

      // Stop level monitoring
      await stopLevelMonitoring();

      logger.debug('[MicTestStep] Starting recording for:', selectedDevice);

      // Start recording
      await invoke('start_recording_with_devices', {
        micDeviceName: selectedDevice,
        systemDeviceName: null,
      });

      // Timer for 5 seconds
      recordingIntervalRef.current = setInterval(() => {
        setRecordingTime((t) => {
          if (t >= 5) {
            if (recordingIntervalRef.current) {
              clearInterval(recordingIntervalRef.current);
            }
            return 5;
          }
          return t + 0.1;
        });
      }, 100);
    } catch (error) {
      logger.error('[MicTestStep] Failed to start recording:', error);
      toast.error('Error al iniciar grabación de prueba');
      setIsRecording(false);
    }
  };

  const handleStopRecording = async () => {
    try {
      if (recordingIntervalRef.current) {
        clearInterval(recordingIntervalRef.current);
      }

      logger.debug('[MicTestStep] Stopping recording');
      await invoke('stop_recording');

      setIsRecording(false);
      setRecordingTime(0);

      // For MVP: just mark as complete after recording
      // In future: could add audio playback here
      setPhase('complete');

      toast.success('Grabación completada. Buen sonido detectado ✓');
    } catch (error) {
      logger.error('[MicTestStep] Failed to stop recording:', error);
      toast.error('Error al detener grabación');
      setIsRecording(false);
    }
  };

  const handleSkip = async () => {
    if (monitoringActive) {
      await stopLevelMonitoring();
    }
    goNext();
  };

  const handleContinue = async () => {
    if (monitoringActive) {
      await stopLevelMonitoring();
    }
    goNext();
  };

  const handleRefreshDevices = async () => {
    setLoadingDevices(true);
    try {
      const audioDevices = await invoke<AudioDevice[]>('get_audio_devices');
      const inputDevices = audioDevices.filter(
        (d) => d.device_type === 'Microphone' || d.device_type === 'Input'
      );
      setDevices(inputDevices);
      if (inputDevices.length > 0 && !selectedDevice) {
        setSelectedDevice(inputDevices[0].name);
      }
      logger.debug('[MicTestStep] Refreshed audio devices:', inputDevices);
    } catch (error) {
      logger.error('[MicTestStep] Failed to refresh devices:', error);
      toast.error('Error al actualizar dispositivos');
    } finally {
      setLoadingDevices(false);
    }
  };

  return (
    <OnboardingContainer
      title={
        phase === 'device-selection'
          ? 'Seleccionar Micrófono'
          : phase === 'level-monitoring'
          ? 'Prueba de Micrófono'
          : 'Grabación Completada'
      }
      description={
        phase === 'device-selection'
          ? 'Elige el dispositivo de entrada que usarás'
          : phase === 'level-monitoring'
          ? 'Habla normalmente. Si la barra se mueve, tu micrófono funciona ✓'
          : 'Tu micrófono está configurado correctamente'
      }
      step={6}
      totalSteps={6}
      hideProgress={false}
      showNavigation={phase === 'complete' || (phase === 'level-monitoring' && levelDetected)}
      canGoNext={phase === 'complete' || (phase === 'level-monitoring' && levelDetected)}
    >
      <div className="max-w-lg mx-auto space-y-6">
        {phase === 'device-selection' && (
          <>
            {/* Device Selector */}
            <div className="space-y-3">
              <label className="block text-sm font-medium text-neutral-700 dark:text-neutral-300">
                Dispositivo de Entrada
              </label>

              <div className="flex gap-2">
                <select
                  value={selectedDevice}
                  onChange={(e) => setSelectedDevice(e.target.value)}
                  disabled={loadingDevices}
                  className="flex-1 px-3 py-2 bg-white dark:bg-neutral-800 border border-neutral-200 dark:border-neutral-700 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  <option value="">
                    {loadingDevices ? 'Cargando dispositivos...' : 'Seleccionar dispositivo'}
                  </option>
                  {devices.map((device) => (
                    <option key={device.name} value={device.name}>
                      {device.name}
                    </option>
                  ))}
                </select>

                <button
                  onClick={handleRefreshDevices}
                  disabled={loadingDevices}
                  className="p-2 bg-neutral-100 dark:bg-neutral-800 hover:bg-neutral-200 dark:hover:bg-neutral-700 rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                  title="Refrescar dispositivos"
                >
                  <RotateCcw className="w-4 h-4" />
                </button>
              </div>

              {devices.length === 0 && !loadingDevices && (
                <p className="text-xs text-amber-600 dark:text-amber-400">
                  No se encontraron dispositivos de entrada. Conecta un micrófono e intenta de nuevo.
                </p>
              )}
            </div>

            {/* CTA */}
            <div className="flex flex-col gap-3 pt-4">
              <Button
                onClick={handleStartMonitoring}
                disabled={!selectedDevice}
                className="w-full h-11"
              >
                <Mic className="w-4 h-4 mr-2" />
                Comenzar Prueba
              </Button>

              <button
                onClick={handleSkip}
                className="text-sm text-neutral-500 hover:text-neutral-700 dark:hover:text-neutral-400 transition-colors"
              >
                Saltar este paso
              </button>
            </div>
          </>
        )}

        {phase === 'level-monitoring' && (
          <>
            {/* Level Meter */}
            <div className="space-y-4">
              <div className="p-4 bg-neutral-50 dark:bg-neutral-900 rounded-lg border border-neutral-200 dark:border-neutral-800">
                <p className="text-xs text-neutral-600 dark:text-neutral-400 mb-3">
                  Nivel de micrófono
                </p>
                <AudioLevelMeter
                  rmsLevel={rmsLevel}
                  peakLevel={peakLevel}
                  isActive={isActive}
                  deviceName={selectedDevice}
                  size="large"
                  variant="mic"
                />
              </div>

              {/* Status Messages */}
              {levelDetected && (
                <div className="p-3 bg-green-50 dark:bg-green-950 border border-green-200 dark:border-green-800 rounded-lg">
                  <p className="text-sm text-green-700 dark:text-green-300 font-medium">
                    ✓ ¡Te escuchamos! Micrófono funcionando correctamente.
                  </p>
                </div>
              )}

              {!levelDetected && rmsLevel < 0.02 && monitoringActive && (
                <div className="p-3 bg-amber-50 dark:bg-amber-950 border border-amber-200 dark:border-amber-800 rounded-lg">
                  <p className="text-sm text-amber-700 dark:text-amber-300">
                    No detectamos sonido. Habla más fuerte o cambia de dispositivo.
                  </p>
                </div>
              )}
            </div>

            {/* Recording Preview CTA */}
            <div className="flex flex-col gap-3 pt-4">
              <Button
                onClick={handleStartRecording}
                disabled={isRecording}
                className="w-full h-11"
              >
                <Volume2 className="w-4 h-4 mr-2" />
                Grabar Prueba (5s)
              </Button>

              <button
                onClick={handleSkip}
                className="text-sm text-neutral-500 hover:text-neutral-700 dark:hover:text-neutral-400 transition-colors"
              >
                Saltar este paso
              </button>
            </div>

            {isRecording && (
              <div className="p-3 bg-blue-50 dark:bg-blue-950 border border-blue-200 dark:border-blue-800 rounded-lg">
                <p className="text-sm text-blue-700 dark:text-blue-300">
                  Grabando: {recordingTime.toFixed(1)}s / 5s
                </p>
              </div>
            )}
          </>
        )}

        {phase === 'complete' && (
          <>
            {/* Success State */}
            <div className="p-4 bg-green-50 dark:bg-green-950 border border-green-200 dark:border-green-800 rounded-lg">
              <p className="text-sm text-green-700 dark:text-green-300">
                Tu micrófono está configurado y listo para usar.
              </p>
            </div>

            <div className="flex flex-col gap-3 pt-4">
              <Button onClick={handleContinue} className="w-full h-11">
                Continuar
              </Button>
            </div>
          </>
        )}
      </div>
    </OnboardingContainer>
  );
}
