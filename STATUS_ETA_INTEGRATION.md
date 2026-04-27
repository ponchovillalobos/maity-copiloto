# StatusETA System - Integration Guide

## Overview

Sistema de Status Bar con ETA countdown para Maity Desktop. Aparece automáticamente en la parte superior durante operaciones largas (procesamiento de transcripts, evaluación LLM, indexado embeddings) y desaparece cuando termina.

## Architecture

### Context + Hook: `StatusETAContext.tsx`

Ubicación: `frontend/src/contexts/StatusETAContext.tsx`

Exporta:
- `StatusETAProvider` - Proveedor de React que envuelve la app
- `useStatusETA()` - Hook para acceder a start/finish/state

```tsx
const { state, start, finish } = useStatusETA()
```

### Component: `StatusETA.tsx`

Ubicación: `frontend/src/components/StatusBar/StatusETA.tsx`

Características:
- Slide-down animation con framer-motion
- Countdown actualizado cada 500ms
- Progress bar animada
- Detección automática si tarda >2× ETA estimado
- Glass-morphic design con blur 16px
- Variants de color: info (azul), warn (ámbar), success (esmeralda)

### Integration en `layout.tsx`

1. Importar Provider y Component:
```tsx
import { StatusETAProvider } from '@/contexts/StatusETAContext'
import { StatusETA } from '@/components/StatusBar/StatusETA'
```

2. Envolver app con Provider:
```tsx
<StatusETAProvider>
  <SidebarProvider>
    {/* ... rest of providers ... */}
  </SidebarProvider>
</StatusETAProvider>
```

3. Montar component en el layout:
```tsx
<div className="flex flex-col h-screen">
  <StatusETA />
  {/* OfflineIndicator y resto */}
</div>
```

## Usage Example

En cualquier componente con `useStatusETA()`:

```tsx
import { useStatusETA } from '@/contexts/StatusETAContext'

export function MyComponent() {
  const { start, finish } = useStatusETA()

  const handleLongOperation = async () => {
    // Iniciar countdown: "Procesando transcripción", ~8 segundos
    start('Procesando transcripción', 8, 'info')
    
    try {
      await someOperation()
      finish()
    } catch (error) {
      finish() // También termina en error
    }
  }

  return <button onClick={handleLongOperation}>Procesar</button>
}
```

## Auto-Triggered Events (Future Enhancement)

El componente escucha eventos Tauri (código preparado para expansión):
- `recording-stop-complete` → start("Procesando transcripción", 8, 'info')
- `meeting-saved` → finish()
- `coach-thinking` con stage='analyzing' → start("Analizando con IA", 5, 'info')

Esto permite que operaciones largas que ocurren en el backend Rust se reflejen automáticamente en la UI sin necesidad de código manual en cada componente.

## Type Safety

Interfaces TypeScript:
```tsx
type StatusVariant = 'info' | 'warn' | 'success'

interface StatusETAState {
  active: boolean
  label: string
  etaSec?: number        // estimación en segundos
  startedAt?: number     // Date.now() cuando se inició
  variant?: StatusVariant
}
```

## Build Status

✓ TypeScript: Compilación limpia (corepack pnpm exec tsc --noEmit)
✓ Build: Tauri release build completó exitosamente (exit code 0)
✓ Artefactos: 
  - maity-desktop.exe (57M)
  - Maity_0.4.0_x64_en-US.msi (32M)
  - Maity_0.4.0_x64-setup.exe (17M)

## Files Created/Modified

### Created:
- `frontend/src/contexts/StatusETAContext.tsx` - Provider + Hook
- `frontend/src/components/StatusBar/StatusETA.tsx` - UI Component

### Modified:
- `frontend/src/app/layout.tsx` - Integración Provider + Component

## Notes

- Componente usa `framer-motion` y `lucide-react` (ya instalados)
- Sin dependencias nuevas
- Spanish neutro en todos los textos UI
- Design responsivo con tailwindcss
- Zero performance impact cuando `active: false`
