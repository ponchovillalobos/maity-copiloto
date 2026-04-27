'use client';

import React, { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Sparkles, PictureInPicture2, MessageCircleMore, X, ChevronRight } from 'lucide-react';

type SlideIndex = 0 | 1 | 2;

const TOUR_COMPLETED_KEY = 'maity_v0.4_tour_completed';

/**
 * TourOverlay — Tour visual de onboarding v0.4.0
 *
 * Muestra 3 slides sobre las features clave:
 * 1. Coach IA en vivo
 * 2. Ventana flotante always-on-top
 * 3. Chat global con historial
 *
 * Persistencia vía localStorage. Se puede reactivar manualmente via window event 'open-v04-tour'.
 */
export function TourOverlay() {
  const [currentSlide, setCurrentSlide] = useState<SlideIndex>(0);
  const [isVisible, setIsVisible] = useState(false);

  // Solo cargar localStorage en cliente
  useEffect(() => {
    const completed = localStorage.getItem(TOUR_COMPLETED_KEY);
    if (!completed) {
      setIsVisible(true);
    }
  }, []);

  // Escuchar evento global para reabrir el tour
  useEffect(() => {
    const handleOpenTour = () => {
      setIsVisible(true);
      setCurrentSlide(0);
      localStorage.removeItem(TOUR_COMPLETED_KEY);
    };
    window.addEventListener('open-v04-tour', handleOpenTour);
    return () => window.removeEventListener('open-v04-tour', handleOpenTour);
  }, []);

  const handleNext = () => {
    if (currentSlide < 2) {
      setCurrentSlide((currentSlide + 1) as SlideIndex);
    } else {
      // Último slide - completar tour
      completeTour();
    }
  };

  const handleSkip = () => {
    completeTour();
  };

  const completeTour = () => {
    localStorage.setItem(TOUR_COMPLETED_KEY, 'true');
    setIsVisible(false);
  };

  if (!isVisible) return null;

  return (
    <AnimatePresence>
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        exit={{ opacity: 0 }}
        transition={{ duration: 0.2 }}
        className="fixed inset-0 z-[300] flex items-center justify-center"
        style={{
          background: 'rgba(0, 0, 0, 0.7)',
          backdropFilter: 'blur(4px)',
        }}
      >
        <motion.div
          initial={{ opacity: 0, scale: 0.95, y: 10 }}
          animate={{ opacity: 1, scale: 1, y: 0 }}
          exit={{ opacity: 0, scale: 0.95 }}
          transition={{ duration: 0.2 }}
          className="relative w-full max-w-lg mx-4"
          style={{
            background: 'rgba(15, 16, 24, 0.95)',
            backdropFilter: 'blur(20px)',
            border: '1px solid rgba(255, 255, 255, 0.14)',
            borderRadius: '16px',
          }}
        >
          {/* Skip button - esquina superior derecha */}
          <button
            onClick={handleSkip}
            className="absolute top-4 right-4 p-1.5 text-gray-400 hover:text-gray-200 transition-colors rounded-lg hover:bg-white/5"
            aria-label="Saltar tour"
          >
            <X className="w-5 h-5" />
          </button>

          {/* Contenido del slide */}
          <div className="p-8 pt-12 pb-6 text-center">
            <AnimatePresence mode="wait">
              {currentSlide === 0 && (
                <motion.div
                  key="slide-0"
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: -10 }}
                  transition={{ duration: 0.3 }}
                >
                  <div className="flex justify-center mb-6">
                    <motion.div
                      animate={{ y: [0, -4, 0] }}
                      transition={{ duration: 2, repeat: Infinity }}
                    >
                      <Sparkles className="w-16 h-16 text-[#485df4]" />
                    </motion.div>
                  </div>
                  <h2 className="text-2xl font-bold text-white mb-3">Tu copiloto durante la reunión</h2>
                  <p className="text-sm text-gray-300 leading-relaxed">
                    Maity te da tips accionables cada 20 segundos mientras hablas con tu cliente. Detecta objeciones, monólogos largos, momentos de cierre. Sin tomar tu atención del interlocutor.
                  </p>
                  {/* SVG inline: Coach tip bubble */}
                  <div className="mt-6 flex justify-center">
                    <svg width="160" height="120" viewBox="0 0 160 120" className="opacity-80">
                      {/* Cara estilizada */}
                      <circle cx="80" cy="50" r="25" fill="rgba(72, 93, 244, 0.2)" stroke="rgba(72, 93, 244, 0.4)" strokeWidth="2" />
                      {/* Ojos */}
                      <circle cx="72" cy="45" r="3" fill="rgba(72, 93, 244, 0.6)" />
                      <circle cx="88" cy="45" r="3" fill="rgba(72, 93, 244, 0.6)" />
                      {/* Boca */}
                      <path d="M 72 55 Q 80 60 88 55" stroke="rgba(72, 93, 244, 0.6)" strokeWidth="2" fill="none" />
                      {/* Burbuja de tip */}
                      <rect x="10" y="80" width="140" height="35" rx="8" fill="rgba(72, 93, 244, 0.15)" stroke="rgba(72, 93, 244, 0.4)" strokeWidth="1.5" />
                      <text x="80" y="103" textAnchor="middle" fontSize="10" fill="rgba(72, 93, 244, 0.8)" fontWeight="500">
                        Pregunta acerca de presupuesto
                      </text>
                      {/* Flecha apuntando a cara */}
                      <path d="M 80 75 L 78 70 L 82 70 Z" fill="rgba(72, 93, 244, 0.4)" />
                    </svg>
                  </div>
                </motion.div>
              )}

              {currentSlide === 1 && (
                <motion.div
                  key="slide-1"
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: -10 }}
                  transition={{ duration: 0.3 }}
                >
                  <div className="flex justify-center mb-6">
                    <motion.div
                      animate={{ scale: [1, 1.05, 1] }}
                      transition={{ duration: 2, repeat: Infinity }}
                    >
                      <PictureInPicture2 className="w-16 h-16 text-[#485df4]" />
                    </motion.div>
                  </div>
                  <h2 className="text-2xl font-bold text-white mb-3">Visible sobre Zoom y Teams</h2>
                  <p className="text-sm text-gray-300 leading-relaxed">
                    Una ventana glass-morphic se abre sola cuando empiezas a grabar. Muestra salud de la conversación, tiempo de palabra, tips activos y preguntas del cliente. La puedes mover y minimizar.
                  </p>
                  {/* SVG inline: Floating window with metrics */}
                  <div className="mt-6 flex justify-center">
                    <svg width="160" height="120" viewBox="0 0 160 120" className="opacity-80">
                      {/* Ventana flotante */}
                      <rect x="30" y="20" width="100" height="80" rx="6" fill="rgba(72, 93, 244, 0.15)" stroke="rgba(72, 93, 244, 0.4)" strokeWidth="1.5" />
                      {/* Título barra */}
                      <rect x="30" y="20" width="100" height="24" rx="6" fill="rgba(72, 93, 244, 0.25)" stroke="none" />
                      <text x="80" y="38" textAnchor="middle" fontSize="9" fill="rgba(72, 93, 244, 0.7)" fontWeight="600">
                        Coach v0.4
                      </text>
                      {/* Gauge ring */}
                      <circle cx="80" cy="60" r="18" fill="none" stroke="rgba(72, 93, 244, 0.2)" strokeWidth="2" />
                      <circle cx="80" cy="60" r="18" fill="none" stroke="rgba(72, 93, 244, 0.6)" strokeWidth="2" strokeDasharray="56.5 113" strokeLinecap="round" />
                      <text x="80" y="65" textAnchor="middle" fontSize="11" fill="rgba(72, 93, 244, 0.8)" fontWeight="600">
                        75
                      </text>
                      <text x="80" y="77" textAnchor="middle" fontSize="8" fill="rgba(72, 93, 244, 0.6)">
                        Salud
                      </text>
                    </svg>
                  </div>
                </motion.div>
              )}

              {currentSlide === 2 && (
                <motion.div
                  key="slide-2"
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  exit={{ opacity: 0, y: -10 }}
                  transition={{ duration: 0.3 }}
                >
                  <div className="flex justify-center mb-6">
                    <motion.div
                      animate={{ rotate: [0, 5, -5, 0] }}
                      transition={{ duration: 2, repeat: Infinity }}
                    >
                      <MessageCircleMore className="w-16 h-16 text-[#485df4]" />
                    </motion.div>
                  </div>
                  <h2 className="text-2xl font-bold text-white mb-3">Pregunta a todas tus reuniones</h2>
                  <p className="text-sm text-gray-300 leading-relaxed">
                    Después de algunas grabaciones, abre el chat global desde el sidebar (Ctrl+K → /chat) y pregunta cosas como "¿qué objeciones recurrentes han surgido?". Las respuestas citan literalmente con timestamps.
                  </p>
                  {/* SVG inline: Chat with citation */}
                  <div className="mt-6 flex justify-center">
                    <svg width="160" height="120" viewBox="0 0 160 120" className="opacity-80">
                      {/* Chat bubble usuário */}
                      <rect x="15" y="20" width="110" height="28" rx="8" fill="rgba(72, 93, 244, 0.2)" stroke="rgba(72, 93, 244, 0.4)" strokeWidth="1.5" />
                      <text x="70" y="40" textAnchor="middle" fontSize="9" fill="rgba(72, 93, 244, 0.8)">
                        ¿Objeciones recurrentes?
                      </text>
                      {/* Chat bubble respuesta */}
                      <rect x="20" y="55" width="120" height="35" rx="8" fill="rgba(72, 93, 244, 0.15)" stroke="rgba(72, 93, 244, 0.35)" strokeWidth="1.5" />
                      <text x="80" y="68" textAnchor="middle" fontSize="8" fill="rgba(72, 93, 244, 0.7)">
                        Precio y implementación
                      </text>
                      {/* Citation */}
                      <rect x="25" y="70" width="110" height="16" rx="4" fill="rgba(72, 93, 244, 0.1)" stroke="rgba(72, 93, 244, 0.3)" strokeWidth="1" strokeDasharray="4" />
                      <text x="80" y="82" textAnchor="middle" fontSize="7" fill="rgba(72, 93, 244, 0.6)">
                        [Reunión Cliente X, 03:42]
                      </text>
                    </svg>
                  </div>
                </motion.div>
              )}
            </AnimatePresence>
          </div>

          {/* Indicadores de progreso (dots) */}
          <div className="flex justify-center gap-2 mb-6">
            {[0, 1, 2].map((i) => (
              <motion.div
                key={i}
                className="h-2 rounded-full transition-all"
                animate={{
                  width: currentSlide === i ? 24 : 8,
                  backgroundColor: currentSlide === i ? '#485df4' : 'rgba(255, 255, 255, 0.2)',
                }}
              />
            ))}
          </div>

          {/* Botones de navegación */}
          <div className="flex items-center justify-between px-8 pb-8 gap-4">
            <button
              onClick={handleSkip}
              className="text-sm text-gray-400 hover:text-gray-300 transition-colors"
            >
              Saltar tour
            </button>
            <button
              onClick={handleNext}
              className="flex items-center gap-2 px-6 py-2.5 rounded-lg font-medium transition-all"
              style={{
                background: '#485df4',
                color: 'white',
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.opacity = '0.9';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.opacity = '1';
              }}
            >
              {currentSlide === 2 ? 'Comenzar' : 'Siguiente'}
              <ChevronRight className="w-4 h-4" />
            </button>
          </div>
        </motion.div>
      </motion.div>
    </AnimatePresence>
  );
}
