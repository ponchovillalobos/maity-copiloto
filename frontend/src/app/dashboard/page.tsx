'use client';

/**
 * /dashboard — Control milimétrico de Maity. Vista única con métricas live
 * (CPU/RAM, modelos, eventos), histórico de iteraciones (`/dev`), trends de
 * calidad (WER + scores) y matriz auditable de botones.
 *
 * Solo accesible vía URL directa o CommandPalette.
 */

import { useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';
import { ArrowLeft, Gauge } from 'lucide-react';
import { quietInvoke } from '@/lib/safeInvoke';
import { SystemPanel } from './_components/SystemPanel';
import { ModelPanel } from './_components/ModelPanel';
import { SummaryKPIs } from './_components/SummaryKPIs';
import { PipelineTimingChart } from './_components/PipelineTimingChart';
import { QualityTrendsChart } from './_components/QualityTrendsChart';
import { IterationsTable, type IterationRow } from './_components/IterationsTable';
import { IterationDetailModal } from './_components/IterationDetailModal';
import { ButtonsMatrix } from './_components/ButtonsMatrix';
import { LiveEventsStream } from './_components/LiveEventsStream';
import { PromptsSummary } from './_components/PromptsSummary';

export default function DashboardPage() {
  const router = useRouter();
  const [iterations, setIterations] = useState<IterationRow[]>([]);
  const [selectedId, setSelectedId] = useState<number | null>(null);

  useEffect(() => {
    let cancelled = false;
    const tick = async () => {
      const r = await quietInvoke<IterationRow[]>('dashboard_list_iterations', { limit: 200 });
      if (!cancelled && r) setIterations(r);
    };
    tick();
    const id = setInterval(tick, 5_000);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, []);

  return (
    <div className="min-h-screen bg-gray-950 text-gray-100 p-6">
      <div className="max-w-7xl mx-auto space-y-6">
        <header className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <button
              onClick={() => router.back()}
              className="text-gray-400 hover:text-white"
              aria-label="Volver"
            >
              <ArrowLeft className="w-5 h-5" />
            </button>
            <Gauge className="w-6 h-6 text-blue-400" />
            <div>
              <h1 className="text-2xl font-bold">Dashboard Maity</h1>
              <p className="text-xs text-gray-500">Control milimétrico — refresh 5s · iteraciones {iterations.length}</p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <a
              href="/dev"
              className="text-xs px-3 py-1.5 rounded-md bg-blue-500 hover:bg-blue-600 text-white"
            >
              Ir a /dev (cargar audio)
            </a>
          </div>
        </header>

        <section className="grid md:grid-cols-3 gap-4">
          <SystemPanel />
          <ModelPanel />
          <SummaryKPIs />
        </section>

        <PipelineTimingChart iterations={iterations} />

        <QualityTrendsChart iterations={iterations} />

        <IterationsTable iterations={iterations} onSelect={setSelectedId} />

        <ButtonsMatrix />

        <section className="grid md:grid-cols-2 gap-4">
          <LiveEventsStream />
          <PromptsSummary />
        </section>

        <IterationDetailModal iterationId={selectedId} onClose={() => setSelectedId(null)} />
      </div>
    </div>
  );
}
