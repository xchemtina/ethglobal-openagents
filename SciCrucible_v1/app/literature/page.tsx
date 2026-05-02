"use client"

import { useState } from "react"
import Link from "next/link"
import { GlobalNav } from "@/components/global-nav"
import {
  JOURNALS,
  LITERATURE_AGENTS,
  SECTORS,
  SEEDED_PAPERS,
} from "@/lib/data"
import {
  BookOpen,
  Radio,
  Network,
  AlertTriangle,
  ExternalLink,
  Database,
  Zap,
  FileText,
  ArrowRight,
  ChevronRight,
  ChevronDown,
} from "lucide-react"
import { cn } from "@/lib/utils"

const ACCESS_META: Record<string, { label: string; color: string }> = {
  "open-access":  { label: "OA",     color: "oklch(0.70 0.18 148)" },
  "hybrid":       { label: "Hybrid", color: "oklch(0.72 0.16 78)"  },
  "subscription": { label: "Sub",    color: "oklch(0.65 0.20 25)"  },
}

const STATUS_COLOR: Record<string, string> = {
  live:     "oklch(0.70 0.18 148)",
  backfill: "oklch(0.72 0.16 78)",
  paused:   "oklch(0.50 0.006 60)",
  error:    "oklch(0.65 0.20 25)",
}

const INGEST_COLORS = [
  "oklch(0.72 0.18 192)",
  "oklch(0.70 0.18 148)",
  "oklch(0.72 0.16 78)",
  "oklch(0.65 0.16 262)",
]

function formatRelative(iso: string) {
  const diff = Date.now() - new Date(iso).getTime()
  const mins = Math.floor(diff / 60000)
  if (mins < 60) return `${mins}m ago`
  const hrs = Math.floor(mins / 60)
  if (hrs < 24) return `${hrs}h ago`
  return `${Math.floor(hrs / 24)}d ago`
}

