# Ventana Flotante Always-On-Top

## Descripción General

Ventana glass-morphic transparente que flota sobre Zoom, Teams u otras aplicaciones permitiendo al usuario leer tips del coach sin tapar la pantalla del interlocutor. Funciona en paralelo con `CoachPanel` principal.

**Características:**
- Modo expandido 320×380px (tips completos)
- Modo compacto 140×110px (solo métrica + 1 línea tip)
- Always-on-top: permanece visible sobre todas las ventanas
- Transparencia con blur (backdrop-filter)
- Toggle minimizar/expandir vía botón Minimize2
- Posicionamiento automático en esquina superior-derecha

## Arquitectura

### Backend Rust

**Módulo:**
```
frontend/src-tauri/src/coach/floating.rs (~220 LOC)
```

**Comandos Tauri:**

```rust
#[tauri::command]
async fn open_floating_coach(
    app: AppHandle<impl Runtime>
) -> Result<(), String>
```
Abre ventana flotante en esquina superior-derecha. Si ya existe, la trae al frente.

```rust
#[tauri::command]
async fn close_floating_coach(
    app: AppHandle<impl Runtime>
) -> Result<(), String>
```
Cierra ventana flotante.

```rust
#[tauri::command]
async fn floating_toggle_compact(
    app: AppHandle<impl Runtime>,
    is_compact: bool
) -> Result<(), String>
```
Cambia modo expandido ↔ compacto.

### Configuración Tauri

**Capability en `tauri.conf.json`:**
```json
{
  "permissions": [
    "core:window:allow-set-always-on-top",
    "core:window:allow-set-decorations",
    "core:window:allow-set-position",
    "core:window:allow-set-size",
    "core:window:allow-create",
    "core:window:allow-close",
    "coach-floating:all"
  ]
}
```

**Definición de capability:**
```json
{
  "identifier": "coach-floating",
  "description": "Permite crear ventana flotante para tips del coach",
  "windows": ["floating_coach"],
  "webapi": {
    "window": {
      "setAlwaysOnTop": [],
      "setDecorations": [],
      "setPosition": [],
      "setSize": []
    }
  }
}
```

### Frontend TypeScript

**Página raíz:**
```typescript
// frontend/src/app/floating/page.tsx (~120 LOC)
export default function FloatingCoachPage() {
  // Renderiza el panel compacto/expandido sin sidebar
  // Escucha eventos: coach-tip-update, audio-levels, meeting-metrics
}
```

**Root layout con detección:**
```typescript
// frontend/src/app/layout.tsx (snippet)
export default function RootLayout({ children }) {
  const pathname = usePathname();
  const isFloating = pathname === '/floating';
  
  return (
    <html>
      <body>
        {!isFloating && <Sidebar />}  {/* Ocultar sidebar en /floating */}
        {children}
      </body>
    </html>
  );
}
```

**Componente UI:**
```typescript
// frontend/src/components/Coach/FloatingPanel.tsx (~180 LOC)
export function FloatingPanel({ isCompact }: Props) {
  if (isCompact) {
    return <CompactView />;  // 140×110, métrica + 1 línea
  }
  return <ExpandedView />;   // 320×380, completo
}
```

## Flujo de Uso

1. **Usuario hace clic botón PiP** en `CoachPanel` (header derecho)
2. Backend invoca `open_floating_coach` → abre nueva ventana
3. **Ventana se posiciona** en esquina superior-derecha (margen 32px top/right)
4. **Usuario minimiza/expande** vía botón toggle en la ventana flotante
5. **Tips se actualizan en tiempo real** — mismos eventos que el panel principal
6. **Usuario cierra reunión** → ventana se cierra automáticamente (optional)

## Posicionamiento

**Algoritmo:**
```typescript
function calculateFloatingPosition(primaryMonitor: Monitor) {
  const screenWidth = primaryMonitor.scaledSize.width;
  const screenHeight = primaryMonitor.scaledSize.height;
  
  const MARGIN = 32;
  const WINDOW_WIDTH = isCompact ? 140 : 320;
  const WINDOW_HEIGHT = isCompact ? 110 : 380;
  
  const x = screenWidth - WINDOW_WIDTH - MARGIN;
  const y = 80;  // Top margin para barra de apps
  
  return { x, y };
}
```

**Márgenes:**
- Top: 80px (debajo de barra de Windows/menubar)
- Right: 32px
- Mantiene distancia del borde de la pantalla

## Eventos Consumidos

La ventana flotante escucha estos eventos emitidos por el backend:

| Evento | Payload | Uso |
|--------|---------|-----|
| `coach-tip-update` | `{ tip: string, priority: "critical"\|"important"\|"soft" }` | Mostrar nuevo tip |
| `coach-suggestion` | `CoachSuggestion` | Actualizar tarjeta (alternativa a tip-update) |
| `audio-levels` | `{ mic_db: number, system_db: number }` | Barras de nivel mic/sistema |
| `meeting-metrics` | `{ health_score: 0-100, wpm: number, time_elapsed: number }` | Métrica de salud principal |

**Implementación frontend:**
```typescript
useEffect(() => {
  const unlistens = [
    listen('coach-tip-update', (event) => setCurrentTip(event.payload)),
    listen('audio-levels', (event) => setAudioLevels(event.payload)),
    listen('meeting-metrics', (event) => setMetrics(event.payload)),
  ];
  
  return () => unlistens.forEach(u => u());
}, []);
```

