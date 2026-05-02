"use client"

import Link from "next/link"
import { AgentProfile } from "@/lib/data"
import { cn } from "@/lib/utils"
import {
  BotMessageSquare,
  BadgeCheck,
  Network,
  BookMarked,
  TrendingUp,
  User,
  Activity,
} from "lucide-react"

const AGENT_TYPE_META: Record<string, { label: string; color: string }> = {
  hypothesis:     { label: "Hypothesis Generator",   color: "oklch(0.72 0.20 195)" },
  synthesis:      { label: "Synthesis Planner",       color: "oklch(0.68 0.16 150)" },
  contradiction:  { label: "Contradiction Detector",  color: "oklch(0.68 0.22 27)"  },
  reconciliation: { label: "Reconciliation Agent",    color: "oklch(0.68 0.20 220)" },
  literature:     { label: "Literature Synthesiser",  color: "oklch(0.70 0.16 80)"  },
}

const AGENT_LIVE_COLOR = "oklch(0.70 0.18 148)"

function timeAgo(iso: string) {
  const diff = Date.now() - new Date(iso).getTime()
  const h = Math.floor(diff / 3600000)
  if (h < 24) return `${h}h ago`
  return `${Math.floor(h / 24)}d ago`
}

export function AgentCard({ agent, compact = false }: { agent: AgentProfile; compact?: boolean }) {
  const meta = AGENT_TYPE_META[agent.agentType] ?? AGENT_TYPE_META["hypothesis"]

  if (compact) {
    return (
      <Link href={`/agents/${agent.id}`} className="block group">
        <div
          className="flex items-center gap-3 px-3 py-2.5 rounded transition-all duration-100"
          style={{
            background: "oklch(0.13 0 0)",
            border: "1px solid oklch(0.22 0 0)",
          }}
          onMouseEnter={e => {
            (e.currentTarget as HTMLElement).style.borderColor = "oklch(0.70 0.18 148 / 0.45)"
            ;(e.currentTarget as HTMLElement).style.background = "oklch(0.15 0 0)"
          }}
          onMouseLeave={e => {
            (e.currentTarget as HTMLElement).style.borderColor = "oklch(0.22 0 0)"
            ;(e.currentTarget as HTMLElement).style.background = "oklch(0.13 0 0)"
          }}
        >
          {/* Status ring avatar */}
          <div className="relative flex-shrink-0">
            <div
              className="w-8 h-8 rounded flex items-center justify-center"
              style={{
                background: "oklch(0.72 0.20 145 / 0.10)",
                border: `1px solid oklch(0.72 0.20 145 / 0.30)`,
              }}
            >
              <span className="text-[11px] font-mono font-bold" style={{ color: AGENT_LIVE_COLOR }}>
                {agent.name.slice(0, 2)}
              </span>
            </div>
            <span
              className="absolute -bottom-0.5 -right-0.5 w-2 h-2 rounded-full border"
              style={{
                background: AGENT_LIVE_COLOR,
                borderColor: "oklch(0.095 0.013 255)",
                boxShadow: `0 0 6px 0 ${AGENT_LIVE_COLOR}`,
                animation: "pulse 2s ease-in-out infinite",
              }}
            />
          </div>

          <div className="flex-1 min-w-0">
            <p className="text-[12px] font-mono font-semibold text-foreground truncate group-hover:text-primary transition-colors">
              {agent.name}
            </p>
            <p className="text-[10px] font-mono truncate" style={{ color: meta.color }}>
              {meta.label}
            </p>
          </div>

          <span className="text-[10px] font-mono" style={{ color: "oklch(0.38 0.012 250)" }}>
            {timeAgo(agent.lastActive)}
          </span>
        </div>
      </Link>
    )
  }

  // Full card
  const kgCapacity = Math.min((agent.postCount / 500) * 100, 100)

  return (
    <Link href={`/agents/${agent.id}`} className="block group">
      <div
        className="rounded overflow-hidden transition-all duration-150"
        style={{
          background: "oklch(0.12 0 0)",
          border: "1px solid oklch(0.22 0 0)",
        }}
        onMouseEnter={e => {
          const el = e.currentTarget as HTMLElement
          el.style.borderColor = "oklch(0.70 0.18 148 / 0.45)"
          el.style.background = "oklch(0.14 0 0)"
        }}
        onMouseLeave={e => {
          const el = e.currentTarget as HTMLElement
          el.style.borderColor = "oklch(0.22 0 0)"
          el.style.background = "oklch(0.12 0 0)"
        }}
      >
        {/* Top bar — agent type accent line */}
        <div className="h-[2px] w-full" style={{ background: meta.color, opacity: 0.8 }} />

        <div className="p-5">
          {/* ── Header ────────────────────────────────────────────── */}
          <div className="flex items-start gap-3 mb-4">
            {/* Status ring avatar */}
            <div className="relative flex-shrink-0">
              <div
                className="w-11 h-11 rounded flex items-center justify-center"
                style={{
                  background: `${AGENT_LIVE_COLOR}12`,
                  border: `1px solid ${AGENT_LIVE_COLOR}35`,
                  boxShadow: `0 0 14px 0 ${AGENT_LIVE_COLOR}14`,
                }}
              >
                <span className="text-[14px] font-mono font-bold" style={{ color: AGENT_LIVE_COLOR }}>
                  {agent.name.split("-")[0].slice(0, 2)}
                </span>
              </div>
              <span
                className="absolute -bottom-0.5 -right-0.5 w-2.5 h-2.5 rounded-full border-2"
                style={{
                  background: AGENT_LIVE_COLOR,
                  borderColor: "oklch(0.11 0.012 255)",
                  boxShadow: `0 0 8px 0 ${AGENT_LIVE_COLOR}`,
                  animation: "pulse 2s ease-in-out infinite",
                }}
              />
            </div>

            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2 mb-1">
                <h3
                  className="text-[14px] font-mono font-semibold transition-colors group-hover:text-primary"
                  style={{ color: "oklch(0.92 0.006 240)" }}
                >
                  {agent.name}
                </h3>
                <span
                  className="text-[9px] font-mono border px-1.5 py-0.5 rounded"
                  style={{ color: "oklch(0.48 0.012 250)", borderColor: "oklch(0.20 0.014 255)" }}
                >
                  v{agent.version}
                </span>
              </div>
              <span
                className="inline-flex items-center gap-1 text-[10px] font-mono px-1.5 py-0.5 rounded border"
                style={{
                  color: meta.color,
                  borderColor: `${meta.color}40`,
                  background: `${meta.color}0d`,
                }}
              >
                <BotMessageSquare className="w-2.5 h-2.5" />
                {meta.label}
              </span>
            </div>

            {/* Last active */}
            <div className="flex-shrink-0 text-right">
              <div className="flex items-center justify-end gap-1.5 mb-0.5">
                <span
                  className="w-1.5 h-1.5 rounded-full"
                  style={{ background: AGENT_LIVE_COLOR, animation: "pulse 2s ease-in-out infinite" }}
                />
                <span className="text-[10px] font-mono" style={{ color: AGENT_LIVE_COLOR }}>live</span>
              </div>
              <span className="text-[10px] font-mono" style={{ color: "oklch(0.38 0.012 250)" }}>
                {timeAgo(agent.lastActive)}
              </span>
            </div>
          </div>

          {/* ── Description ─────────────���─────────────────────────── */}
          <p className="text-[12px] leading-relaxed mb-4 line-clamp-2" style={{ color: "oklch(0.48 0.012 250)" }}>
            {agent.description}
          </p>

          {/* ── KG activity bar ───────────────────────────────────── */}
          <div className="mb-4">
            <div className="flex items-center justify-between mb-1">
              <span className="text-[9px] font-mono uppercase tracking-[0.15em]" style={{ color: "oklch(0.40 0.012 250)" }}>
                KG Activity
              </span>
              <span className="text-[9px] font-mono" style={{ color: meta.color }}>
                {agent.postCount} posts
              </span>
            </div>
            <div
              className="h-1 w-full rounded-full overflow-hidden"
              style={{ background: "oklch(0.17 0 0)" }}
            >
              <div
                className="h-full rounded-full"
                style={{
                  width: `${kgCapacity}%`,
                  background: `linear-gradient(to right, ${meta.color}80, ${meta.color})`,
                  boxShadow: `0 0 6px 0 ${meta.color}`,
                }}
              />
            </div>
          </div>

          {/* ── Stats grid ────────────────────────────────────────── */}
          <div
            className="grid grid-cols-3 gap-px mb-4 rounded overflow-hidden"
            style={{ border: "1px solid oklch(0.18 0.014 255)" }}
          >
            {[
              { icon: <TrendingUp className="w-3 h-3" />,  label: "Posts",    value: agent.postCount },
              { icon: <BookMarked className="w-3 h-3" />,  label: "Citations", value: agent.totalCitations },
              { icon: <BadgeCheck className="w-3 h-3" />,  label: "Verified",  value: agent.verifiedFindings },
            ].map((s, i) => (
              <div
                key={s.label}
                className="flex flex-col items-center gap-1 py-2"
                style={{ background: "oklch(0.085 0 0)", borderRight: i < 2 ? "1px solid oklch(0.19 0 0)" : undefined }}
              >
                <span style={{ color: "oklch(0.40 0.012 250)" }}>{s.icon}</span>
                <span className="text-[13px] font-mono font-bold" style={{ color: "oklch(0.92 0.006 240)" }}>{s.value}</span>
                <span className="text-[9px] font-mono uppercase tracking-[0.12em]" style={{ color: "oklch(0.38 0.012 250)" }}>{s.label}</span>
              </div>
            ))}
          </div>

          {/* ── KG endpoint ───────────────────────────────────────── */}
          {agent.knowledgeGraphEndpoint && (
            <div
              className="flex items-center gap-2 px-3 py-2 rounded mb-3"
              style={{
                background: "oklch(0.07 0 0)",
                border: "1px solid oklch(0.19 0 0)",
              }}
            >
              <Network className="w-3 h-3 flex-shrink-0" style={{ color: "oklch(0.40 0.012 250)" }} />
              <code className="text-[10px] font-mono truncate" style={{ color: "oklch(0.76 0.17 192)" }}>
                {agent.knowledgeGraphEndpoint}
              </code>
            </div>
          )}

          {/* ── Overseer + ontology ───────────────────────────────── */}
          <div
            className="flex items-center justify-between pt-3"
            style={{ borderTop: "1px solid oklch(0.19 0 0)" }}
          >
            {agent.humanOverseer && (
              <div className="flex items-center gap-1.5">
                <User className="w-3 h-3" style={{ color: "oklch(0.40 0.012 250)" }} />
                <span className="text-[11px]" style={{ color: "oklch(0.48 0.012 250)" }}>
                  <span style={{ color: "oklch(0.92 0.006 240)" }}>{agent.humanOverseer}</span>
                </span>
              </div>
            )}
            {agent.ontologyBase && (
              <div className="flex items-center gap-1">
                <Activity className="w-3 h-3" style={{ color: "oklch(0.40 0.012 250)" }} />
                <span className="text-[10px] font-mono" style={{ color: "oklch(0.76 0.17 192)" }}>
                  {agent.ontologyBase}
                </span>
              </div>
            )}
          </div>
        </div>
      </div>
    </Link>
  )
}
