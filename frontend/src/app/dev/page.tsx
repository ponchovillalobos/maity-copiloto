'use client';

/**
 * /dev — Form de testing E2E. Dos modos:
 *
 * 1. **Single audio** (estéreo o mono): un archivo, decodifica y procesa.
 * 2. **QA con ground truth**: dos archivos (mic.wav + sys.wav) + texto exacto
 *    de cada speaker → calcula WER (Word Error Rate) por canal y global.
 *
 * Solo accesible vía URL directa (`/dev`). NO aparece en navegación principal.
 */

import { useEffect, useRef, useState } from 'react';
import { useRouter, useSearchParams } from 'next/navigation';
import { listen } from '@tauri-apps/api/event';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { quietInvoke, safeInvoke } from '@/lib/safeInvoke';
import { Upload, Loader2, FileAudio, CheckCircle2, AlertCircle, Target, Layers, FolderOpen, X } from 'lucide-react';

interface ImportProgress {
  stage: 'decoding' | 'transcribing' | 'evaluating' | 'done';
  current_chunk: number;
  total_chunks: number;
  message: string;
}

interface WerResult {
  wer: number;
  reference_words: number;
  hypothesis_words: number;
  substitutions: number;
  insertions: number;
  deletions: number;
  hits: number;
}

interface ImportResult {
  meeting_id: string;
  transcript_segments: number;
  total_duration_seconds: number;
  channel_layout: 'stereo' | 'mono' | 'two-files';
  wer_global?: WerResult;
  wer_user?: WerResult;
  wer_interlocutor?: WerResult;
  maity_transcript_full: string;
}

type Mode = 'single' | 'qa' | 'batch';

interface BatchScenario {
  name: string;
  user_audio_path: string;
  interlocutor_audio_path: string;
  ground_truth_user: string;
  ground_truth_interlocutor: string;
}

interface BatchRunStatus {
  name: string;
  status: 'pending' | 'running' | 'done' | 'error';
  wer_user?: number;
  wer_inter?: number;
  pipeline_ms?: number;
  error?: string;
}

