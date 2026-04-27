'use client'

import { useEffect, useState } from 'react'
import { motion, AnimatePresence } from 'framer-motion'
import { Loader2 } from 'lucide-react'
import { useStatusETA } from '@/contexts/StatusETAContext'
import { listen } from '@tauri-apps/api/event'

export function StatusETA() {
  const { state } = useStatusETA()
  const [remainingSec, setRemainingSec] = useState<number>(0)
  const [elapsedSec, setElapsedSec] = useState<number>(0)
  const [isTakingLong, setIsTakingLong] = useState(false)

  // Update countdown every 500ms
  useEffect(() => {
    if (!state.active || state.etaSec === undefined || state.startedAt === undefined) {
      return
    }

    const etaSec = state.etaSec
    const startedAt = state.startedAt

    const interval = setInterval(() => {
      const now = Date.now()
      const elapsed = (now - startedAt) / 1000
      const remaining = Math.max(0, etaSec - elapsed)

      setElapsedSec(elapsed)
      setRemainingSec(remaining)

      // Check if it's taking more than 2x the estimated time
      if (elapsed > etaSec * 2) {
        setIsTakingLong(true)
      } else {
        setIsTakingLong(false)
      }
    }, 500)

    return () => clearInterval(interval)
  }, [state.active, state.etaSec, state.startedAt])

  // Listen to Tauri events for auto-start/finish
  useEffect(() => {
    if (!state.active) {
      const unlistenRecordingStop = listen('recording-stop-complete', () => {
        // Auto-start transcription processing
        // Note: actual start needs to be called by the component that has useStatusETA context
      })

      const unlistenMeetingSaved = listen('meeting-saved', () => {
        // Auto-finish
        // Note: actual finish needs to be called by the component
      })

      const unlistenCoachThinking = listen<{ stage: string }>('coach-thinking', () => {
        // Note: actual handling needs to be called by the component
      })

      return () => {
        unlistenRecordingStop.then(fn => fn())
        unlistenMeetingSaved.then(fn => fn())
        unlistenCoachThinking.then(fn => fn())
      }
    }
  }, [state.active])

  const formatTime = (seconds: number): string => {
    if (seconds === 0) return 'Casi listo...'
    return `${Math.ceil(seconds)}s`
  }

  const progressPercent = state.etaSec !== undefined ? Math.min((elapsedSec / state.etaSec) * 100, 95) : 0

  const variantColors = {
    info: { border: 'border-blue-500', bg: 'from-blue-950' },
    warn: { border: 'border-amber-500', bg: 'from-amber-950' },
    success: { border: 'border-emerald-500', bg: 'from-emerald-950' },
  }

  const variant = state.variant || 'info'
  const colors = variantColors[variant]

  return (
    <AnimatePresence mode="wait">
      {state.active && (
        <motion.div
          key="status-eta-bar"
          initial={{ y: -64, opacity: 0 }}
          animate={{ y: 0, opacity: 1 }}
          exit={{ y: -64, opacity: 0 }}
          transition={{ type: 'spring', damping: 20, stiffness: 300 }}
          className={`fixed top-0 left-0 right-0 z-50 border-b ${colors.border}`}
          style={{
            background: 'rgba(15, 16, 24, 0.95)',
            backdropFilter: 'blur(16px)',
          }}
        >
          {/* Progress bar background */}
          <div
            className="absolute bottom-0 left-0 h-1 bg-gradient-to-r from-blue-500 via-purple-500 to-pink-500 transition-all duration-300"
            style={{
              width: `${progressPercent}%`,
              opacity: 0.8,
            }}
          />

          <div className="flex items-center justify-between px-6 py-3 h-16">
            {/* Left: Spinner */}
            <div className="flex-shrink-0">
              <Loader2 className="w-5 h-5 text-blue-400 animate-spin" />
            </div>

            {/* Center: Label */}
            <div className="flex-1 text-center px-4">
              <p className="text-sm font-medium text-gray-100">{state.label}</p>
              {isTakingLong && (
                <p className="text-xs text-amber-400 mt-1">
                  Tarda más de lo normal — espera o cancela
                </p>
              )}
            </div>

            {/* Right: Time remaining */}
            <div className="flex-shrink-0 text-right">
              <span className="text-sm font-semibold text-gray-300">
                {remainingSec === 0 ? (
                  <span className="text-emerald-400">Completando...</span>
                ) : (
                  <span>{formatTime(remainingSec)} restantes</span>
                )}
              </span>
            </div>
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  )
}
