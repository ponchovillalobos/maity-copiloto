'use client';

import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';

type ScenarioResult = {
  scenario: string;
  category: string;
  generated_tip: string | null;
  verb_match: boolean;
  has_quoted_phrase: boolean;
  word_count: number;
  passed: boolean;
  latency_ms: number;
  notes: string;
};

type EvalReport = {
  total: number;
  passed: number;
  failed: number;
  avg_latency_ms: number;
  results: ScenarioResult[];
};

export default function ScenariosPage() {
  const [report, setReport] = useState<EvalReport | null>(null);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const runAll = async () => {
    setRunning(true);
    setError(null);
    setReport(null);
    try {
      const r = await invoke<EvalReport>('dev_eval_scenarios');
      setReport(r);
    } catch (e) {
      setError(String(e));
    } finally {
      setRunning(false);
    }
  };

  return (
    <div className="p-6 bg-gray-950 text-gray-100 min-h-screen font-mono text-sm">
      <h1 className="text-2xl font-bold mb-4">Tip Eval Harness — v31.10</h1>
      <p className="text-gray-400 mb-4">
        Corre 12 scenarios pre-armados contra <code>coach_simple_tick</code> y reporta
        si el tip cumple formato <code>VERBO:&quot;frase&quot;</code> + verbo esperado.
        Iteración: ajusta prompt en <code>coach/commands.rs:246</code>, rebuild, repite.
      </p>

      <button
        onClick={runAll}
        disabled={running}
        className="px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-700 rounded text-white font-bold mb-6"
      >
        {running ? 'Corriendo 12 scenarios... (~3-5min)' : 'Run All Scenarios'}
      </button>

      {error && (
        <div className="bg-red-900/30 border border-red-500 p-3 rounded mb-4">
          ERROR: {error}
        </div>
      )}

      {report && (
        <>
          <div className="bg-gray-900 p-4 rounded mb-4">
            <div className="text-lg">
              <span className="text-green-400">{report.passed} PASS</span>
              {' / '}
              <span className="text-red-400">{report.failed} FAIL</span>
              {' / '}
              <span>{report.total} total</span>
              {' — '}
              <span className="text-gray-400">avg {report.avg_latency_ms}ms</span>
            </div>
            <div className="text-sm text-gray-500 mt-1">
              Pass rate: {((report.passed / report.total) * 100).toFixed(0)}%
            </div>
          </div>

          <table className="w-full border-collapse">
            <thead>
              <tr className="border-b border-gray-700 text-left">
                <th className="p-2 w-8"></th>
                <th className="p-2">Scenario</th>
                <th className="p-2">Tip generado</th>
                <th className="p-2 w-16">Words</th>
                <th className="p-2 w-20">Latency</th>
                <th className="p-2">Notas</th>
              </tr>
            </thead>
            <tbody>
              {report.results.map((r, i) => (
                <tr
                  key={i}
                  className={`border-b border-gray-800 ${
                    r.passed ? 'bg-green-900/10' : 'bg-red-900/10'
                  }`}
                >
                  <td className="p-2 text-center">
                    {r.passed ? (
                      <span className="text-green-400">✓</span>
                    ) : (
                      <span className="text-red-400">✗</span>
                    )}
                  </td>
                  <td className="p-2">
                    <div className="font-bold">{r.scenario}</div>
                    <div className="text-xs text-gray-500">{r.category}</div>
                  </td>
                  <td className="p-2 italic text-gray-300">
                    {r.generated_tip ?? <span className="text-gray-600">— sin tip —</span>}
                  </td>
                  <td className="p-2 text-center">{r.word_count}</td>
                  <td className="p-2 text-right">{r.latency_ms}ms</td>
                  <td className="p-2 text-xs text-yellow-400">{r.notes || '—'}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </>
      )}
    </div>
  );
}
