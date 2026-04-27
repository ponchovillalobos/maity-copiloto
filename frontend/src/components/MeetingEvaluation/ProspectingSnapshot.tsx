"use client";

import React, { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { motion, AnimatePresence } from "framer-motion";
import { Copy, Loader, AlertCircle, TrendingUp, Users, Target, Calendar } from "lucide-react";

interface EmailDraft {
  asunto: string;
  saludo: string;
  cuerpo: string;
  cierre: string;
}

interface ProximoPaso {
  accion: string;
  responsable?: string | null;
  fecha?: string | null;
}

interface ProspectingSnapshot {
  email_draft: EmailDraft;
  objeciones_detectadas: string[];
  competidores_mencionados: string[];
  dolores_detectados: string[];
  proximos_pasos: ProximoPaso[];
  nivel_interes_estimado: string;
  razon_nivel: string;
}

interface ProspectingResult {
  meeting_id: string;
  snapshot: ProspectingSnapshot;
  model: string;
  latency_ms: number;
}

interface ProspectingSnapshotProps {
  meetingId: string;
}

export const ProspectingSnapshot: React.FC<ProspectingSnapshotProps> = ({
  meetingId,
}) => {
  const [result, setResult] = useState<ProspectingResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copyFeedback, setCopyFeedback] = useState(false);

  const generateSnapshot = async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await invoke<ProspectingResult>(
        "generate_prospecting_snapshot",
        {
          meeting_id: meetingId,
          model: undefined,
        }
      );
      setResult(res);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg);
    } finally {
      setLoading(false);
    }
  };

  const copyEmailToClipboard = async () => {
    if (!result) return;
    const { email_draft } = result.snapshot;
    const emailText = `Asunto: ${email_draft.asunto}

${email_draft.saludo}

${email_draft.cuerpo}

${email_draft.cierre}`;

    try {
      await navigator.clipboard.writeText(emailText);
      setCopyFeedback(true);
      setTimeout(() => setCopyFeedback(false), 2000);
    } catch (err) {
      console.error("Error copying:", err);
    }
  };

  const renderLevel = (level: string) => {
    const color =
      level === "alto"
        ? "bg-green-500/20 text-green-700 dark:text-green-400 border-green-500/30"
        : level === "medio"
          ? "bg-amber-500/20 text-amber-700 dark:text-amber-400 border-amber-500/30"
          : "bg-red-500/20 text-red-700 dark:text-red-400 border-red-500/30";

    return (
      <span
        className={`px-3 py-1 rounded-full text-sm font-medium border ${color}`}
      >
        {level.charAt(0).toUpperCase() + level.slice(1)}
      </span>
    );
  };

  if (!result && !loading) {
    return (
      <div className="flex flex-col items-center justify-center py-12 px-6">
        <AlertCircle className="w-16 h-16 text-muted-foreground/40 mb-4" />
        <p className="text-center text-muted-foreground mb-6">
          Genera un análisis de prospecting automático basado en la
          transcripción.
        </p>
        <motion.button
          whileHover={{ scale: 1.02 }}
          whileTap={{ scale: 0.98 }}
          onClick={generateSnapshot}
          disabled={loading}
          className="px-6 py-3 bg-gradient-to-r from-blue-600 to-cyan-600 text-white rounded-lg font-medium shadow-lg hover:shadow-xl disabled:opacity-50"
        >
          Generar Email + Análisis
        </motion.button>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="flex flex-col items-center justify-center py-12">
        <Loader className="w-8 h-8 animate-spin text-blue-500 mb-4" />
        <p className="text-muted-foreground">Analizando reunión...</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-red-500/10 border border-red-500/30 rounded-lg p-4">
        <p className="text-red-700 dark:text-red-400 font-medium">Error</p>
        <p className="text-red-600/80 dark:text-red-400/70 text-sm mt-1">
          {error}
        </p>
      </div>
    );
  }

  if (!result) return null;

  const { snapshot } = result;
  const { email_draft } = snapshot;

  return (
    <motion.div
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      className="space-y-6"
    >
      {/* Email Draft Section */}
      <div className="bg-gradient-to-br from-blue-500/10 via-cyan-500/5 to-blue-500/5 border border-blue-500/20 rounded-xl p-6 backdrop-blur-sm">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-semibold text-foreground">
            Email Draft
          </h3>
          <motion.button
            whileHover={{ scale: 1.05 }}
            whileTap={{ scale: 0.95 }}
            onClick={copyEmailToClipboard}
            className="p-2 rounded-lg hover:bg-blue-500/20 transition-colors"
            title="Copiar email"
          >
            <Copy className="w-5 h-5 text-blue-600 dark:text-blue-400" />
          </motion.button>
        </div>

        {copyFeedback && (
          <div className="text-sm text-green-600 dark:text-green-400 mb-3">
            ✓ Email copiado
          </div>
        )}

        <div className="space-y-4">
          <div>
            <label className="text-xs font-semibold text-muted-foreground uppercase">
              Asunto
            </label>
            <p className="text-foreground font-medium mt-1">{email_draft.asunto}</p>
          </div>

          <div>
            <label className="text-xs font-semibold text-muted-foreground uppercase">
              Saludo
            </label>
            <p className="text-foreground mt-1">{email_draft.saludo}</p>
          </div>

          <div>
            <label className="text-xs font-semibold text-muted-foreground uppercase">
              Cuerpo
            </label>
            <div className="text-foreground/90 mt-1 space-y-2 text-sm leading-relaxed">
              {email_draft.cuerpo.split("\n").map((para, i) => (
                <p key={i}>{para}</p>
              ))}
            </div>
          </div>

          <div>
            <label className="text-xs font-semibold text-muted-foreground uppercase">
              Cierre
            </label>
            <p className="text-foreground mt-1">{email_draft.cierre}</p>
          </div>
        </div>
      </div>

      {/* Insights Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* Interest Level */}
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.1 }}
          className="bg-gradient-to-br from-emerald-500/10 to-emerald-500/5 border border-emerald-500/20 rounded-lg p-4 backdrop-blur-sm"
        >
          <div className="flex items-center gap-2 mb-3">
            <TrendingUp className="w-5 h-5 text-emerald-600 dark:text-emerald-400" />
            <h4 className="font-semibold text-foreground">Nivel Interés</h4>
          </div>
          <div className="space-y-2">
            <div>{renderLevel(snapshot.nivel_interes_estimado)}</div>
            <p className="text-sm text-muted-foreground">
              {snapshot.razon_nivel}
            </p>
          </div>
        </motion.div>

        {/* Next Steps Count */}
        {snapshot.proximos_pasos.length > 0 && (
          <motion.div
            initial={{ opacity: 0, y: 10 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ delay: 0.2 }}
            className="bg-gradient-to-br from-violet-500/10 to-violet-500/5 border border-violet-500/20 rounded-lg p-4 backdrop-blur-sm"
          >
            <div className="flex items-center gap-2 mb-3">
              <Calendar className="w-5 h-5 text-violet-600 dark:text-violet-400" />
              <h4 className="font-semibold text-foreground">Próximos Pasos</h4>
            </div>
            <p className="text-2xl font-bold text-violet-600 dark:text-violet-400">
              {snapshot.proximos_pasos.length}
            </p>
          </motion.div>
        )}
      </div>

      {/* Objections */}
      {snapshot.objeciones_detectadas.length > 0 && (
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.3 }}
          className="bg-red-500/10 border border-red-500/20 rounded-lg p-4 backdrop-blur-sm"
        >
          <h4 className="font-semibold text-red-700 dark:text-red-400 mb-3">
            Objeciones Detectadas
          </h4>
          <ul className="space-y-2">
            {snapshot.objeciones_detectadas.map((obj, i) => (
              <li
                key={i}
                className="text-sm text-red-600/80 dark:text-red-400/70 flex items-start gap-2"
              >
                <span className="text-red-500 mt-1 font-bold">•</span>
                <span>{obj}</span>
              </li>
            ))}
          </ul>
        </motion.div>
      )}

      {/* Competitors */}
      {snapshot.competidores_mencionados.length > 0 && (
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.4 }}
          className="bg-blue-500/10 border border-blue-500/20 rounded-lg p-4 backdrop-blur-sm"
        >
          <div className="flex items-center gap-2 mb-3">
            <Users className="w-5 h-5 text-blue-600 dark:text-blue-400" />
            <h4 className="font-semibold text-blue-700 dark:text-blue-400">
              Competidores Mencionados
            </h4>
          </div>
          <ul className="space-y-2">
            {snapshot.competidores_mencionados.map((comp, i) => (
              <li
                key={i}
                className="text-sm text-blue-600/80 dark:text-blue-400/70 flex items-start gap-2"
              >
                <span className="text-blue-500 mt-1 font-bold">•</span>
                <span>{comp}</span>
              </li>
            ))}
          </ul>
        </motion.div>
      )}

      {/* Pain Points */}
      {snapshot.dolores_detectados.length > 0 && (
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.5 }}
          className="bg-amber-500/10 border border-amber-500/20 rounded-lg p-4 backdrop-blur-sm"
        >
          <div className="flex items-center gap-2 mb-3">
            <Target className="w-5 h-5 text-amber-600 dark:text-amber-400" />
            <h4 className="font-semibold text-amber-700 dark:text-amber-400">
              Dolores Detectados
            </h4>
          </div>
          <ul className="space-y-2">
            {snapshot.dolores_detectados.map((dolor, i) => (
              <li
                key={i}
                className="text-sm text-amber-600/80 dark:text-amber-400/70 flex items-start gap-2"
              >
                <span className="text-amber-500 mt-1 font-bold">•</span>
                <span>{dolor}</span>
              </li>
            ))}
          </ul>
        </motion.div>
      )}

      {/* Next Steps Details */}
      {snapshot.proximos_pasos.length > 0 && (
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.6 }}
          className="bg-green-500/10 border border-green-500/20 rounded-lg p-4 backdrop-blur-sm"
        >
          <h4 className="font-semibold text-green-700 dark:text-green-400 mb-3">
            Acciones Comprometidas
          </h4>
          <ul className="space-y-3">
            {snapshot.proximos_pasos.map((paso, i) => (
              <li
                key={i}
                className="text-sm bg-green-500/5 p-3 rounded border border-green-500/20"
              >
                <p className="text-green-700 dark:text-green-400 font-medium">
                  {paso.accion}
                </p>
                <div className="flex gap-4 mt-2 text-xs text-green-600/70 dark:text-green-400/70">
                  {paso.responsable && (
                    <span>👤 {paso.responsable}</span>
                  )}
                  {paso.fecha && <span>📅 {paso.fecha}</span>}
                </div>
              </li>
            ))}
          </ul>
        </motion.div>
      )}

      {/* Metadata Footer */}
      <div className="text-xs text-muted-foreground text-center pt-4 border-t border-border">
        Generado con {result.model} en {result.latency_ms}ms
      </div>
    </motion.div>
  );
};
