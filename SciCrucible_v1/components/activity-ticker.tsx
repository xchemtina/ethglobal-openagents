"use client"

import {
  Activity,
  AlertTriangle,
  Database,
  Zap,
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
  { icon: <BotMessageSquare className="w-3 h-3" />, agent: "Curie-α",     action: "extracted 47 claims from Nature Chem. (DOI 10.1038/...)",   color: "oklch(0.72 0.18 192)", timestamp: "08s" },
  { icon: <AlertTriangle    className="w-3 h-3" />, agent: "Werner-ζ",    action: "claim conflict detected — paper-008 vs KG node n_4271",     color: "oklch(0.65 0.20 25)",  timestamp: "23s" },
  { icon: <Database         className="w-3 h-3" />, agent: "Babbage-θ",   action: "ingested 3 ChemRxiv preprints — organometallic catalysis",  color: "oklch(0.67 0.15 172)", timestamp: "47s" },
  { icon: <Zap              className="w-3 h-3" />, agent: "Boltzmann-β", action: "computed 142 KG nodes — physical chemistry sector",         color: "oklch(0.68 0.15 155)", timestamp: "1m"  },
  { icon: <Activity         className="w-3 h-3" />, agent: "Faraday-η",   action: "DFT benchmark complete — r2SCAN-3c MAD = 2.1 kcal·mol⁻¹",   color: "oklch(0.70 0.18 28)",  timestamp: "1m" },
  { icon: <GitBranch        className="w-3 h-3" />, agent: "Pauling-γ",   action: "verified 12 derivations against PRL archive",               color: "oklch(0.65 0.16 262)", timestamp: "2m" },
  { icon: <Layers           className="w-3 h-3" />, agent: "Leibniz-Σ",   action: "cross-sector synthesis — 8 KG nodes linked across sectors", color: "oklch(0.76 0.17 192)", timestamp: "2m" },
  { icon: <FileText         className="w-3 h-3" />, agent: "Helmholtz-ε", action: "5 thermo. claims awaiting peer review",                     color: "oklch(0.72 0.16 78)",  timestamp: "3m" },
  { icon: <BotMessageSquare className="w-3 h-3" />, agent: "Curie-α",     action: "Crossref dedup — 2 ChemRxiv preprints matched to journal",  color: "oklch(0.72 0.18 192)", timestamp: "4m" },
  { icon: <Database         className="w-3 h-3" />, agent: "Werner-ζ",    action: "wrote 89 OWL triples to inorganic-chemistry KG namespace",  color: "oklch(0.70 0.18 28)",  timestamp: "5m" },
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
