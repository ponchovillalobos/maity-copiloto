'use client';

interface ContextMenuProps {
  visible: boolean;
  x: number;
  y: number;
  selectedBlockCount: number;
  onCopy: () => void;
  onDelete: () => void;
}

export function ContextMenu({
  visible,
  x,
  y,
  selectedBlockCount,
  onCopy,
  onDelete,
}: ContextMenuProps) {
  if (!visible || selectedBlockCount === 0) return null;

  return (
    <div
      className="fixed z-50 bg-white dark:bg-gray-800 shadow-lg rounded-lg py-1 min-w-[160px] border border-[#e7e7e9] dark:border-gray-700
                 animate-in fade-in zoom-in-95 duration-150"
      style={{ left: x, top: y }}
      onClick={e => e.stopPropagation()}
    >
      <button
        className="w-full px-4 py-2 text-left hover:bg-[#e7e7e9] dark:hover:bg-gray-700 flex items-center space-x-2"
        onClick={onCopy}
      >
        <span className="text-gray-200 dark:text-gray-300">📋</span>
        <span>Copiar {selectedBlockCount > 1 ? `${selectedBlockCount} bloques` : 'bloque'}</span>
      </button>
      <button
        className="w-full px-4 py-2 text-left hover:bg-[#e7e7e9] dark:hover:bg-gray-700 text-[#cc0040] dark:text-red-400 flex items-center space-x-2"
        onClick={onDelete}
      >
        <span>🗑️</span>
        <span>Eliminar {selectedBlockCount > 1 ? `${selectedBlockCount} bloques` : 'bloque'}</span>
      </button>
    </div>
  );
}
