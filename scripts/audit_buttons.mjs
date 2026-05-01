#!/usr/bin/env node
// Source-level button audit. Verifies for each button in button_audit:
// 1. source_file exists
// 2. button is referenced by id-pattern (e.g. data-button-id, onClick handler name)
// 3. invoke command (if any) is registered in lib.rs invoke_handler
// 4. file has no commented-out handler with TODO/FIXME

import { DatabaseSync } from 'node:sqlite';
import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';

const DB = path.join(process.env.APPDATA || os.homedir(), 'com.maity.ai', 'meeting_minutes.sqlite');
const PROJECT_ROOT = 'D:/Proyectos de Kiro/Maity-desktop';

const lib_rs = fs.readFileSync(path.join(PROJECT_ROOT, 'frontend/src-tauri/src/lib.rs'), 'utf-8');
const handler_block = (lib_rs.match(/\.invoke_handler\(tauri::generate_handler!\[([\s\S]+?)\]\)/) || [])[1] || '';

// Map button_id → required invoke command(s) or behavior
const checks = {
  'coach.request_tip':       { invokes: ['coach_suggest'] },
  'coach.tip_next':          { ui_only: true },
  'coach.tip_prev':          { ui_only: true },
  'sidebar.toggle_coach':    { ui_only: true },
  'cmd.dashboard':           { route: '/dashboard' },
  'cmd.export_json':         { ui_only: true },
  'cmd.export_md':           { ui_only: true },
  'cmd.export_pdf':          { invokes: ['export_evaluation_pdf'] },
  'cmd.global_chat':         { invokes: ['coach_chat'] },
  'cmd.new_recording':       { invokes: ['start_recording'] },
  'cmd.open_floating':       { ui_only: true },
  'cmd.semantic_search':     { invokes: ['semantic_search'] },
  'dev.import_audio':        { invokes: ['dev_import_audio_file'] },
  'dev.qa_two_audios':       { invokes: ['dev_import_two_audios'] },
  'eval.compliance_report':  { invokes: ['compliance_export_report'] },
  'eval.export_pdf':         { invokes: ['export_evaluation_pdf'] },
  'eval.generate':           { invokes: ['coach_evaluate_post_meeting'] },
  'sidebar.export_pdf':      { invokes: ['export_evaluation_pdf'] },
  'sidebar.delete_meeting':  { invokes: ['delete_meeting'] },
  'sidebar.new_meeting':     { ui_only: true },
  'sidebar.search':          { route: '/search' },
  'rec.pause':               { invokes: ['pause_recording'] },
  'rec.start':               { invokes: ['start_recording'] },
  'rec.stop':                { invokes: ['stop_recording'] },
  'summary.generate':        { invokes: ['api_process_transcript'] },
  'summary.template_select': { ui_only: true },
};

const db = new DatabaseSync(DB);
const buttons = db.prepare('SELECT id,display_name,source_file,category FROM button_audit ORDER BY category,id').all();
const upd = db.prepare('UPDATE button_audit SET status=?, notes=?, last_checked_at=CURRENT_TIMESTAMP WHERE id=?');

const results = { ok: 0, warn: 0, broken: 0, manual: 0 };

for (const b of buttons) {
  const check = checks[b.id];
  const fpath = path.join(PROJECT_ROOT, b.source_file);
  const fileExists = fs.existsSync(fpath);
  const fileContent = fileExists ? fs.readFileSync(fpath, 'utf-8') : '';
  const fileHasTodo = /TODO|FIXME|@deprecated|\/\/ disabled/i.test(fileContent);

  let status = 'untested', notes = [];

  if (!fileExists) {
    status = 'broken';
    notes.push(`Source file missing: ${b.source_file}`);
  } else if (!check) {
    status = 'untested';
    notes.push('No automated check defined');
  } else if (check.ui_only) {
    status = 'untested';
    notes.push('UI-only — requires manual click test');
    results.manual++;
    upd.run(status, notes.join('; '), b.id);
    continue;
  } else if (check.invokes) {
    const missing = check.invokes.filter(cmd => !handler_block.includes(cmd));
    if (missing.length === 0) {
      status = 'ok';
      notes.push(`invoke commands registered: ${check.invokes.join(', ')}`);
    } else {
      status = 'broken';
      notes.push(`Missing in lib.rs invoke_handler: ${missing.join(', ')}`);
    }
  } else if (check.route) {
    const routePath = path.join(PROJECT_ROOT, 'frontend/src/app', check.route, 'page.tsx');
    if (fs.existsSync(routePath)) {
      status = 'ok';
      notes.push(`Route page exists: ${check.route}/page.tsx`);
    } else {
      status = 'broken';
      notes.push(`Route page missing: ${check.route}`);
    }
  }

  if (fileHasTodo && status === 'ok') {
    notes.push('Has TODO/FIXME in file (review manually)');
  }

  upd.run(status, notes.join('; '), b.id);
  results[status === 'ok' ? 'ok' : (status === 'broken' ? 'broken' : 'warn')]++;
}

db.close();

console.log('=== AUDIT RESULTS ===');
console.log('  ok      :', results.ok);
console.log('  broken  :', results.broken);
console.log('  warn    :', results.warn);
console.log('  manual  :', results.manual, '(UI-only, requires user click)');
console.log('Total:', buttons.length);
