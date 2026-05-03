/**
 * Metadata de tips compartida entre CoachPanel (panel principal) y FloatingPage (burbuja).
 * Garantiza que las etiquetas (categoría + prioridad) sean idénticas en ambos lugares.
 */

export interface CategoryMeta {
  /** Texto en español que se muestra al usuario. */
  label: string;
  /** Color hex (para la burbuja flotante; el panel usa clases Tailwind separadas). */
  hex: string;
  /** Color clase Tailwind para el panel. */
  textClass: string;
  /** Background + border combinados para el panel. */
  bgClass: string;
  /** Emoji/ícono visual del v32.0 (opcional para legacy). */
  icon?: string;
}

export interface PriorityMeta {
  /** Texto que el usuario lee. */
  label: string;
  /** Hex color para la burbuja. */
  hex: string;
  /** Emoji que representa la prioridad. */
  emoji: string;
  /** Clase Tailwind para badges del panel. */
  badgeClass: string;
}

export const CATEGORY_META: Record<string, CategoryMeta> = {
  // v32.0: 6 categorías operativas con ícono + color del sistema MAITY.
  RESPIRA:  { label: 'RESPIRA',  hex: '#FF4757', textClass: 'text-red-400',    bgClass: 'bg-red-500/10 border-red-500/40',     icon: '🫁' },
  PAUSA:    { label: 'PAUSA',    hex: '#FFA502', textClass: 'text-amber-400',  bgClass: 'bg-amber-500/10 border-amber-500/40', icon: '✋' },
  PREGUNTA: { label: 'PREGUNTA', hex: '#00C2FF', textClass: 'text-cyan-300',   bgClass: 'bg-cyan-500/10 border-cyan-500/40',   icon: '❓' },
  ESCUCHA:  { label: 'ESCUCHA',  hex: '#8E44FF', textClass: 'text-purple-300', bgClass: 'bg-purple-500/10 border-purple-500/40', icon: '👂' },
  VALIDA:   { label: 'VALIDA',   hex: '#1BEA9A', textClass: 'text-emerald-300', bgClass: 'bg-emerald-500/10 border-emerald-500/40', icon: '💚' },
  AVANZA:   { label: 'AVANZA',   hex: '#FF0050', textClass: 'text-pink-400',   bgClass: 'bg-pink-500/10 border-pink-500/40',   icon: '➡️' },
  // Legacy v31.x
  icebreaker:    { label: 'Romper hielo',  hex: '#fde047', textClass: 'text-yellow-300',  bgClass: 'bg-yellow-500/10 border-yellow-500/40' },
  discovery:     { label: 'Descubrir',     hex: '#67e8f9', textClass: 'text-cyan-300',    bgClass: 'bg-cyan-500/10 border-cyan-500/40' },
  question:      { label: 'Pregunta',      hex: '#93c5fd', textClass: 'text-blue-300',    bgClass: 'bg-blue-500/10 border-blue-500/40' },
  objection:     { label: 'Objeción',      hex: '#fdba74', textClass: 'text-orange-300',  bgClass: 'bg-orange-500/10 border-orange-500/40' },
  closing:       { label: 'Cierre',        hex: '#86efac', textClass: 'text-green-300',   bgClass: 'bg-green-500/10 border-green-500/40' },
  pacing:        { label: 'Ritmo',         hex: '#d8b4fe', textClass: 'text-purple-300',  bgClass: 'bg-purple-500/10 border-purple-500/40' },
  rapport:       { label: 'Rapport',       hex: '#f9a8d4', textClass: 'text-pink-300',    bgClass: 'bg-pink-500/10 border-pink-500/40' },
  persuasion:    { label: 'Persuasión',    hex: '#a5b4fc', textClass: 'text-indigo-300',  bgClass: 'bg-indigo-500/10 border-indigo-500/40' },
  service:       { label: 'Servicio',      hex: '#fca5a5', textClass: 'text-red-300',     bgClass: 'bg-red-500/10 border-red-500/40' },
  negotiation:   { label: 'Negociación',   hex: '#fcd34d', textClass: 'text-amber-300',   bgClass: 'bg-amber-500/10 border-amber-500/40' },
  self_control:  { label: 'Autocontrol',   hex: '#fca5a5', textClass: 'text-red-300',     bgClass: 'bg-red-500/10 border-red-500/40' },
  listening:     { label: 'Escuchando',    hex: '#a8b3ff', textClass: 'text-blue-300',    bgClass: 'bg-blue-500/10 border-blue-500/40' },
};

const FALLBACK_CATEGORY: CategoryMeta = CATEGORY_META.pacing;

export function categoryMeta(category?: string): CategoryMeta {
  if (!category) return FALLBACK_CATEGORY;
  return CATEGORY_META[category] ?? FALLBACK_CATEGORY;
}

export const PRIORITY_META: Record<string, PriorityMeta> = {
  critical:  { label: 'Crítico',     hex: '#ff0050', emoji: '🔴', badgeClass: 'bg-red-500/20 text-red-300 border-red-500/40' },
  important: { label: 'Importante',  hex: '#f59e0b', emoji: '🟡', badgeClass: 'bg-yellow-500/20 text-yellow-300 border-yellow-500/40' },
  soft:      { label: 'Sugerencia',  hex: '#1bea9a', emoji: '🟢', badgeClass: 'bg-green-500/20 text-green-300 border-green-500/40' },
  // Aliases legacy: el LLM a veces devuelve high/medium/low; las mapeamos al
  // mismo set canónico para que panel y burbuja muestren la MISMA etiqueta.
  high:      { label: 'Crítico',     hex: '#ff0050', emoji: '🔴', badgeClass: 'bg-red-500/20 text-red-300 border-red-500/40' },
  medium:    { label: 'Importante',  hex: '#f59e0b', emoji: '🟡', badgeClass: 'bg-yellow-500/20 text-yellow-300 border-yellow-500/40' },
  low:       { label: 'Sugerencia',  hex: '#1bea9a', emoji: '🟢', badgeClass: 'bg-green-500/20 text-green-300 border-green-500/40' },
};

const FALLBACK_PRIORITY: PriorityMeta = PRIORITY_META.soft;

export function priorityMeta(priority?: string): PriorityMeta {
  if (!priority) return FALLBACK_PRIORITY;
  return PRIORITY_META[priority] ?? FALLBACK_PRIORITY;
}