export default function LiteraturePage() {
  const [expandedJournal, setExpandedJournal] = useState<string | null>(null)
  const [expandedAgent, setExpandedAgent]     = useState<string | null>(null)

  const totalPapers    = LITERATURE_AGENTS.reduce((s, a) => s + a.papersProcessed, 0)
  const totalClaims    = LITERATURE_AGENTS.reduce((s, a) => s + a.claimsExtracted, 0)
  const totalNodes     = LITERATURE_AGENTS.reduce((s, a) => s + a.kgNodesCreated, 0)
  const totalRate      = LITERATURE_AGENTS.reduce((s, a) => s + a.processingRatePerHour, 0)
  const totalConflicts = SEEDED_PAPERS.reduce((s, p) => s + p.claimConflicts, 0)

  return (
    <div className="flex min-h-screen bg-background">
      <GlobalNav />

      <main className="flex-1 ml-64 min-h-screen">

        {/* ── Header ──────────────────────────────────────────────────── */}
        <header
          className="sticky top-0 z-30 px-8 py-3 flex items-center justify-between"
          style={{
            background: "oklch(0.08 0.012 255 / 0.92)",
            backdropFilter: "blur(12px)",
            borderBottom: "1px solid oklch(0.22 0 0)",
          }}
        >
          <div className="flex items-center gap-3">
            <div
              className="flex items-center gap-2 px-3 py-1.5 rounded"
              style={{ background: "oklch(0.12 0 0)", border: "1px solid oklch(0.22 0 0)" }}
            >
              <span className="text-[10px] font-mono" style={{ color: "oklch(0.72 0.20 195)" }}>crucible.science</span>
              <span className="text-[10px] font-mono text-muted-foreground">/</span>
              <span className="text-[10px] font-mono text-foreground">literature</span>
            </div>
            <div className="flex items-center gap-1.5">
              <span
                className="w-1.5 h-1.5 rounded-full"
                style={{ background: "oklch(0.72 0.20 145)", animation: "pulse 2s ease-in-out infinite", boxShadow: "0 0 6px oklch(0.72 0.20 145)" }}
              />
              <span className="text-[10px] font-mono" style={{ color: "oklch(0.72 0.20 145)" }}>
                {LITERATURE_AGENTS.length} agents live — {totalRate} papers/hr
              </span>
            </div>
          </div>
          <Link
            href="/docs#literature-agents"
            className="text-[11px] font-mono flex items-center gap-1 transition-colors hover:text-foreground"
            style={{ color: "oklch(0.72 0.20 195)" }}
          >
            Agent API <ArrowRight className="w-3 h-3" />
          </Link>
        </header>

        <div className="px-8 py-6 max-w-6xl">

          {/* ── Swarm stats strip ────────────────────────────────────── */}
          <div className="grid grid-cols-5 gap-2.5 mb-6">
            {[
              { label: "Papers ingested",   value: totalPapers.toLocaleString(),  icon: <FileText className="w-3.5 h-3.5" />, color: "oklch(0.92 0.006 240)" },
              { label: "Claims extracted",  value: totalClaims.toLocaleString(),  icon: <Database className="w-3.5 h-3.5" />, color: "oklch(0.72 0.20 195)" },
              { label: "KG nodes created",  value: totalNodes.toLocaleString(),   icon: <Network className="w-3.5 h-3.5" />,  color: "oklch(0.68 0.18 260)" },
              { label: "Papers / hr",       value: `${totalRate}`,               icon: <Zap className="w-3.5 h-3.5" />,     color: "oklch(0.72 0.20 145)" },
              { label: "Claim conflicts",   value: `${totalConflicts}`,          icon: <AlertTriangle className="w-3.5 h-3.5" />, color: "oklch(0.70 0.16 80)" },
            ].map(s => (
              <div
                key={s.label}
                className="rounded px-4 py-3"
                style={{ background: "oklch(0.12 0 0)", border: "1px solid oklch(0.22 0 0)" }}
              >
                <div className="flex items-center gap-1.5 mb-1.5" style={{ color: s.color }}>
                  {s.icon}
                  <span className="text-[9px] font-mono uppercase tracking-[0.15em]" style={{ color: "oklch(0.42 0.006 60)" }}>
                    {s.label}
                  </span>
                </div>
                <span className="text-[18px] font-mono font-bold" style={{ color: s.color }}>{s.value}</span>
              </div>
            ))}
          </div>

          <div className="flex gap-5">

            {/* ── Agent roster ─────────────────────────────────────────── */}
            <div className="flex-1 min-w-0">
              <div className="flex items-center justify-between mb-3">
                <p className="text-[9px] font-mono uppercase tracking-[0.2em]" style={{ color: "oklch(0.42 0.006 60)" }}>
                  Sector literature agents ({LITERATURE_AGENTS.length})
                </p>
              </div>

              <div className="flex flex-col gap-2">
                {LITERATURE_AGENTS.map(agent => {
                  const sector   = SECTORS.find(s => s.id === agent.sectorId)
                  const expanded = expandedAgent === agent.id
                  const agentJournals = JOURNALS.filter(j => agent.journals.includes(j.id))

                  return (
                    <div
                      key={agent.id}
                      className="rounded overflow-hidden transition-all duration-150"
                      style={{ background: "oklch(0.12 0 0)", border: "1px solid oklch(0.22 0 0)" }}
                    >
                      {/* Row header — always visible */}
                      <div className="flex items-center gap-3 px-4 py-3">

                        {/* Avatar + live dot */}
                        <div className="relative flex-shrink-0">
                          <div
                            className="w-9 h-9 rounded flex items-center justify-center"
                            style={{
                              background: "oklch(0.72 0.20 145 / 0.10)",
                              border: "1px solid oklch(0.72 0.20 145 / 0.30)",
                            }}
                          >
                            <span className="text-[12px] font-mono font-bold" style={{ color: "oklch(0.72 0.20 145)" }}>
                              {agent.name.split("-")[0].slice(0, 2)}
                            </span>
                          </div>
                          <span
                            className="absolute -bottom-0.5 -right-0.5 w-2 h-2 rounded-full border"
                            style={{
                              background: STATUS_COLOR[agent.ingestionStatus],
                              borderColor: "oklch(0.12 0 0)",
                              boxShadow: `0 0 6px ${STATUS_COLOR[agent.ingestionStatus]}`,
                              animation: "pulse 2s ease-in-out infinite",
                            }}
                          />
                        </div>

                        {/* Name + sector */}
                        <div className="min-w-0 flex-1">
                          <div className="flex items-center gap-2">
                            <span className="text-[13px] font-mono font-semibold text-foreground">{agent.name}</span>
                            <span
                              className="text-[9px] font-mono border px-1.5 py-0.5 rounded"
                              style={{ color: "oklch(0.42 0.006 60)", borderColor: "oklch(0.22 0 0)" }}
                            >
                              v{agent.version}
                            </span>
                          </div>
                          <span className="text-[11px]" style={{ color: "oklch(0.50 0.006 60)" }}>
                            {sector?.shortLabel}
                          </span>
                        </div>

                        {/* Inline metrics */}
                        <div className="hidden lg:flex items-center gap-4 mr-4">
                          {[
                            { label: "papers", value: agent.papersProcessed.toLocaleString() },
                            { label: "claims", value: agent.claimsExtracted.toLocaleString() },
                            { label: "/hr",    value: agent.processingRatePerHour },
                          ].map(m => (
                            <div key={m.label} className="text-center">
                              <p className="text-[12px] font-mono font-bold text-foreground">{m.value}</p>
                              <p className="text-[9px] font-mono" style={{ color: "oklch(0.40 0.006 60)" }}>{m.label}</p>
                            </div>
                          ))}
                        </div>

                        {/* Last heartbeat */}
                        <span className="text-[10px] font-mono flex-shrink-0" style={{ color: "oklch(0.40 0.006 60)" }}>
                          {formatRelative(agent.lastHeartbeat)}
                        </span>

                        {/* Expand toggle */}
                        <button
                          onClick={() => setExpandedAgent(expanded ? null : agent.id)}
                          className="flex-shrink-0 p-1 rounded transition-colors hover:bg-accent/50"
                        >
                          {expanded
                            ? <ChevronDown className="w-4 h-4 text-muted-foreground" />
                            : <ChevronRight className="w-4 h-4 text-muted-foreground" />
                          }
                        </button>

                        {/* Sector link */}
                        <Link
                          href={`/literature/${agent.sectorId}`}
                          className="flex-shrink-0 flex items-center gap-1 text-[10px] font-mono px-2 py-1 rounded transition-all"
                          style={{ color: "oklch(0.72 0.20 195)", border: "1px solid oklch(0.72 0.20 195 / 0.25)", background: "oklch(0.72 0.20 195 / 0.06)" }}
                        >
                          Feed <ArrowRight className="w-2.5 h-2.5" />
                        </Link>
                      </div>

                      {/* Expanded detail */}
                      {expanded && (
                        <div
                          className="px-4 pb-4 pt-2"
                          style={{ borderTop: "1px solid oklch(0.17 0 0)" }}
                        >
                          {/* Ingest source bar */}
                          <div className="mb-3">
                            <p className="text-[9px] font-mono uppercase tracking-[0.15em] mb-1.5" style={{ color: "oklch(0.40 0.006 60)" }}>
                              Ingest breakdown
                            </p>
                            <div className="flex h-2 rounded-full overflow-hidden gap-px">
                              {agent.ingestBreakdown.map((b, i) => (
                                <div
                                  key={i}
                                  style={{ width: `${b.fraction * 100}%`, background: INGEST_COLORS[i % 4] }}
                                />
                              ))}
                            </div>
                            <div className="flex flex-wrap gap-x-4 gap-y-0.5 mt-1.5">
                              {agent.ingestBreakdown.map((b, i) => (
                                <span key={i} className="text-[10px] font-mono" style={{ color: "oklch(0.42 0.006 60)" }}>
                                  <span style={{ color: INGEST_COLORS[i % 4] }}>{Math.round(b.fraction * 100)}%</span>{" "}{b.source}
                                </span>
                              ))}
                            </div>
                          </div>

                          {/* ArXiv categories */}
                          <div className="mb-3">
                            <p className="text-[9px] font-mono uppercase tracking-[0.15em] mb-1.5" style={{ color: "oklch(0.40 0.006 60)" }}>
                              ArXiv categories
                            </p>
                            <div className="flex flex-wrap gap-1.5">
                              {agent.arxivCategories.map(cat => (
                                <a
                                  key={cat}
                                  href={`https://arxiv.org/list/${cat}/recent`}
                                  target="_blank"
                                  rel="noreferrer"
                                  className="text-[10px] font-mono px-2 py-0.5 rounded border transition-all hover:opacity-80"
                                  style={{
                                    color: "oklch(0.72 0.20 195)",
                                    borderColor: "oklch(0.72 0.20 195 / 0.30)",
                                    background: "oklch(0.72 0.20 195 / 0.07)",
                                  }}
                                >
                                  {cat}
                                </a>
                              ))}
                            </div>
                          </div>

                          {/* Journals */}
                          <div>
                            <p className="text-[9px] font-mono uppercase tracking-[0.15em] mb-1.5" style={{ color: "oklch(0.40 0.006 60)" }}>
                              Journals monitored
                            </p>
                            <div className="flex flex-wrap gap-1.5">
                              {agentJournals.map(j => {
                                const am = ACCESS_META[j.accessType]
                                return (
                                  <span
                                    key={j.id}
                                    className="text-[10px] font-mono px-2 py-0.5 rounded border"
  style={{ color: "oklch(0.56 0.006 60)", borderColor: "oklch(0.22 0 0)", background: "oklch(0.085 0 0)" }}
  >
  {j.shortName}
  {j.isPreprint && <span className="ml-1" style={{ color: "oklch(0.72 0.16 78)" }}>preprint</span>}
  <span className="ml-1.5" style={{ color: am.color }}>{am.label}</span>
  </span>
                                )
                              })}
                            </div>
                          </div>
                        </div>
                      )}
                    </div>
                  )
                })}
              </div>
            </div>

            {/* ── Right sidebar ────────────────────────────────────────── */}
            <aside className="w-60 flex-shrink-0 flex flex-col gap-4">

              {/* Journal list */}
              <div
                className="rounded overflow-hidden"
                style={{ background: "oklch(0.12 0 0)", border: "1px solid oklch(0.22 0 0)" }}
              >
                <div
                  className="px-4 py-2.5"
                  style={{ borderBottom: "1px solid oklch(0.19 0 0)" }}
                >
                  <p className="text-[9px] font-mono uppercase tracking-[0.2em]" style={{ color: "oklch(0.42 0.006 60)" }}>
                    Seeded journals ({JOURNALS.length})
                  </p>
                </div>
                <div className="flex flex-col">
                  {JOURNALS.map(journal => {
                    const am      = ACCESS_META[journal.accessType]
                    const isOpen  = expandedJournal === journal.id
                    return (
                      <button
                        key={journal.id}
                        onClick={() => setExpandedJournal(isOpen ? null : journal.id)}
                        className="px-4 py-2.5 text-left transition-all duration-100"
                        style={{
                          borderBottom: "1px solid oklch(0.17 0 0)",
                          background: isOpen ? "oklch(0.15 0 0)" : "transparent",
                        }}
                      >
  <div className="flex items-center justify-between gap-2 mb-0.5">
  <span className="flex items-center gap-1.5 text-[11px] font-medium text-foreground leading-snug truncate">
  {journal.shortName}
  {journal.isPreprint && (
    <span className="text-[8px] font-mono border px-1 py-px rounded" style={{ color: "oklch(0.72 0.16 78)", borderColor: "oklch(0.72 0.16 78 / 0.35)", background: "oklch(0.72 0.16 78 / 0.08)" }}>
      preprint
    </span>
  )}
  </span>
                          <span
                            className="text-[8px] font-mono border px-1 py-px rounded flex-shrink-0"
                            style={{ color: am.color, borderColor: `${am.color}44`, background: `${am.color}0d` }}
                          >
                            {am.label}
                          </span>
                        </div>
                        <div className="flex items-center justify-between">
                          <span className="text-[9px] font-mono" style={{ color: "oklch(0.40 0.006 60)" }}>
                            {journal.papersIngested.toLocaleString()} papers
                          </span>
                          <span className="flex items-center gap-0.5 text-[9px] font-mono" style={{ color: "oklch(0.72 0.20 145)" }}>
                            <span
                              className="w-1 h-1 rounded-full"
                              style={{ background: "oklch(0.72 0.20 145)", animation: "pulse 2s ease-in-out infinite" }}
                            />
                            live
                          </span>
                        </div>

                        {isOpen && (
                          <div
                            className="mt-2 pt-2 space-y-1.5"
                            style={{ borderTop: "1px solid oklch(0.19 0 0)" }}
                          >
                            {[
                              { k: "OA fraction", v: `${Math.round(journal.openAccessFraction * 100)}%` },
                              { k: "Avg claims",  v: `${journal.avgClaimsPerPaper} / paper` },
                              { k: "Source",      v: journal.ingestSource },
                              { k: "Last ingest", v: formatRelative(journal.lastIngested) },
                              { k: "eISSN",       v: journal.eissn },
                            ].map(r => (
                              <div key={r.k} className="flex items-center justify-between text-[9px] font-mono">
                                <span style={{ color: "oklch(0.40 0.006 60)" }}>{r.k}</span>
                                <span style={{ color: "oklch(0.72 0.20 195)" }}>{r.v}</span>
                              </div>
                            ))}
                            <a
                              href={`https://www.nature.com/${journal.id}`}
                              target="_blank"
                              rel="noreferrer"
                              className="flex items-center gap-0.5 text-[9px] font-mono hover:underline"
                              style={{ color: "oklch(0.72 0.20 195)" }}
                              onClick={e => e.stopPropagation()}
                            >
                              Homepage <ExternalLink className="w-2 h-2" />
                            </a>
                          </div>
                        )}
                      </button>
                    )
                  })}
                </div>
              </div>

              {/* ArXiv category cloud */}
              <div
                className="rounded px-4 py-3"
                style={{ background: "oklch(0.12 0 0)", border: "1px solid oklch(0.22 0 0)" }}
              >
                <p className="text-[9px] font-mono uppercase tracking-[0.2em] mb-2.5" style={{ color: "oklch(0.42 0.006 60)" }}>
                  ArXiv categories monitored
                </p>
                <div className="flex flex-wrap gap-1.5">
                  {Array.from(new Set(LITERATURE_AGENTS.flatMap(a => a.arxivCategories))).sort().map(cat => (
                    <a
                      key={cat}
                      href={`https://arxiv.org/list/${cat}/recent`}
                      target="_blank"
                      rel="noreferrer"
                      className="text-[9px] font-mono px-2 py-1 rounded border transition-all hover:opacity-80"
                      style={{
                        color: "oklch(0.72 0.20 195)",
                        borderColor: "oklch(0.72 0.20 195 / 0.25)",
                        background: "oklch(0.72 0.20 195 / 0.07)",
                      }}
                    >
                      {cat}
                    </a>
                  ))}
                </div>
              </div>

              {/* Oversight panel */}
              <div
                className="rounded px-4 py-3"
                style={{ background: "oklch(0.12 0 0)", border: "1px solid oklch(0.22 0 0)" }}
              >
                <p className="text-[9px] font-mono uppercase tracking-[0.2em] mb-3" style={{ color: "oklch(0.42 0.006 60)" }}>
                  Human oversight
                </p>
                <div className="flex flex-col gap-3">
                  {[
                    { name: "Prof. Markus Kraft",  orcid: "0000-0002-4283-6901", agents: ["Curie-α", "Bardeen-γ", "Dirac-δ"] },
                    { name: "Prof. Timothy Noel",  orcid: "0000-0002-1814-969X", agents: ["Boltzmann-β", "Poincare-ε", "Werner-ζ", "Faraday-η", "Babbage-θ"] },
                  ].map(o => (
                    <div key={o.orcid}>
                      <p className="text-[11px] font-semibold text-foreground leading-none mb-0.5">{o.name}</p>
                      <a
                        href={`https://orcid.org/${o.orcid}`}
                        target="_blank"
                        rel="noreferrer"
                        className="flex items-center gap-0.5 text-[9px] font-mono hover:underline mb-1"
                        style={{ color: "oklch(0.70 0.20 30)" }}
                      >
                        {o.orcid} <ExternalLink className="w-2 h-2" />
                      </a>
                      <p className="text-[10px]" style={{ color: "oklch(0.40 0.006 60)" }}>
                        {o.agents.join(", ")}
                      </p>
                    </div>
                  ))}
                </div>
              </div>
            </aside>
          </div>
        </div>
      </main>
    </div>
  )
}
