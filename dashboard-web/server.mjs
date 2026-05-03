// Dashboard web local — lee SQLite de Maity y sirve vista en http://localhost:3119
// Sin dependencias externas: usa node:sqlite (Node 22+) y node:http nativo.
//
// Lanzar: node dashboard-web/server.mjs
// Browser: http://localhost:3119

import { DatabaseSync } from 'node:sqlite';
import http from 'node:http';
import os from 'node:os';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const PORT = Number(process.env.MAITY_DASHBOARD_PORT) || 3119;
const DB_PATH = path.join(process.env.APPDATA || os.homedir(), 'com.maity.ai', 'meeting_minutes.sqlite');

if (!fs.existsSync(DB_PATH)) {
  console.error(`[dashboard-web] No encontré DB en ${DB_PATH}`);
  console.error('Asegurate que Maity haya corrido al menos una vez.');
  process.exit(1);
}

function openDb() {
  return new DatabaseSync(DB_PATH, { readOnly: true });
}

function safeRows(sql, params = []) {
  try {
    const db = openDb();
    const stmt = db.prepare(sql);
    const rows = stmt.all(...params);
    db.close();
    return rows;
  } catch (e) {
    console.error('[dashboard-web] DB error:', e.message);
    return [];
  }
}

function safeOne(sql, params = []) {
  try {
    const db = openDb();
    const stmt = db.prepare(sql);
    const row = stmt.get(...params);
    db.close();
    return row;
  } catch (e) {
    console.error('[dashboard-web] DB error:', e.message);
    return null;
  }
}

function jsonReply(res, body) {
  res.writeHead(200, { 'Content-Type': 'application/json; charset=utf-8' });
  res.end(JSON.stringify(body));
}

const HTML = fs.readFileSync(path.join(__dirname, 'index.html'), 'utf-8');

