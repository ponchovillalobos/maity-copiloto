'use client';

/**
 * /dev — Form de testing E2E. Toma un archivo de audio (mp3/wav/m4a),
 * dispara el pipeline completo (decodificación → transcripción → tips →
 * evaluación) y redirige a la reunión generada.
 *
 * Solo accesible vía URL directa. NO aparece en navegación principal.
 * Se mantiene útil mientras iteramos calidad sin grabar reuniones reales.
 */

import { useEffect, useRef, useState } from 'react';
import { useRouter } from 'next/navigation';
import { listen } from '@tauri-apps/api/event';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { safeInvoke } from '@/lib/safeInvoke';
import { Upload, Loader2, FileAudio, CheckCircle2, AlertCircle } from 'lucide-react';

interface ImportProgress {
  stage: 'decoding' | 'transcribing' | 'evaluating' | 'done';
  current_chunk: number;
  total_chunks: number;
  message: string;
}

interface ImportResult {
  meeting_id: string;
  transcript_segments: number;
  total_duration_seconds: number;
}

export default function DevImportPage() {
  const router = useRouter();
  const [filePath, setFilePath] = useState<string | null>(null);
  const [meetingName, setMeetingName] = useState('');
  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState<ImportProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<ImportResult | null>(null);
  const cancelRef = useRef(false);

  useEffect(() => {
    const u = listen<ImportProgress>('dev-import-progress', (e) => {
      if (cancelRef.current) return;
      setProgress(e.payload);
    });
    return () => {
      u.then((fn) => fn());
    };
  }, []);

  const handlePickFile = async () => {
    try {
      const selected = await openDialog({
        multiple: false,
        directory: false,
        filters: [
          { name: 'Audio', extensions: ['mp3', 'wav', 'm4a', 'ogg', 'flac', 'webm'] },
        ],
      });
      if (typeof selected === 'string') {
        setFilePath(selected);
        setError(null);
      }
    } catch (e) {
      setError(`No se pudo abrir el selector: ${String(e)}`);
    }
  };

  const handleStart = async () => {
    if (!filePath) return;
    setRunning(true);
    setError(null);
    setResult(null);
    setProgress({ stage: 'decoding', current_chunk: 0, total_chunks: 0, message: 'Iniciando…' });

    const res = await safeInvoke<ImportResult>(
      'dev_import_audio_file',
      { filePath, meetingName: meetingName.trim() || null },
      'No se pudo procesar el audio. Revisa logs.',
    );

    if (res) {
      setResult(res);
      setProgress({
        stage: 'done',
        current_chunk: 1,
        total_chunks: 1,
        message: 'Listo',
      });
      setTimeout(() => {
        router.push(`/meeting-details?id=${encodeURIComponent(res.meeting_id)}&source=test`);
      }, 1200);
    } else {
      setError('La importación falló. Revisa la consola para más detalles.');
    }
    setRunning(false);
  };

  const overallPct = progress
    ? progress.stage === 'done'
      ? 100
      : progress.stage === 'evaluating'
        ? 95
        : progress.stage === 'transcribing'
          ? Math.min(90, Math.round((progress.current_chunk / Math.max(1, progress.total_chunks)) * 90))
          : 5
    : 0;

  return (
    <div className="min-h-screen bg-gray-950 text-gray-100 p-8">
      <div className="max-w-2xl mx-auto space-y-6">
        <header className="space-y-2">
          <h1 className="text-2xl font-bold">Importar audio de prueba</h1>
          <p className="text-sm text-gray-400">
            Carga un archivo .mp3 / .wav / .m4a y Maity ejecutará el pipeline
            completo: transcripción + evaluación post-meeting. Útil para iterar
            calidad sin grabar reuniones reales.
          </p>
        </header>

        <section className="rounded-xl border border-white/10 bg-white/5 p-6 space-y-4">
          <div>
            <label className="text-sm font-medium text-gray-200 block mb-2">
              Archivo de audio
            </label>
            <button
              onClick={handlePickFile}
              disabled={running}
              className="w-full flex items-center gap-3 rounded-lg border border-dashed border-white/20 hover:border-blue-400 bg-white/5 px-4 py-6 transition-colors disabled:opacity-50"
            >
              <Upload className="w-5 h-5 text-blue-300" />
              <div className="flex-1 text-left">
                {filePath ? (
                  <>
                    <div className="text-sm font-medium text-gray-100 truncate">
                      {filePath.split(/[\\/]/).pop()}
                    </div>
                    <div className="text-xs text-gray-400 truncate">{filePath}</div>
                  </>
                ) : (
                  <div className="text-sm text-gray-300">
                    Click para seleccionar archivo (.mp3, .wav, .m4a)
                  </div>
                )}
              </div>
              <FileAudio className="w-5 h-5 text-gray-500" />
            </button>
          </div>

          <div>
            <label className="text-sm font-medium text-gray-200 block mb-2">
              Nombre de la reunión (opcional)
            </label>
            <input
              type="text"
              value={meetingName}
              onChange={(e) => setMeetingName(e.target.value)}
              placeholder="Test Reunión Ventas"
              disabled={running}
              className="w-full rounded-lg border border-white/15 bg-white/5 px-3 py-2 text-sm text-gray-100 placeholder:text-gray-500 focus:outline-none focus:border-blue-400 disabled:opacity-50"
            />
          </div>

          <button
            onClick={handleStart}
            disabled={!filePath || running}
            className="w-full rounded-lg bg-blue-500 hover:bg-blue-600 disabled:bg-gray-700 disabled:cursor-not-allowed text-white text-sm font-medium px-4 py-3 flex items-center justify-center gap-2"
          >
            {running ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin" />
                Procesando…
              </>
            ) : (
              <>Procesar reunión simulada</>
            )}
          </button>
        </section>

        {progress && (
          <section className="rounded-xl border border-white/10 bg-white/5 p-6 space-y-4">
            <div className="flex items-center justify-between text-sm">
              <span className="font-medium capitalize">{progress.stage}</span>
              <span className="tabular-nums">{overallPct}%</span>
            </div>
            <div className="h-2 rounded-full bg-white/10 overflow-hidden">
              <div
                className="h-full bg-gradient-to-r from-blue-400 to-emerald-400 transition-all duration-300"
                style={{ width: `${overallPct}%` }}
              />
            </div>
            <p className="text-xs text-gray-400">{progress.message}</p>
          </section>
        )}

        {error && (
          <section className="rounded-lg border border-rose-500/40 bg-rose-500/10 p-4 flex items-start gap-2">
            <AlertCircle className="w-4 h-4 text-rose-400 mt-0.5 flex-shrink-0" />
            <p className="text-sm text-rose-100">{error}</p>
          </section>
        )}

        {result && !running && (
          <section className="rounded-lg border border-emerald-500/40 bg-emerald-500/10 p-4 flex items-start gap-2">
            <CheckCircle2 className="w-5 h-5 text-emerald-400 mt-0.5 flex-shrink-0" />
            <div className="flex-1 text-sm text-emerald-100">
              <p className="font-semibold">
                Reunión creada con {result.transcript_segments} segmentos (
                {Math.round(result.total_duration_seconds)}s totales)
              </p>
              <p className="text-xs text-emerald-200/80 mt-1">
                Redirigiendo a meeting-details…
              </p>
            </div>
          </section>
        )}
      </div>
    </div>
  );
}
