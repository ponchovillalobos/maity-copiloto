import { useState } from 'react';
import { motion } from 'framer-motion';
import { Card, CardHeader, CardTitle } from './Card';
import { allPrompts, type FullPrompt } from '../data/prompts';

export function PromptViewer() {
  const [selectedId, setSelectedId] = useState<string>(allPrompts[0]?.id ?? '');
  const selected = allPrompts.find((p) => p.id === selectedId) ?? allPrompts[0];
  const [copied, setCopied] = useState(false);
  const [collapsed, setCollapsed] = useState(false);

  const handleCopy = async () => {
    if (!selected) return;
    await navigator.clipboard.writeText(selected.content);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  if (!selected) {
    return (
      <Card>
        <CardHeader>
          <CardTitle>Prompt Completo</CardTitle>
        </CardHeader>
        <p className="text-xs text-gray-500">No prompts disponibles.</p>
      </Card>
    );
  }

  return (
    <div className="space-y-4">
      {/* Prompt selector (si hay más de uno) */}
      {allPrompts.length > 1 && (
        <div className="flex flex-wrap gap-2">
          {allPrompts.map((p) => (
            <button
              key={p.id}
              onClick={() => setSelectedId(p.id)}
              className={`rounded-md border px-3 py-1.5 text-xs font-medium transition-colors ${
                selectedId === p.id
                  ? 'border-brand-500 bg-brand-500/10 text-white'
                  : 'border-surface-3 bg-surface-2 text-gray-400 hover:border-surface-4'
              }`}
            >
              {p.name}
            </button>
          ))}
        </div>
      )}

      <PromptMetadata prompt={selected} />

      <Card delay={0.1}>
        <CardHeader>
          <div className="flex h-6 w-6 items-center justify-center rounded-md bg-accent-purple/10">
            <svg className="h-3.5 w-3.5 text-accent-purple" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
              <polyline points="14 2 14 8 20 8" />
            </svg>
          </div>
          <CardTitle>Contenido del Prompt</CardTitle>
          <div className="ml-auto flex items-center gap-2">
            <button
              onClick={() => setCollapsed((v) => !v)}
              className="rounded-md border border-surface-3 bg-surface-2 px-3 py-1 text-[11px] font-medium text-gray-400 hover:border-surface-4 hover:text-white transition-colors"
            >
              {collapsed ? 'Expandir' : 'Contraer'}
            </button>
            <button
              onClick={handleCopy}
              className={`rounded-md border px-3 py-1 text-[11px] font-medium transition-colors ${
                copied
                  ? 'border-accent-green bg-accent-green/10 text-accent-green'
                  : 'border-surface-3 bg-surface-2 text-gray-400 hover:border-surface-4 hover:text-white'
              }`}
            >
              {copied ? '✓ Copiado' : '📋 Copiar'}
            </button>
          </div>
        </CardHeader>

        {!collapsed && (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            className="relative max-h-[720px] overflow-auto rounded-lg border border-surface-3 bg-[#0b0b12]"
          >
            <pre className="whitespace-pre-wrap break-words p-4 font-mono text-[11px] leading-relaxed text-gray-300">
              {selected.content}
            </pre>
          </motion.div>
        )}
      </Card>
    </div>
  );
}

function PromptMetadata({ prompt }: { prompt: FullPrompt }) {
  const items = [
    { label: 'Nombre', value: prompt.name, color: 'text-white' },
    { label: 'Versión', value: prompt.version, color: 'text-accent-cyan' },
    { label: 'Modelo', value: prompt.model, color: 'text-accent-purple' },
    { label: 'Tokens (aprox)', value: prompt.tokenCount.toLocaleString(), color: 'text-accent-green' },
    { label: 'Actualizado', value: prompt.lastUpdated, color: 'text-gray-400' },
  ];

  return (
    <Card delay={0.05}>
      <CardHeader>
        <CardTitle>Metadata</CardTitle>
      </CardHeader>

      <p className="mb-4 text-xs text-gray-400">{prompt.description}</p>

      <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-5">
        {items.map((item) => (
          <div
            key={item.label}
            className="rounded-lg border border-surface-3 bg-surface-2/50 p-3"
          >
            <div className="text-[10px] uppercase tracking-wider text-gray-500">
              {item.label}
            </div>
            <div className={`mt-1 font-mono text-xs font-medium ${item.color}`}>
              {item.value}
            </div>
          </div>
        ))}
      </div>
    </Card>
  );
}