const server = http.createServer((req, res) => {
  const url = new URL(req.url, `http://localhost:${PORT}`);

  if (url.pathname === '/' || url.pathname === '/index.html') {
    res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
    res.end(HTML);
    return;
  }

  // /lab: pagina dedicada Prompt Lab
  if (url.pathname === '/lab') {
    const labHtmlPath = path.join(__dirname, 'lab.html');
    if (fs.existsSync(labHtmlPath)) {
      res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
      res.end(fs.readFileSync(labHtmlPath, 'utf-8'));
      return;
    }
  }

  // /api/prompt_current: lee prompt activo desde commands.rs
  if (url.pathname === '/api/prompt_current') {
    const commandsPath = path.join(__dirname, '..', 'frontend', 'src-tauri', 'src', 'coach', 'commands.rs');
    try {
      const content = fs.readFileSync(commandsPath, 'utf-8');
      const sysMatch = content.match(/let system_prompt = "([^"]+)"/);
      const userBlock = content.match(/let user_prompt = format!\(\s*"([\s\S]*?)",\s*window_capped/);
      const versionMatch = content.match(/v31\.\d+:?\s+prompt/);
      jsonReply(res, {
        version: versionMatch ? versionMatch[0] : 'unknown',
        system_prompt: sysMatch ? sysMatch[1].replace(/\\n/g, '\n') : null,
        user_prompt_template: userBlock ? userBlock[1].replace(/\\"/g, '"').replace(/\\n/g, '\n') : null,
      });
    } catch (e) {
      jsonReply(res, { error: e.message });
    }
    return;
  }

  // /api/prompt_lab: runs del lab
  if (url.pathname === '/api/prompt_lab') {
    const limit = Math.min(2000, Number(url.searchParams.get('limit')) || 500);
    const runId = url.searchParams.get('run_id');
    const rows = runId
      ? safeRows(
          `SELECT * FROM prompt_lab_runs WHERE run_id = ? ORDER BY fixture_name, window_idx LIMIT ?`,
          [runId, limit]
        )
      : safeRows(
          `SELECT * FROM prompt_lab_runs ORDER BY created_at DESC LIMIT ?`,
          [limit]
        );
    // Lista de runs unicos
    const runs = safeRows(
      `SELECT run_id, prompt_version, COUNT(*) as total, SUM(passed) as passed,
              MIN(created_at) as started_at,
              ROUND(AVG(latency_ms)) as avg_latency
       FROM prompt_lab_runs GROUP BY run_id ORDER BY started_at DESC LIMIT 50`
    );
    jsonReply(res, { rows, runs });
    return;
  }

  if (url.pathname === '/api/iterations') {
    const limit = Math.min(500, Number(url.searchParams.get('limit')) || 100);
    const rows = safeRows(
      `SELECT id, meeting_id, iteration_label, channel_layout, total_duration_seconds,
              decode_ms, transcribe_user_ms, transcribe_interlocutor_ms,
              evaluation_ms, total_pipeline_ms,
              wer_global, wer_user, wer_interlocutor,
              evaluation_score, evaluation_sections_filled,
              prompt_version, coach_model, evaluation_model, created_at
       FROM dev_iterations
       ORDER BY created_at DESC
       LIMIT ?`,
      [limit],
    );
    return jsonReply(res, rows);
  }

  if (url.pathname.startsWith('/api/iteration/')) {
    const id = Number(url.pathname.split('/').pop());
    const row = safeOne(
      `SELECT * FROM dev_iterations WHERE id = ?`,
      [id],
    );
    return jsonReply(res, row);
  }

  if (url.pathname === '/api/summary') {
    const total = safeOne('SELECT COUNT(*) AS c FROM dev_iterations')?.c ?? 0;
    const last7d = safeOne(
      "SELECT COUNT(*) AS c FROM dev_iterations WHERE created_at >= datetime('now', '-7 days')",
    )?.c ?? 0;
    const avgWerUser = safeOne(
      'SELECT AVG(wer_user) AS v FROM dev_iterations WHERE wer_user IS NOT NULL',
    )?.v;
    const avgWerInter = safeOne(
      'SELECT AVG(wer_interlocutor) AS v FROM dev_iterations WHERE wer_interlocutor IS NOT NULL',
    )?.v;
    const avgScore = safeOne(
      'SELECT AVG(evaluation_score) AS v FROM dev_iterations WHERE evaluation_score IS NOT NULL',
    )?.v;
    const avgPipeline = safeOne(
      'SELECT AVG(total_pipeline_ms) AS v FROM dev_iterations WHERE total_pipeline_ms IS NOT NULL',
    )?.v;
    const lastAt = safeOne('SELECT MAX(created_at) AS v FROM dev_iterations')?.v;
    const lastRun = safeOne(
      'SELECT iteration_label, wer_user, wer_interlocutor, total_pipeline_ms, created_at FROM dev_iterations ORDER BY created_at DESC LIMIT 1',
    );
    return jsonReply(res, {
      total,
      last_7d: last7d,
      avg_wer_user: avgWerUser,
      avg_wer_interlocutor: avgWerInter,
      avg_evaluation_score: avgScore,
      avg_pipeline_ms: avgPipeline,
      last_iteration_at: lastAt,
      last_run: lastRun,
    });
  }

  if (url.pathname === '/api/tip_tests') {
    const limit = Math.min(500, Number(url.searchParams.get('limit')) || 200);
    const rows = safeRows(
      `SELECT id, scenario, test_run_id, build_version, meeting_type,
              expected_tips, generated_tip, generated_category, generated_confidence,
              latency_ms, similarity_score, is_duplicate, novelty_score, notes, created_at
       FROM tip_tests ORDER BY created_at DESC LIMIT ?`,
      [limit],
    );
    return jsonReply(res, rows);
  }

  if (url.pathname === '/api/evaluations') {
    const limit = Math.min(500, Number(url.searchParams.get('limit')) || 100);
    const rows = safeRows(
      `SELECT meeting_id, puntuacion_global, nivel, prompt_version, model_used,
              duration_minutes, created_at
       FROM meeting_evaluations ORDER BY created_at DESC LIMIT ?`,
      [limit],
    );
    return jsonReply(res, rows);
  }

  if (url.pathname === '/api/improvements') {
    const limit = Math.min(200, Number(url.searchParams.get('limit')) || 100);
    const rows = safeRows(
      `SELECT id, iteration_label, category, title, description, files_changed,
              before_metric, after_metric, build_hash, created_at
       FROM improvements ORDER BY created_at DESC LIMIT ?`,
      [limit],
    );
    return jsonReply(res, rows);
  }

  if (url.pathname === '/api/buttons') {
    const rows = safeRows(
      'SELECT id, display_name, source_file, source_line, category, status, notes, last_checked_at FROM button_audit ORDER BY status DESC, category, display_name',
    );
    return jsonReply(res, rows);
  }

  if (url.pathname === '/api/logs') {
    const logFile = process.env.MAITY_LOG_FILE || '/tmp/maity-app.log';
    const lines = Math.min(500, Number(url.searchParams.get('lines')) || 200);
    try {
      const data = fs.readFileSync(logFile, 'utf-8');
      const all = data.split('\n').filter(Boolean);
      return jsonReply(res, { lines: all.slice(-lines), total: all.length, file: logFile });
    } catch (e) {
      return jsonReply(res, { lines: [], total: 0, error: e.message, file: logFile });
    }
  }

  if (url.pathname === '/api/tips_live') {
    // v26.1: histórico PERMANENTE de tips reales (coach_tips_log).
    // Antes leía tip_tests (solo tests) — ahora lee tips de PRODUCCIÓN.
    // Default: TODOS los tips (sin filtro de tiempo) — preserva histórico.
    // Opcional ?minutes=N para filtrar.
    const minutesAgo = url.searchParams.has('minutes') ? Number(url.searchParams.get('minutes')) : null;
    const limit = Math.min(500, Number(url.searchParams.get('limit')) || 100);
    const where = minutesAgo
      ? `WHERE created_at >= datetime('now', '-' || ${minutesAgo} || ' minutes')`
      : '';
    const rows = safeRows(
      `SELECT id, meeting_id, tip, category, subcategory, technique,
              priority, tip_type, confidence, latency_ms, model, minute,
              trigger_signal, suggested_category, is_duplicate, created_at
       FROM coach_tips_log ${where} ORDER BY id DESC LIMIT ?`,
      [limit],
    );
    const stats = (() => {
      const lats = rows.map(r => r.latency_ms).filter(Boolean);
      return {
        count: rows.length,
        avg_latency_ms: lats.length ? Math.round(lats.reduce((a,b)=>a+b,0)/lats.length) : 0,
        max_latency_ms: lats.length ? Math.max(...lats) : 0,
        min_latency_ms: lats.length ? Math.min(...lats) : 0,
        slow_count: lats.filter(l => l > 5000).length,
        dup_count: rows.filter(r => r.is_duplicate === 1).length,
        models: [...new Set(rows.map(r => r.model).filter(Boolean))],
        unique_tips: new Set(rows.map(r => r.tip)).size,
      };
    })();
    return jsonReply(res, { rows, stats });
  }

  if (url.pathname === '/api/runtime') {
    // v23: lee snapshot escrito por Rust system_monitor cada 1s.
    // Incluye is_recording flag + CPU/RAM live + warnings de umbrales.
    const runtimePath = path.join(process.env.APPDATA || os.homedir(), 'com.maity.ai', 'runtime.json');
    try {
      const raw = fs.readFileSync(runtimePath, 'utf-8');
      const data = JSON.parse(raw);
      const stale = (Date.now() - (data.ts || 0)) > 5000;
      const ramPct = data.ram_total_mb ? Math.round((data.ram_used_mb / data.ram_total_mb) * 100) : 0;
      const warnings = [];
      if (data.cpu_pct > 85) warnings.push(`CPU CRÍTICO: ${data.cpu_pct.toFixed(0)}%`);
      else if (data.cpu_pct > 70) warnings.push(`CPU alto: ${data.cpu_pct.toFixed(0)}%`);
      if (ramPct > 90) warnings.push(`RAM CRÍTICA: ${ramPct}%`);
      else if (ramPct > 80) warnings.push(`RAM alta: ${ramPct}%`);
      if (data.process_ram_mb > 4000) warnings.push(`Maity RAM proceso ${data.process_ram_mb}MB`);
      return jsonReply(res, { ...data, ram_pct: ramPct, stale, warnings });
    } catch (e) {
      return jsonReply(res, { error: 'Maity no está corriendo o snapshot no disponible', stale: true });
    }
  }

  if (url.pathname === '/api/system') {
    const cpus = os.cpus();
    const total = os.totalmem();
    const free = os.freemem();
    return jsonReply(res, {
      cpu_count: cpus.length,
      cpu_model: cpus[0]?.model || 'unknown',
      ram_used_gb: ((total - free) / (1024 ** 3)).toFixed(2),
      ram_total_gb: (total / (1024 ** 3)).toFixed(2),
      ram_pct: Math.round(((total - free) / total) * 100),
      uptime_hours: (os.uptime() / 3600).toFixed(1),
      platform: `${os.platform()} ${os.release()}`,
      hostname: os.hostname(),
    });
  }

  res.writeHead(404, { 'Content-Type': 'text/plain' });
  res.end('Not found');
});

server.listen(PORT, () => {
  console.log(`\n  ✓ Maity Dashboard Web`);
  console.log(`  → http://localhost:${PORT}`);
  console.log(`  DB: ${DB_PATH}\n`);
});