export default function DevImportPage() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const initialAutoMode = (searchParams?.get('mode') as Mode) || 'single';
  const [mode, setMode] = useState<Mode>(initialAutoMode);

  // Single mode
  const [filePath, setFilePath] = useState<string | null>(null);
  // QA mode
  const [userAudioPath, setUserAudioPath] = useState<string | null>(null);
  const [interAudioPath, setInterAudioPath] = useState<string | null>(null);
  const [groundTruthUser, setGroundTruthUser] = useState('');
  const [groundTruthInter, setGroundTruthInter] = useState('');

  const [meetingName, setMeetingName] = useState('');
  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState<ImportProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<ImportResult | null>(null);

  // Batch mode
  const [batchFolder, setBatchFolder] = useState<string>('D:\\Poncho\\Videos\\Edicion-Claude\\output');
  const [batchScenarios, setBatchScenarios] = useState<BatchScenario[]>([]);
  const [batchRuns, setBatchRuns] = useState<BatchRunStatus[]>([]);
  const [batchCurrentIdx, setBatchCurrentIdx] = useState(0);
  const [batchAbort, setBatchAbort] = useState(false);
  const batchAbortRef = useRef(false);

  useEffect(() => {
    const u = listen<ImportProgress>('dev-import-progress', (e) => {
      setProgress(e.payload);
    });
    return () => {
      u.then((fn) => fn());
    };
  }, []);

  const pickAudioFile = async (): Promise<string | null> => {
    try {
      const selected = await openDialog({
        multiple: false,
        directory: false,
        filters: [
          { name: 'Audio', extensions: ['mp3', 'wav', 'm4a', 'ogg', 'flac', 'webm'] },
        ],
      });
      return typeof selected === 'string' ? selected : null;
    } catch (e) {
      setError(`No se pudo abrir el selector: ${String(e)}`);
      return null;
    }
  };

  const handleStart = async () => {
    setRunning(true);
    setError(null);
    setResult(null);
    setProgress({ stage: 'decoding', current_chunk: 0, total_chunks: 0, message: 'Iniciando…' });

    let res: ImportResult | null = null;

    if (mode === 'single') {
      if (!filePath) {
        setError('Selecciona un archivo');
        setRunning(false);
        return;
      }
      res = await safeInvoke<ImportResult>(
        'dev_import_audio_file',
        { filePath, meetingName: meetingName.trim() || null },
        'No se pudo procesar el audio.',
      );
    } else {
      if (!userAudioPath || !interAudioPath) {
        setError('Selecciona ambos archivos (user + interlocutor)');
        setRunning(false);
        return;
      }
      res = await safeInvoke<ImportResult>(
        'dev_import_two_audios',
        {
          userAudioPath,
          interlocutorAudioPath: interAudioPath,
          groundTruthUser: groundTruthUser.trim() || null,
          groundTruthInterlocutor: groundTruthInter.trim() || null,
          meetingName: meetingName.trim() || null,
        },
        'No se pudieron procesar los audios.',
      );
    }

    if (res) {
      setResult(res);
      setProgress({ stage: 'done', current_chunk: 1, total_chunks: 1, message: 'Listo' });
      // En modo QA con WER, NO redirigimos automáticamente — el usuario quiere ver métricas.
      const hasWer = !!(res.wer_global || res.wer_user || res.wer_interlocutor);
      if (!hasWer) {
        setTimeout(() => {
          router.push(`/meeting-details?id=${encodeURIComponent(res!.meeting_id)}&source=test`);
        }, 1500);
      }
    } else {
      setError('La importación falló. Revisa la consola para más detalles.');
    }
    setRunning(false);
  };

  const handlePickFolder = async () => {
    try {
      const selected = await openDialog({ multiple: false, directory: true });
      if (typeof selected === 'string') {
        setBatchFolder(selected);
        setBatchScenarios([]);
        setBatchRuns([]);
      }
    } catch (e) {
      setError(`No se pudo abrir selector de carpeta: ${String(e)}`);
    }
  };

  const handleScanBatchFolder = async () => {
    if (!batchFolder.trim()) return;
    setError(null);
    const list = await safeInvoke<BatchScenario[]>(
      'dev_list_batch_scenarios',
      { folderPath: batchFolder.trim() },
      'No se pudo escanear la carpeta.',
    );
    if (list) {
      setBatchScenarios(list);
      setBatchRuns(list.map((s) => ({ name: s.name, status: 'pending' })));
    }
  };

  const handleStartBatch = async () => {
    if (batchScenarios.length === 0) return;
    setRunning(true);
    setError(null);
    setBatchAbort(false);
    batchAbortRef.current = false;

    for (let i = 0; i < batchScenarios.length; i++) {
      if (batchAbortRef.current) {
        setError('Batch abortado por el usuario.');
        break;
      }
      const sc = batchScenarios[i];
      setBatchCurrentIdx(i);
      setBatchRuns((prev) => prev.map((r, idx) => (idx === i ? { ...r, status: 'running' } : r)));

      const t0 = Date.now();
      const res = await quietInvoke<ImportResult>('dev_import_two_audios', {
        userAudioPath: sc.user_audio_path,
        interlocutorAudioPath: sc.interlocutor_audio_path,
        groundTruthUser: sc.ground_truth_user.trim() || null,
        groundTruthInterlocutor: sc.ground_truth_interlocutor.trim() || null,
        meetingName: sc.name,
      });
      const elapsed = Date.now() - t0;

      setBatchRuns((prev) =>
        prev.map((r, idx) =>
          idx === i
            ? res
              ? {
                  ...r,
                  status: 'done',
                  wer_user: res.wer_user?.wer,
                  wer_inter: res.wer_interlocutor?.wer,
                  pipeline_ms: elapsed,
                }
              : { ...r, status: 'error', error: 'invoke falló' }
            : r,
        ),
      );
    }

    setRunning(false);
    setBatchCurrentIdx(batchScenarios.length);
  };

  const handleAbortBatch = () => {
    batchAbortRef.current = true;
    setBatchAbort(true);
  };

  // Auto-run desde URL: /dev?mode=batch&autorun=1&folder=<path>
  const autoRunRef = useRef(false);
  useEffect(() => {
    if (autoRunRef.current) return;
    if (mode !== 'batch') return;
    const autorun = searchParams?.get('autorun');
    const folder = searchParams?.get('folder');
    if (autorun !== '1' || !folder) return;
    autoRunRef.current = true;
    setBatchFolder(folder);
    (async () => {
      const list = await safeInvoke<BatchScenario[]>(
        'dev_list_batch_scenarios',
        { folderPath: folder },
        'No se pudo escanear la carpeta.',
      );
      if (!list || list.length === 0) return;
      setBatchScenarios(list);
      setBatchRuns(list.map((s) => ({ name: s.name, status: 'pending' })));
      setRunning(true);
      batchAbortRef.current = false;
      for (let i = 0; i < list.length; i++) {
        if (batchAbortRef.current) break;
        const sc = list[i];
        setBatchCurrentIdx(i);
        setBatchRuns((prev) => prev.map((r, idx) => (idx === i ? { ...r, status: 'running' } : r)));
        const t0 = Date.now();
        const res = await quietInvoke<ImportResult>('dev_import_two_audios', {
          userAudioPath: sc.user_audio_path,
          interlocutorAudioPath: sc.interlocutor_audio_path,
          groundTruthUser: sc.ground_truth_user.trim() || null,
          groundTruthInterlocutor: sc.ground_truth_interlocutor.trim() || null,
          meetingName: sc.name,
        });
        const elapsed = Date.now() - t0;
        setBatchRuns((prev) =>
          prev.map((r, idx) =>
            idx === i
              ? res
                ? { ...r, status: 'done', wer_user: res.wer_user?.wer, wer_inter: res.wer_interlocutor?.wer, pipeline_ms: elapsed }
                : { ...r, status: 'error', error: 'invoke falló' }
              : r,
          ),
        );
      }
      setRunning(false);
      setBatchCurrentIdx(list.length);
      try {
        const tt = await quietInvoke<{ run_id: string; tips_generated: number; avg_similarity: number }>('dev_run_tip_tests', {});
        if (tt) console.log('[dev/batch] tip_tests:', tt);
      } catch (e) {
        console.warn('[dev/batch] dev_run_tip_tests failed:', e);
      }
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [mode]);

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
      <div className="max-w-3xl mx-auto space-y-6">
        <header className="space-y-2">
          <h1 className="text-2xl font-bold">Importar audio de prueba</h1>
          <p className="text-sm text-gray-400">
            Carga audio y dispara el pipeline completo de Maity. Modo QA mide
            WER (Word Error Rate) contra ground truth para validar precisión.
          </p>
        </header>

        {/* Mode selector */}
        <div className="flex gap-2 border-b border-white/10 pb-3">
          <button
            onClick={() => setMode('single')}
            disabled={running}
            className={`px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
              mode === 'single'
                ? 'bg-blue-500 text-white'
                : 'bg-white/5 text-gray-300 hover:bg-white/10'
            }`}
          >
            <FileAudio className="inline w-4 h-4 mr-2" />
            Single (estéreo o mono)
          </button>
          <button
            onClick={() => setMode('qa')}
            disabled={running}
            className={`px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
              mode === 'qa'
                ? 'bg-emerald-500 text-white'
                : 'bg-white/5 text-gray-300 hover:bg-white/10'
            }`}
          >
            <Target className="inline w-4 h-4 mr-2" />
            QA con ground truth
          </button>
          <button
            onClick={() => setMode('batch')}
            disabled={running}
            className={`px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
              mode === 'batch'
                ? 'bg-purple-500 text-white'
                : 'bg-white/5 text-gray-300 hover:bg-white/10'
            }`}
          >
            <Layers className="inline w-4 h-4 mr-2" />
            Batch (carpeta entera)
          </button>
        </div>

        {mode === 'batch' ? (
          <section className="rounded-xl border border-purple-500/30 bg-purple-500/5 p-6 space-y-4">
            <div className="text-xs text-purple-100/80 leading-relaxed">
              Escanea una carpeta con subcarpetas-scenarios. Cada subfolder debe
              contener: <code>*-valentina.mp3</code> (user), <code>*-cliente.mp3</code> (interlocutor)
              y opcionalmente <code>*.txt</code> con líneas <code>[user] ...</code> /
              <code>[interlocutor] ...</code> como ground truth. Procesa secuencial,
              persiste cada iteración en `dev_iterations` para ver en /dashboard.
            </div>

            <div>
              <label className="text-sm font-medium text-gray-200 block mb-2">Carpeta raíz</label>
              <div className="flex gap-2">
                <input
                  type="text"
                  value={batchFolder}
                  onChange={(e) => setBatchFolder(e.target.value)}
                  placeholder="D:\\Poncho\\Videos\\Edicion-Claude\\output"
                  disabled={running}
                  className="flex-1 rounded-lg border border-white/15 bg-white/5 px-3 py-2 text-sm text-gray-100 placeholder:text-gray-500 focus:outline-none focus:border-purple-400 disabled:opacity-50"
                />
                <button
                  onClick={handlePickFolder}
                  disabled={running}
                  className="rounded-lg border border-white/15 bg-white/5 hover:bg-white/10 px-3 py-2 text-sm text-gray-200 disabled:opacity-50"
                >
                  <FolderOpen className="w-4 h-4" />
                </button>
                <button
                  onClick={handleScanBatchFolder}
                  disabled={running || !batchFolder.trim()}
                  className="rounded-lg bg-purple-500 hover:bg-purple-600 disabled:bg-gray-700 px-4 py-2 text-sm text-white"
                >
                  Escanear
                </button>
              </div>
            </div>

            {batchScenarios.length > 0 && (
              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <span className="text-sm font-medium text-gray-200">
                    {batchScenarios.length} scenarios detectados
                  </span>
                  {!running ? (
                    <button
                      onClick={handleStartBatch}
                      className="rounded-lg bg-purple-500 hover:bg-purple-600 text-white text-sm font-medium px-4 py-2"
                    >
                      Procesar todos
                    </button>
                  ) : (
                    <button
                      onClick={handleAbortBatch}
                      className="rounded-lg bg-rose-500 hover:bg-rose-600 text-white text-sm font-medium px-4 py-2 flex items-center gap-2"
                    >
                      <X className="w-4 h-4" />
                      Abortar
                    </button>
                  )}
                </div>

                {running && (
                  <div className="text-xs text-purple-200">
                    Procesando {batchCurrentIdx + 1}/{batchScenarios.length}: <b>{batchScenarios[batchCurrentIdx]?.name}</b>
                  </div>
                )}

                <div className="max-h-96 overflow-y-auto rounded-lg border border-white/10 bg-black/20">
                  <table className="w-full text-xs">
                    <thead className="sticky top-0 bg-gray-900 text-[10px] uppercase text-gray-400 border-b border-white/10">
                      <tr>
                        <th className="text-left p-2">#</th>
                        <th className="text-left p-2">Scenario</th>
                        <th className="text-left p-2">Status</th>
                        <th className="text-right p-2">WER user</th>
                        <th className="text-right p-2">WER inter</th>
                        <th className="text-right p-2">Tiempo</th>
                      </tr>
                    </thead>
                    <tbody>
                      {batchRuns.map((r, idx) => {
                        const werColor = (w?: number) =>
                          w == null ? 'text-gray-500' : w < 0.1 ? 'text-emerald-400' : w < 0.2 ? 'text-amber-400' : 'text-rose-400';
                        const statusBadge = {
                          pending: <span className="text-gray-500">⏳ pending</span>,
                          running: <span className="text-blue-400">⚙ running</span>,
                          done: <span className="text-emerald-400">✓ done</span>,
                          error: <span className="text-rose-400">✗ error</span>,
                        }[r.status];
                        return (
                          <tr key={r.name} className="border-b border-white/5">
                            <td className="p-2 text-gray-500 tabular-nums">{idx + 1}</td>
                            <td className="p-2 text-gray-200">{r.name}</td>
                            <td className="p-2">{statusBadge}</td>
                            <td className={`p-2 text-right tabular-nums ${werColor(r.wer_user)}`}>
                              {r.wer_user != null ? `${(r.wer_user * 100).toFixed(1)}%` : '–'}
                            </td>
                            <td className={`p-2 text-right tabular-nums ${werColor(r.wer_inter)}`}>
                              {r.wer_inter != null ? `${(r.wer_inter * 100).toFixed(1)}%` : '–'}
                            </td>
                            <td className="p-2 text-right tabular-nums text-gray-400">
                              {r.pipeline_ms != null ? `${(r.pipeline_ms / 1000).toFixed(1)}s` : '–'}
                            </td>
                          </tr>
                        );
                      })}
                    </tbody>
                  </table>
                </div>

                <div className="text-[10px] text-gray-500">
                  Después de procesar todo, abre <a href="/dashboard" className="underline text-purple-300">/dashboard</a> para ver tendencias agregadas.
                </div>
              </div>
            )}
          </section>
        ) : mode === 'single' ? (
          <section className="rounded-xl border border-white/10 bg-white/5 p-6 space-y-4">
            <div>
              <label className="text-sm font-medium text-gray-200 block mb-2">Archivo de audio</label>
              <button
                onClick={async () => {
                  const p = await pickAudioFile();
                  if (p) {
                    setFilePath(p);
                    setError(null);
                  }
                }}
                disabled={running}
                className="w-full flex items-center gap-3 rounded-lg border border-dashed border-white/20 hover:border-blue-400 bg-white/5 px-4 py-6 transition-colors disabled:opacity-50"
              >
                <Upload className="w-5 h-5 text-blue-300" />
                <div className="flex-1 text-left">
                  {filePath ? (
                    <>
                      <div className="text-sm font-medium text-gray-100 truncate">{filePath.split(/[\\/]/).pop()}</div>
                      <div className="text-xs text-gray-400 truncate">{filePath}</div>
                    </>
                  ) : (
                    <div className="text-sm text-gray-300">Click para seleccionar (.mp3, .wav, .m4a, .ogg, .flac)</div>
                  )}
                </div>
                <FileAudio className="w-5 h-5 text-gray-500" />
              </button>
              <p className="text-xs text-gray-500 mt-2">
                Estéreo recomendado: <b>L=user</b>, <b>R=interlocutor</b>. Mono = todo interlocutor.
              </p>
            </div>
          </section>
        ) : (
          <section className="rounded-xl border border-emerald-500/30 bg-emerald-500/5 p-6 space-y-4">
            <div className="text-xs text-emerald-100/80 leading-relaxed">
              Sube los dos audios por separado y opcionalmente el texto exacto que se dice en cada
              uno. Maity transcribirá ambos y mostrará el WER (Word Error Rate) — métrica estándar
              STT donde 0% = perfecto, &lt;10% = excelente, &lt;20% = aceptable.
            </div>

            <div>
              <label className="text-sm font-medium text-gray-200 block mb-2">
                Audio del USER (tu micrófono)
              </label>
              <button
                onClick={async () => {
                  const p = await pickAudioFile();
                  if (p) {
                    setUserAudioPath(p);
                    setError(null);
                  }
                }}
                disabled={running}
                className="w-full flex items-center gap-3 rounded-lg border border-dashed border-white/20 hover:border-blue-400 bg-white/5 px-4 py-4 disabled:opacity-50"
              >
                <Upload className="w-4 h-4 text-blue-300" />
                <div className="flex-1 text-left text-sm">
                  {userAudioPath ? (
                    <span className="text-gray-100 truncate">{userAudioPath.split(/[\\/]/).pop()}</span>
                  ) : (
                    <span className="text-gray-400">Seleccionar audio del usuario</span>
                  )}
                </div>
              </button>
              <textarea
                value={groundTruthUser}
                onChange={(e) => setGroundTruthUser(e.target.value)}
                placeholder="(opcional) Texto exacto que dice el USER. Pegar la transcripción de referencia para calcular WER."
                disabled={running}
                rows={3}
                className="w-full mt-2 rounded-lg border border-white/15 bg-white/5 px-3 py-2 text-xs text-gray-100 placeholder:text-gray-500 focus:outline-none focus:border-emerald-400 disabled:opacity-50 resize-y"
              />
            </div>

            <div>
              <label className="text-sm font-medium text-gray-200 block mb-2">
                Audio del INTERLOCUTOR (cliente / sistema)
              </label>
              <button
                onClick={async () => {
                  const p = await pickAudioFile();
                  if (p) {
                    setInterAudioPath(p);
                    setError(null);
                  }
                }}
                disabled={running}
                className="w-full flex items-center gap-3 rounded-lg border border-dashed border-white/20 hover:border-emerald-400 bg-white/5 px-4 py-4 disabled:opacity-50"
              >
                <Upload className="w-4 h-4 text-emerald-300" />
                <div className="flex-1 text-left text-sm">
                  {interAudioPath ? (
                    <span className="text-gray-100 truncate">{interAudioPath.split(/[\\/]/).pop()}</span>
                  ) : (
                    <span className="text-gray-400">Seleccionar audio del interlocutor</span>
                  )}
                </div>
              </button>
              <textarea
                value={groundTruthInter}
                onChange={(e) => setGroundTruthInter(e.target.value)}
                placeholder="(opcional) Texto exacto que dice el INTERLOCUTOR."
                disabled={running}
                rows={3}
                className="w-full mt-2 rounded-lg border border-white/15 bg-white/5 px-3 py-2 text-xs text-gray-100 placeholder:text-gray-500 focus:outline-none focus:border-emerald-400 disabled:opacity-50 resize-y"
              />
            </div>
          </section>
        )}

        <div>
          <label className="text-sm font-medium text-gray-200 block mb-2">
            Nombre de la reunión (opcional)
          </label>
          <input
            type="text"
            value={meetingName}
            onChange={(e) => setMeetingName(e.target.value)}
            placeholder={mode === 'qa' ? 'QA Test Ventas 01' : 'Test Reunión Ventas'}
            disabled={running}
            className="w-full rounded-lg border border-white/15 bg-white/5 px-3 py-2 text-sm text-gray-100 placeholder:text-gray-500 focus:outline-none focus:border-blue-400 disabled:opacity-50"
          />
        </div>

        {mode !== 'batch' && <button
          onClick={handleStart}
          disabled={running || (mode === 'single' ? !filePath : !userAudioPath || !interAudioPath)}
          className={`w-full rounded-lg ${
            mode === 'qa' ? 'bg-emerald-500 hover:bg-emerald-600' : 'bg-blue-500 hover:bg-blue-600'
          } disabled:bg-gray-700 disabled:cursor-not-allowed text-white text-sm font-medium px-4 py-3 flex items-center justify-center gap-2`}
        >
          {running ? (
            <>
              <Loader2 className="w-4 h-4 animate-spin" />
              Procesando…
            </>
          ) : mode === 'qa' ? (
            <>Procesar y calcular WER</>
          ) : (
            <>Procesar reunión simulada</>
          )}
        </button>}

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
          <>
            <section className="rounded-lg border border-emerald-500/40 bg-emerald-500/10 p-4 space-y-2">
              <div className="flex items-start gap-2">
                <CheckCircle2 className="w-5 h-5 text-emerald-400 mt-0.5 flex-shrink-0" />
                <div className="flex-1 text-sm text-emerald-100">
                  <p className="font-semibold">
                    Reunión creada con {result.transcript_segments} segmentos (
                    {Math.round(result.total_duration_seconds)}s totales)
                  </p>
                  <p className="text-xs text-emerald-200/80 mt-1">
                    Layout: <b>{result.channel_layout === 'stereo' ? 'Estéreo (L=user, R=interlocutor)' : result.channel_layout === 'mono' ? 'Mono (todo = interlocutor)' : 'Dos archivos separados'}</b>
                  </p>
                  <button
                    onClick={() => router.push(`/meeting-details?id=${encodeURIComponent(result.meeting_id)}&source=test`)}
                    className="mt-2 text-xs underline text-emerald-200 hover:text-white"
                  >
                    Ver reunión completa →
                  </button>
                </div>
              </div>
            </section>

            {(result.wer_global || result.wer_user || result.wer_interlocutor) && (
              <section className="rounded-lg border border-amber-500/40 bg-amber-500/5 p-4 space-y-3">
                <div className="flex items-center gap-2">
                  <Target className="w-5 h-5 text-amber-400" />
                  <h2 className="text-base font-semibold text-amber-100">Métricas WER</h2>
                </div>

                {result.wer_global && <WerCard label="Global" w={result.wer_global} />}
                {result.wer_user && <WerCard label="User (mic)" w={result.wer_user} />}
                {result.wer_interlocutor && <WerCard label="Interlocutor (sistema)" w={result.wer_interlocutor} />}

                <div className="text-[10px] text-amber-100/60 mt-2">
                  WER &lt; 10% excelente · &lt; 20% aceptable · &gt; 30% revisar audio o modelo.
                </div>
              </section>
            )}

            <section className="rounded-lg border border-white/10 bg-white/5 p-4">
              <h3 className="text-sm font-semibold text-gray-200 mb-2">Transcripción de Maity</h3>
              <pre className="text-xs text-gray-300 whitespace-pre-wrap break-words max-h-60 overflow-y-auto bg-black/20 p-3 rounded">
                {result.maity_transcript_full || '(sin texto)'}
              </pre>
            </section>
          </>
        )}
      </div>
    </div>
  );
}

