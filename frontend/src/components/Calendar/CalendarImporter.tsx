'use client';

import React, { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Calendar, Upload, X, Users, Clock, MapPin, Mail } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { toast } from 'sonner';

interface CalendarEvent {
  uid: string;
  summary: string;
  description: string;
  start: string;
  end: string;
  organizer?: string;
  attendees: string[];
  location?: string;
}

/**
 * CalendarImporter — Modal para importar archivos .ics de calendario local.
 * Parsea eventos y permite asociar reuniones grabadas con eventos de calendario.
 * Privacidad-first: todo ocurre localmente, sin APIs externas.
 */
export function CalendarImporter() {
  const [isOpen, setIsOpen] = useState(false);
  const [events, setEvents] = useState<CalendarEvent[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [selectedEvent, setSelectedEvent] = useState<CalendarEvent | null>(null);

  // Escuchar evento global para abrir el modal
  useEffect(() => {
    const handleOpen = () => setIsOpen(true);
    window.addEventListener('open-calendar-import', handleOpen);
    return () => window.removeEventListener('open-calendar-import', handleOpen);
  }, []);

  const handleSelectFile = async () => {
    try {
      const filePath = await open({
        filters: [
          { name: 'iCalendar', extensions: ['ics'] },
          { name: 'Todos', extensions: ['*'] },
        ],
      });

      if (!filePath) return; // Usuario canceló

      setIsLoading(true);
      const parsed = await invoke<CalendarEvent[]>('calendar_parse_ics_file', {
        path: filePath,
      });
      setEvents(parsed);
      toast.success(`Cargados ${parsed.length} eventos`);
    } catch (error) {
      toast.error(`Error al cargar archivo: ${error}`);
    } finally {
      setIsLoading(false);
    }
  };

  const handleSelectEvent = (event: CalendarEvent) => {
    setSelectedEvent(event);
    // Guardar en localStorage para asociar con próxima reunión
    localStorage.setItem('maity_pending_calendar_event', JSON.stringify(event));
    toast.success('Evento asociado a la próxima reunión');
    setIsOpen(false);
  };

  return (
    <AnimatePresence>
      {isOpen && (
        <motion.div
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          exit={{ opacity: 0 }}
          transition={{ duration: 0.15 }}
          className="fixed inset-0 z-[150] bg-black/50 backdrop-blur-sm flex items-center justify-center p-4"
          onClick={() => setIsOpen(false)}
        >
          <motion.div
            initial={{ opacity: 0, scale: 0.95, y: 10 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.95 }}
            transition={{ duration: 0.2 }}
            className="bg-gray-900 border border-gray-700 rounded-lg shadow-2xl w-full max-w-2xl max-h-[80vh] overflow-hidden flex flex-col"
            onClick={(e) => e.stopPropagation()}
          >
            {/* Encabezado */}
            <div className="bg-gradient-to-r from-blue-600 to-blue-500 px-6 py-4 flex items-center justify-between">
              <div className="flex items-center gap-3">
                <Calendar className="w-5 h-5 text-white" />
                <h2 className="text-lg font-semibold text-white">Importar calendario</h2>
              </div>
              <button
                onClick={() => setIsOpen(false)}
                className="text-white/80 hover:text-white transition-colors"
              >
                <X className="w-5 h-5" />
              </button>
            </div>

            {/* Contenido */}
            <div className="flex-1 overflow-y-auto p-6 space-y-4">
              {events.length === 0 ? (
                <div className="text-center py-12">
                  <Upload className="w-12 h-12 text-gray-500 mx-auto mb-3" />
                  <p className="text-gray-300 text-sm mb-4">
                    Carga un archivo .ics exportado de Outlook, Google Calendar o Apple
                  </p>
                  <button
                    onClick={handleSelectFile}
                    disabled={isLoading}
                    className="inline-flex items-center gap-2 px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-700 text-white rounded-lg transition-colors text-sm font-medium"
                  >
                    {isLoading ? 'Cargando...' : 'Seleccionar archivo .ics'}
                  </button>
                </div>
              ) : (
                <>
                  <div className="flex items-center justify-between">
                    <h3 className="text-sm font-semibold text-gray-200">
                      Eventos encontrados: {events.length}
                    </h3>
                    <button
                      onClick={handleSelectFile}
                      className="text-xs text-blue-400 hover:text-blue-300 underline"
                    >
                      Cargar otro archivo
                    </button>
                  </div>

                  {/* Lista de eventos */}
                  <div className="space-y-2">
                    {events.map((event) => (
                      <div
                        key={event.uid}
                        className="bg-gray-800/50 border border-gray-700 rounded-lg p-4 hover:border-blue-500/50 cursor-pointer transition-colors"
                        onClick={() => handleSelectEvent(event)}
                      >
                        <div className="flex items-start justify-between gap-4">
                          <div className="flex-1 min-w-0">
                            <h4 className="font-semibold text-gray-100 truncate">
                              {event.summary || '(sin título)'}
                            </h4>
                            {event.description && (
                              <p className="text-xs text-gray-400 truncate mt-1">
                                {event.description}
                              </p>
                            )}
                          </div>
                          <button className="px-3 py-1 bg-blue-600 hover:bg-blue-700 text-white text-xs rounded transition-colors whitespace-nowrap">
                            Usar este
                          </button>
                        </div>

                        {/* Detalles del evento */}
                        <div className="grid grid-cols-2 gap-2 mt-3 text-xs">
                          {event.start && (
                            <div className="flex items-center gap-2 text-gray-400">
                              <Clock className="w-3 h-3 flex-shrink-0" />
                              <span className="truncate">{event.start}</span>
                            </div>
                          )}
                          {event.location && (
                            <div className="flex items-center gap-2 text-gray-400">
                              <MapPin className="w-3 h-3 flex-shrink-0" />
                              <span className="truncate">{event.location}</span>
                            </div>
                          )}
                          {event.organizer && (
                            <div className="flex items-center gap-2 text-gray-400">
                              <Mail className="w-3 h-3 flex-shrink-0" />
                              <span className="truncate">{event.organizer}</span>
                            </div>
                          )}
                          {event.attendees.length > 0 && (
                            <div className="flex items-center gap-2 text-gray-400">
                              <Users className="w-3 h-3 flex-shrink-0" />
                              <span className="truncate">{event.attendees.length} asistentes</span>
                            </div>
                          )}
                        </div>
                      </div>
                    ))}
                  </div>
                </>
              )}
            </div>

            {/* Pie */}
            <div className="border-t border-gray-700 bg-gray-800/50 px-6 py-3 text-xs text-gray-400">
              Formato: iCalendar (.ics) — privacidad 100% local
            </div>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}
