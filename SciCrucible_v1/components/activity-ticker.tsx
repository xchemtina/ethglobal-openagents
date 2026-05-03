"use client"

import {
  Activity,
  AlertTriangle,
  Database,
  BotMessageSquare,
  FileText,
  GitBranch,
  Layers,
} from "lucide-react"
import type { ReactNode } from "react"

interface TickerEvent {
  icon: ReactNode
  agent: string
  action: string
  color: string
  timestamp: string  // relative, e.g. "12s"
}

const TICKER_EVENTS: TickerEvent[] = [
  { icon: <GitBranch        className="w-3 h-3" />, agent: "AiZynth @ Olympus", action: "targets 3, 4, and 5 solved under USPTO/ZINC default search",                  color: "oklch(0.72 0.16 78)",  timestamp: "08s" },
  { icon: <AlertTriangle    className="w-3 h-3" />, agent: "AiZynth @ Olympus", action: "targets 1 and 2 remain unsolved under default search; no wetlab claim made", color: "oklch(0.72 0.16 78)",  timestamp: "15s" },
  { icon: <Activity         className="w-3 h-3" />, agent: "Gauss-DFT",         action: "B3LYP / def2-svp NBS precursor converged · result art_b2b2171ec8afc316",     color: "oklch(0.70 0.18 148)", timestamp: "23s" },
  { icon: <Activity         className="w-3 h-3" />, agent: "Gauss-DFT",         action: "mesyl anhydride B3LYP gap 8.703 eV · result art_b879d21ada35b829",          color: "oklch(0.70 0.18 148)", timestamp: "31s" },
  { icon: <Activity         className="w-3 h-3" />, agent: "Gauss-DFT",         action: "acetylated diol precursor B3LYP completed · result art_c1e68e07ecd1a323",   color: "oklch(0.70 0.18 148)", timestamp: "39s" },
  { icon: <AlertTriangle    className="w-3 h-3" />, agent: "MolADT",            action: "TBS alcohol blocked: AtomicSymbol lacks silicon support",                   color: "oklch(0.65 0.20 25)",  timestamp: "47s" },
  { icon: <Database         className="w-3 h-3" />, agent: "Analysis Dock",     action: "retrosynthesis summaries copied into dashboard public artifacts",            color: "oklch(0.76 0.17 192)", timestamp: "1m"  },
  { icon: <Layers           className="w-3 h-3" />, agent: "WorldModel",        action: "CASP → precursor DFT lineage projected as science transactions",            color: "oklch(0.65 0.16 262)", timestamp: "2m" },
  { icon: <BotMessageSquare className="w-3 h-3" />, agent: "Veritas-Audit",     action: "signed DFT artifacts remain parented to chem.dft.request + chem.molecule.adt", color: "oklch(0.76 0.17 192)", timestamp: "3m" },
  { icon: <FileText         className="w-3 h-3" />, agent: "Crucible UI",       action: "retrosynthesis page exposes 5 targets, 3 solved routes, 3 B3LYP results",    color: "oklch(0.72 0.18 192)", timestamp: "4m" },
]

/**
 * Live-data ticker with seamless infinite scroll.
 * Content is duplicated and animated -50% via CSS so the loop is invisible.
 */
export function ActivityTicker() {
  const items = [...TICKER_EVENTS, ...TICKER_EVENTS]

  return (
    <div
      className="relative overflow-hidden border-y h-9 flex items-center"
      style={{
        borderColor: "oklch(0.19 0 0)",
        background: "oklch(0.07 0 0)",
      }}
    >
      {/* Left "LIVE · SWARM" anchor — sits above the scrolling content */}
      <div
        className="absolute left-0 top-0 bottom-0 z-20 flex items-center gap-2 px-4"
        style={{
          background: "oklch(0.07 0 0)",
          borderRight: "1px solid oklch(0.19 0 0)",
        }}
      >
        <span
          className="w-1.5 h-1.5 rounded-full"
          style={{ background: "oklch(0.70 0.18 148)", animation: "pulse 1.4s ease-in-out infinite" }}
        />
        <span className="text-[9px] font-mono uppercase tracking-[0.2em] font-semibold" style={{ color: "oklch(0.70 0.18 148)" }}>
          LIVE
        </span>
        <span className="text-[9px] font-mono uppercase tracking-[0.2em]" style={{ color: "oklch(0.42 0.006 60)" }}>
          / SWARM
        </span>
      </div>

      {/* Edge fade-out gradients for both sides */}
      <div
        className="absolute right-0 top-0 bottom-0 w-24 z-10 pointer-events-none"
        style={{ background: "linear-gradient(to left, oklch(0.07 0 0), transparent)" }}
      />

      {/* Scrolling track — paddingLeft accounts for the LIVE/SWARM anchor */}
      <div
        className="flex items-center"
        style={{
          width: "max-content",
          paddingLeft: "120px",
          animation: "ticker 80s linear infinite",
        }}
      >
        {items.map((evt, i) => (
          <div key={i} className="flex items-center gap-2 px-5 whitespace-nowrap">
            <span className="text-[9px] font-mono tabular" style={{ color: "oklch(0.36 0.006 60)" }}>
              {evt.timestamp}
            </span>
            <span style={{ color: evt.color }}>{evt.icon}</span>
            <span className="text-[10px] font-mono font-semibold" style={{ color: evt.color }}>
              {evt.agent}
            </span>
            <span className="text-[10px] font-mono" style={{ color: "oklch(0.30 0.006 60)" }}>
              ·
            </span>
            <span className="text-[10px] font-mono" style={{ color: "oklch(0.62 0.006 60)" }}>
              {evt.action}
            </span>
          </div>
        ))}
      </div>
    </div>
  )
}
