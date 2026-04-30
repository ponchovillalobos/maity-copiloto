'use client';

import React from 'react';
import { CheckCircle, Sparkles } from 'lucide-react';

/**
 * Panel informativo (read-only) sobre la IA local activa.
 * Sin botón de descarga ni nombres técnicos de modelos. Los modelos vienen
 * pre-instalados con la app y se mantienen actualizados en background.
 */
export function AIInfoPanel() {
  return (
    <div className="space-y-6 text-gray-100">
      <div className="rounded-xl border border-white/10 bg-white/5 p-6">
        <div className="flex items-start gap-4">
          <div className="rounded-full p-3 bg-[#485df4]/20 border border-[#485df4]/40">
            <Sparkles className="w-6 h-6 text-[#a8b3ff]" />
          </div>
          <div className="flex-1">
            <div className="flex items-center gap-2">
              <h3 className="text-base font-semibold text-gray-50">IA Local Activa</h3>
              <span className="flex items-center gap-1 text-xs text-emerald-400">
                <CheckCircle className="w-3.5 h-3.5" />
                Lista para usar
              </span>
            </div>
            <p className="text-sm text-gray-300 mt-2 leading-relaxed">
              Maity usa un motor de IA <strong className="text-gray-100">100% local</strong>. No
              necesita internet ni servidores externos. Tus reuniones nunca salen de tu computadora.
            </p>
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        <div className="rounded-lg border border-white/10 bg-white/5 p-4">
          <div className="text-[10px] uppercase tracking-wider text-gray-400 mb-1">Tips en vivo</div>
          <div className="text-sm font-semibold text-gray-50">Sugerencias rápidas</div>
          <div className="text-xs text-gray-400 mt-1">Mientras hablas, sin lag.</div>
        </div>
        <div className="rounded-lg border border-white/10 bg-white/5 p-4">
          <div className="text-[10px] uppercase tracking-wider text-gray-400 mb-1">Evaluación post-reunión</div>
          <div className="text-sm font-semibold text-gray-50">Análisis profundo</div>
          <div className="text-xs text-gray-400 mt-1">Resumen + recomendaciones.</div>
        </div>
        <div className="rounded-lg border border-white/10 bg-white/5 p-4">
          <div className="text-[10px] uppercase tracking-wider text-gray-400 mb-1">Transcripción</div>
          <div className="text-sm font-semibold text-gray-50">Español nativo</div>
          <div className="text-xs text-gray-400 mt-1">Alta precisión sin internet.</div>
        </div>
        <div className="rounded-lg border border-white/10 bg-white/5 p-4">
          <div className="text-[10px] uppercase tracking-wider text-gray-400 mb-1">Privacidad</div>
          <div className="text-sm font-semibold text-gray-50">Cero datos enviados</div>
          <div className="text-xs text-gray-400 mt-1">Todo procesado en tu equipo.</div>
        </div>
      </div>
    </div>
  );
}

export default AIInfoPanel;