## Vista Expandida (320×380px)

**Layout:**
```
┌─────────────────────────────┐
│ X [minimize]   [close]      │  20px header
├─────────────────────────────┤
│                             │
│  ⚡ Salud: 78%   [color]    │  Métrica de salud (120px)
│  📊 WPM: 142    [color]     │
│  ⏱️ Tiempo: 04:35            │
│                             │
├─────────────────────────────┤
│ 🔴 CRÍTICO                  │  Prioridad badge (20px)
│                             │
│ "Escucha más, pregunta      │  Tip text (160px, multiline)
│  sobre necesidades antes    │
│  de proponer soluciones"    │
│                             │
│ Técnica: SPIN               │  Técnica sugerida (18px)
│ Categoría: Discovery        │
│                             │
├─────────────────────────────┤
│                             │  Audio bars (60px)
│ Mic: ▁▂▃▄▅ -12dB          │
│ Sys: ▁▂▂▃▄ -18dB          │
│                             │
└─────────────────────────────┘
```

**Colores de prioridad:**
- 🔴 Crítico: `rgb(220, 38, 38)` (rojo)
- 🟡 Importante: `rgb(202, 138, 4)` (ámbar)
- 🟢 Mejorable: `rgb(34, 197, 94)` (verde)

## Vista Compacta (140×110px)

**Layout:**
```
┌──────────────┐
│ ≡ [X]        │  12px header
├──────────────┤
│ ⚡ 78%       │  Métrica de salud (30px)
│ Escucha más  │  Tip 1 línea (40px, overflow:hidden)
│              │
└──────────────┘
```

**Comportamiento:**
- Métrica "Salud" en grande + color
- Tip truncado a 1 línea
- Botón minimizar (≡) permanece visible
- Al expandir, transición smooth a vista completa

## CSS & Estilos

**Tema glass-morphic:**
```css
.floating-panel {
  background: rgba(20, 20, 28, 0.78);     /* Fondo semi-transparente */
  backdrop-filter: blur(16px);             /* Blur del contenido atrás */
  border: 1px solid rgba(255, 255, 255, 0.08);
  border-radius: 12px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
}

.floating-panel.compact {
  background: rgba(20, 20, 28, 0.82);     /* Ligeramente más opaco en compacto */
  backdrop-filter: blur(20px);             /* Blur ligeramente más fuerte */
}
```

**Tipografía:**
- Expandido: Métrica 24px bold, tip 14px regular, técnica 12px muted
- Compacto: Métrica 20px bold, tip 11px regular

**Transiciones:**
- Cambio size: 200ms ease-out (Tauri window resize)
- Opacidad badge: 300ms ease-in-out
- Audio bars: sin transición (update en tiempo real)

## Interacción Botones

**Botón PiP (Picture-in-Picture) en CoachPanel:**
```typescript
<button 
  onClick={() => invoke('open_floating_coach')}
  title="Abrir ventana flotante"
>
  <PiIcon size={20} />
</button>
```

**Botón Minimize2 en FloatingPanel:**
```typescript
<button
  onClick={() => invoke('floating_toggle_compact', { is_compact: !isCompact })}
>
  {isCompact ? <Maximize2Icon /> : <Minimize2Icon />}
</button>
```

**Botón Close:**
```typescript
<button
  onClick={() => invoke('close_floating_coach')}
>
  ✕
</button>
```

## Limitaciones Conocidas

### macOS
- Requiere permiso `macos:allow-set-always-on-top` (solicita usuario en primera apertura)
- ScreenCaptureKit puede interferir con overlay; worst case: requiere focus switch
- Probado en macOS 12.7+; versiones anteriores pueden tener issues

### Linux
- Depende del window manager (GNOME vs KDE vs i3)
- `always-on-top` no funciona en algunos tiling managers (i3wm)
- Tested on GNOME; otros distros pueden requerir ajustes

### Windows
- Fully soportado en Windows 10/11
- Alt+Tab incluye la ventana flotante (expected behavior)
- En algunos juegos fullscreen, `always-on-top` puede ignorarse

## Auto-Cierre

**Opcional — Implementable:**
```rust
// Al invocar stop_recording, cerrar ventana flotante automáticamente
#[tauri::command]
async fn stop_recording(...) -> Result<(), String> {
    // ... lógica existente ...
    let _ = invoke('close_floating_coach');  // Best-effort
    Ok(())
}
```

**Comportamiento esperado:**
- Usuario detiene grabación
- Ventana flotante se cierra (no necesita acción manual)
- Panel principal permanece abierto

## Testing Manual

1. **Abrir CoachPanel** → grabar reunión
2. **Hacer clic botón PiP** → ventana flotante aparece esquina superior-derecha
3. **Verificar posición** → no tapa interlocutor (Zoom/Teams)
4. **Tips se actualizan** → coinciden con panel principal
5. **Minimizar** → cambia a vista compacta 140×110
6. **Expandir** → vuelve a 320×380
7. **Cerrar ventana** → no afecta CoachPanel
8. **Cerrar CoachPanel** → ventana flotante permanece (independiente)

## Mejoras Futuras

- Drag-to-reposition (usuario puede mover ventana flotante a otra esquina)
- Persistencia de posición (guardar última posición en `localStorage`)
- Pin/unpin (permite que ventana se mueva al traer aplicación principal al frente)
- Multi-monitor support (detectar monitor actual del cursor)
- Temas oscuro/claro (seguir theme de sistema)
