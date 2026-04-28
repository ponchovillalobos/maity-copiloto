'use client';

import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { PlusIcon, TrashIcon, CheckIcon } from '@heroicons/react/24/outline';
import { toast } from 'sonner';

interface CustomPrompt {
  id: number;
  name: string;
  purpose: string;
  prompt_text: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

interface CreateCustomPromptInput {
  name: string;
  purpose: string;
  prompt_text: string;
  activate?: boolean;
}

const PURPOSES = [
  { id: 'tips', label: 'Tips de Coach' },
  { id: 'evaluation', label: 'Evaluación de Comunicación' },
  { id: 'chat', label: 'Chat General' },
  { id: 'prospecting', label: 'Email Prospecting' },
];

export function CustomPromptsPanel() {
  const [prompts, setPrompts] = useState<CustomPrompt[]>([]);
  const [loading, setLoading] = useState(true);
  const [showModal, setShowModal] = useState(false);
  const [selectedPurpose, setSelectedPurpose] = useState<string>('tips');
  const [formData, setFormData] = useState({
    name: '',
    prompt_text: '',
    activate: false,
  });

  const loadPrompts = async () => {
    try {
      setLoading(true);
      const data = await invoke<CustomPrompt[]>('coach_list_custom_prompts', {
        purpose: null,
      });
      setPrompts(data);
    } catch (error) {
      toast.error(`Error cargando prompts: ${String(error)}`);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadPrompts();
  }, []);

  const handleSavePrompt = async () => {
    const name = formData.name.trim();
    const prompt_text = formData.prompt_text.trim();

    if (!name) {
      toast.error('Ingresa un nombre para el prompt');
      return;
    }

    if (prompt_text.length < 20) {
      toast.error('El prompt debe tener al menos 20 caracteres');
      return;
    }

    try {
      const input: CreateCustomPromptInput = {
        name,
        purpose: selectedPurpose,
        prompt_text,
        activate: formData.activate,
      };

      await invoke<number>('coach_save_custom_prompt', { input });

      toast.success('Prompt guardado exitosamente');

      setShowModal(false);
      setFormData({
        name: '',
        prompt_text: '',
        activate: false,
      });
      await loadPrompts();
    } catch (error) {
      toast.error(`Error guardando prompt: ${String(error)}`);
    }
  };

  const handleSetActive = async (id: number) => {
    try {
      await invoke('coach_set_active_custom_prompt', { id });
      toast.success('Prompt activado');
      await loadPrompts();
    } catch (error) {
      toast.error(`Error activando prompt: ${String(error)}`);
    }
  };

  const handleDeletePrompt = async (id: number) => {
    try {
      await invoke('coach_delete_custom_prompt', { id });
      toast.success('Prompt eliminado');
      await loadPrompts();
    } catch (error) {
      toast.error(`Error eliminando prompt: ${String(error)}`);
    }
  };

  const groupedPrompts = PURPOSES.reduce(
    (acc, purpose) => {
      acc[purpose.id] = prompts.filter((p) => p.purpose === purpose.id);
      return acc;
    },
    {} as Record<string, CustomPrompt[]>
  );

  return (
    <div className="space-y-8">
      {/* Header */}
      <div className="flex justify-between items-center">
        <div>
          <h2 className="text-2xl font-semibold text-white">Prompts Personalizados</h2>
          <p className="text-sm text-gray-400 mt-1">
            Personaliza los prompts del coach para tu equipo sin modificar código
          </p>
        </div>
        <button
          onClick={() => setShowModal(true)}
          className="flex items-center gap-2 px-4 py-2 bg-gradient-to-r from-blue-600 to-blue-700 hover:from-blue-700 hover:to-blue-800 text-white rounded-lg transition"
        >
          <PlusIcon className="w-5 h-5" />
          Nuevo Prompt
        </button>
      </div>

      {/* Purpose Sections */}
      <div className="space-y-6">
        {PURPOSES.map((purpose) => (
          <div key={purpose.id} className="border border-gray-700 rounded-lg p-6 bg-gray-900/50 backdrop-blur">
            <h3 className="text-lg font-semibold text-white mb-4">{purpose.label}</h3>

            {loading ? (
              <p className="text-gray-400 text-sm">Cargando...</p>
            ) : groupedPrompts[purpose.id].length === 0 ? (
              <p className="text-gray-500 text-sm italic">Sin prompts personalizados</p>
            ) : (
              <div className="space-y-3">
                {groupedPrompts[purpose.id].map((prompt) => (
                  <div
                    key={prompt.id}
                    className="flex items-center justify-between p-3 bg-gray-800/60 rounded border border-gray-700 hover:border-gray-600 transition"
                  >
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <p className="font-medium text-white truncate">{prompt.name}</p>
                        {prompt.is_active && (
                          <span className="text-xs px-2 py-1 bg-green-900/50 text-green-300 rounded border border-green-700">
                            ACTIVO
                          </span>
                        )}
                      </div>
                      <p className="text-xs text-gray-500 mt-1">
                        Creado: {new Date(prompt.created_at).toLocaleDateString('es-ES')}
                      </p>
                    </div>

                    <div className="flex items-center gap-2 ml-4">
                      {!prompt.is_active && (
                        <button
                          onClick={() => handleSetActive(prompt.id)}
                          className="p-2 hover:bg-blue-900/30 rounded transition text-blue-400 hover:text-blue-300"
                          title="Activar prompt"
                        >
                          <CheckIcon className="w-5 h-5" />
                        </button>
                      )}
                      <button
                        onClick={() => handleDeletePrompt(prompt.id)}
                        className="p-2 hover:bg-red-900/30 rounded transition text-red-400 hover:text-red-300"
                        title="Eliminar prompt"
                      >
                        <TrashIcon className="w-5 h-5" />
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        ))}
      </div>

      {/* Modal */}
      {showModal && (
        <div className="fixed inset-0 bg-black/70 backdrop-blur-sm flex items-center justify-center z-50 p-4">
          <div className="bg-gray-900 border border-gray-700 rounded-lg max-w-2xl w-full max-h-[90vh] overflow-y-auto">
            {/* Modal Header */}
            <div className="flex justify-between items-center p-6 border-b border-gray-700">
              <h3 className="text-xl font-semibold text-white">Nuevo Prompt Personalizado</h3>
              <button
                onClick={() => setShowModal(false)}
                className="text-gray-400 hover:text-white transition"
              >
                ✕
              </button>
            </div>

            {/* Modal Body */}
            <div className="p-6 space-y-4">
              {/* Purpose Select */}
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-2">
                  Tipo de Prompt
                </label>
                <select
                  value={selectedPurpose}
                  onChange={(e) => setSelectedPurpose(e.target.value)}
                  className="w-full px-3 py-2 bg-gray-800 border border-gray-700 text-white rounded-lg focus:outline-none focus:border-blue-500 transition"
                >
                  {PURPOSES.map((p) => (
                    <option key={p.id} value={p.id}>
                      {p.label}
                    </option>
                  ))}
                </select>
              </div>

              {/* Name Input */}
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-2">
                  Nombre del Prompt
                </label>
                <input
                  type="text"
                  value={formData.name}
                  onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                  placeholder="Ej: Coach Ventas B2B"
                  className="w-full px-3 py-2 bg-gray-800 border border-gray-700 text-white rounded-lg focus:outline-none focus:border-blue-500 placeholder-gray-500 transition"
                />
              </div>

              {/* Prompt Text Input */}
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-2">
                  Texto del Prompt
                </label>
                <textarea
                  value={formData.prompt_text}
                  onChange={(e) => setFormData({ ...formData, prompt_text: e.target.value })}
                  placeholder="Escribe el prompt aquí. Mínimo 20 caracteres..."
                  rows={12}
                  className="w-full px-3 py-2 bg-gray-800 border border-gray-700 text-white rounded-lg focus:outline-none focus:border-blue-500 placeholder-gray-500 font-mono text-sm transition"
                />
                <p className="text-xs text-gray-500 mt-1">
                  {formData.prompt_text.length} caracteres
                </p>
              </div>

              {/* Activate Checkbox */}
              <div className="flex items-center gap-2">
                <input
                  type="checkbox"
                  id="activate_prompt"
                  checked={formData.activate}
                  onChange={(e) => setFormData({ ...formData, activate: e.target.checked })}
                  className="w-4 h-4 rounded border-gray-600 bg-gray-800 cursor-pointer"
                />
                <label htmlFor="activate_prompt" className="text-sm text-gray-300 cursor-pointer">
                  Activar este prompt al guardar
                </label>
              </div>
            </div>

            {/* Modal Footer */}
            <div className="flex justify-end gap-3 p-6 border-t border-gray-700">
              <button
                onClick={() => setShowModal(false)}
                className="px-4 py-2 border border-gray-600 text-gray-300 rounded-lg hover:bg-gray-800 transition"
              >
                Cancelar
              </button>
              <button
                onClick={handleSavePrompt}
                className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition"
              >
                Guardar Prompt
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