function WerCard({ label, w }: { label: string; w: WerResult }) {
  const pct = (w.wer * 100).toFixed(1);
  const color =
    w.wer < 0.1 ? 'text-emerald-400' : w.wer < 0.2 ? 'text-amber-400' : 'text-rose-400';
  return (
    <div className="rounded-md border border-white/10 bg-black/20 p-3">
      <div className="flex items-center justify-between mb-2">
        <span className="text-sm font-medium text-gray-100">{label}</span>
        <span className={`text-xl font-bold tabular-nums ${color}`}>{pct}%</span>
      </div>
      <div className="grid grid-cols-4 gap-2 text-[10px] text-gray-400">
        <div>
          <div className="text-gray-500">Hits</div>
          <div className="text-emerald-300 font-mono">{w.hits}</div>
        </div>
        <div>
          <div className="text-gray-500">Subs</div>
          <div className="text-amber-300 font-mono">{w.substitutions}</div>
        </div>
        <div>
          <div className="text-gray-500">Ins</div>
          <div className="text-blue-300 font-mono">{w.insertions}</div>
        </div>
        <div>
          <div className="text-gray-500">Del</div>
          <div className="text-rose-300 font-mono">{w.deletions}</div>
        </div>
      </div>
      <div className="text-[10px] text-gray-500 mt-2">
        Ref: {w.reference_words} palabras · Hyp: {w.hypothesis_words} palabras
      </div>
    </div>
  );
}
