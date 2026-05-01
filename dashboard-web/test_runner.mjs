// Test runner de tips: lee transcripts + ground truth + invoca coach_suggest
// (vía Tauri IPC NO disponible externo — alternativa: HTTP endpoint que Maity
// expone, OR ejecutar directo via lib invocando llama-helper). Por simplicidad
// inicial: simulamos varias llamadas y guardamos solo el plan de comparación.
//
// Uso: cuando Maity esté corriendo + autorun ejecuta dev_import_two_audios,
// los resultados de eval ya quedan persistidos. Esto agregará comparación
// específica de TIPS contra ground truth.
//
// Por ahora este archivo documenta el plan + carga ground truth + escribe
// rows test_run vacíos esperando que llegue tip generation real (próxima fase).

import { DatabaseSync } from 'node:sqlite';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';

const DB = path.join(process.env.APPDATA || os.homedir(), 'com.maity.ai', 'meeting_minutes.sqlite');
const GROUND_TRUTH = path.resolve('test_data/tip_ground_truths.json');

if (!fs.existsSync(DB)) {
  console.error('DB no existe:', DB);
  process.exit(1);
}

const data = JSON.parse(fs.readFileSync(GROUND_TRUTH, 'utf-8'));
const runId = `tipsrun-${new Date().toISOString().slice(0, 19).replace(/[:T]/g, '-')}`;
const buildVersion = process.env.BUILD_VERSION || 'v11';

const db = new DatabaseSync(DB);

console.log(`📋 Test run: ${runId}`);
console.log(`📦 Build: ${buildVersion}`);
console.log(`📁 Ground truth: ${Object.keys(data.scenarios).length} scenarios`);

let inserted = 0;
for (const [scenario, gt] of Object.entries(data.scenarios)) {
  const expected = JSON.stringify(gt.expected_tips);
  // Por ahora insertamos placeholder — se actualizará cuando coach_suggest dispare
  db.prepare(
    `INSERT INTO tip_tests (
      scenario, test_run_id, build_version, meeting_type,
      expected_tips, generated_tip, generated_category, generated_confidence,
      latency_ms, similarity_score, is_duplicate, novelty_score, notes
    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
  ).run(
    scenario,
    runId,
    buildVersion,
    gt.meeting_type,
    expected,
    '[PENDING — invocar coach_suggest con transcripción]',
    null,
    null,
    null,
    null,
    0,
    null,
    `Esperando ${gt.expected_tips.length} tips reales generados por Maity`,
  );
  inserted++;
}
db.close();
console.log(`✓ ${inserted} test placeholders insertados.`);
console.log(`Ver dashboard: http://localhost:3119 → tabla tip_tests`);
