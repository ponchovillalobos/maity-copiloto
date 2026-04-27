'use client'

import { createContext, useContext, useState, ReactNode } from 'react'

export type StatusVariant = 'info' | 'warn' | 'success'

export interface StatusETAState {
  active: boolean
  label: string
  etaSec?: number
  startedAt?: number
  variant?: StatusVariant
}

interface StatusETAContextType {
  state: StatusETAState
  start: (label: string, etaSec: number, variant?: StatusVariant) => void
  finish: () => void
}

const StatusETAContext = createContext<StatusETAContextType | null>(null)

export function StatusETAProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<StatusETAState>({ active: false, label: '' })

  const start = (label: string, etaSec: number, variant?: StatusVariant) => {
    setState({ active: true, label, etaSec, startedAt: Date.now(), variant })
  }

  const finish = () => setState((prev) => ({ ...prev, active: false }))

  const value: StatusETAContextType = { state, start, finish }

  return (
    <StatusETAContext.Provider value={value}>
      {children}
    </StatusETAContext.Provider>
  )
}

export function useStatusETA(): StatusETAContextType {
  const context = useContext(StatusETAContext)
  if (!context) {
    throw new Error('useStatusETA must be used within StatusETAProvider')
  }
  return context
}
