import { useEffect } from 'react';

export function useWindowCloseGuard(isRecording: boolean) {
  useEffect(() => {
    let cleanup: (() => void) | undefined;

    const setup = async () => {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        const appWindow = getCurrentWindow();
        cleanup = await appWindow.onCloseRequested(async (event) => {
          if (isRecording) {
            event.preventDefault();
            try {
              const { confirm } = await import('@tauri-apps/plugin-dialog');
              const shouldClose = await confirm(
                'Hay una grabación en curso. Si cierras la aplicación se detendrá la grabación y podrías perder lo que aún no se ha guardado. ¿Continuar?',
                { title: 'Grabación en progreso', kind: 'warning' }
              );
              if (shouldClose) {
                appWindow.close();
              }
            } catch {
              // If dialog fails, allow close
              appWindow.close();
            }
          }
        });
      } catch {
        // Not in Tauri environment (e.g., browser dev), skip
      }
    };

    setup();
    return () => cleanup?.();
  }, [isRecording]);
}
