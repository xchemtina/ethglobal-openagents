"use client"

import Link from "next/link"
import { POSTS, SECTORS, AGENTS } from "@/lib/data"
import { GlobalNav } from "@/components/global-nav"
import { PostCard } from "@/components/post-card"
import { AgentCard } from "@/components/agent-card"
import { ActivityTicker } from "@/components/activity-ticker"
import { Sparkline, generateSparkData } from "@/components/sparkline"
import { Reticle, SectionStamp, BlockBar } from "@/components/reticle"
import {
  ArrowRight,
  Radio,
  Newspaper,
  ArrowUpRight,
} from "lucide-react"

interface SecondaryStat {
  id: string
  label: string
  value: string
  sublabel: string
  color: string
  fill: number          // 0..1, drives the BlockBar
  trendDir: "up" | "down" | "flat"
  sparkSeed: number
  sparkBase: number
  sparkAmp: number
  sparkTrend: number
}

const SECONDARY_STATS: SecondaryStat[] = [
  {
    id: "M.01",
    label: "PEER-REVIEWED",
    value: "2,847",
    sublabel: "+9 / 24h",
    color: "oklch(0.70 0.18 148)",
    fill: 0.69,
    trendDir: "up",
    sparkSeed: 1117,
    sparkBase: 95,
    sparkAmp: 14,
    sparkTrend: 1.0,
  },
  {
    id: "M.02",
    label: "ACTIVE AGENTS",
    value: "12",
    sublabel: "4R · 8L",
    color: "oklch(0.67 0.18 222)",
    fill: 0.83,
    trendDir: "flat",
    sparkSeed: 2208,
    sparkBase: 12,
    sparkAmp: 4,
    sparkTrend: 0,
  },
  {
    id: "M.03",
    label: "OPEN PROBLEMS",
    value: "312",
    sublabel: "-2 / 24h",
    color: "oklch(0.72 0.16 78)",
    fill: 0.42,
    trendDir: "down",
    sparkSeed: 3301,
    sparkBase: 320,
    sparkAmp: 8,
    sparkTrend: -0.3,
  },
  {
    id: "M.04",
    label: "KG TRIPLES",
    value: "18.7M",
    sublabel: "+24K / 24h",
    color: "oklch(0.65 0.16 262)",
    fill: 0.77,
    trendDir: "up",
    sparkSeed: 4422,
    sparkBase: 18.5,
    sparkAmp: 0.3,
    sparkTrend: 0.012,
  },
]

export default function HomePage() {
  const recentPosts    = POSTS.slice(0, 5)
  const featuredAgents = AGENTS.slice(0, 2)

  // Pre-computed once at module-eval; deterministic per seed
  const heroSpark = generateSparkData(2024, 30, 130, 22, 1.2)

  return (
    <div className="flex min-h-screen bg-background">
      <GlobalNav />

      <main className="flex-1 ml-64 min-h-screen">

        {/* ── Command bar — minimal, sticky ─────────────────────────────── */}
        <header
          className="sticky top-0 z-30 px-8 h-11 flex items-center justify-between"
          style={{
            background: "oklch(0.09 0 0 / 0.92)",
            backdropFilter: "blur(10px)",
            borderBottom: "1px solid oklch(0.19 0 0)",
          }}
        >
          <div className="flex items-center gap-3">
            <Reticle variant="cross" size={10} />
            <span className="text-[10px] font-mono font-semibold tracking-[0.18em]" style={{ color: "oklch(0.76 0.17 192)" }}>
              CRUCIBLE
            </span>
            <span className="text-[10px] font-mono" style={{ color: "oklch(0.30 0.006 60)" }}>·</span>
            <span className="text-[10px] font-mono uppercase tracking-[0.18em] text-foreground">overview</span>

            <div className="flex items-center gap-1.5 ml-4">
              <span className="tag-inverse-green tag-inverse">
                <span className="w-1 h-1 rounded-full" style={{ background: "oklch(0.06 0 0)" }} />
                operational
              </span>
              <span className="text-[10px] font-mono ml-2 tabular" style={{ color: "oklch(0.40 0.006 60)" }}>
                uptime 99.97%
              </span>
            </div>
          </div>

          <div className="flex items-center gap-2">
            <Link
              href="/literature"
              className="flex items-center gap-1.5 px-3 h-7 rounded text-[10px] font-mono uppercase tracking-[0.15em] transition-all duration-100"
              style={{ color: "oklch(0.62 0.006 60)", border: "1px solid oklch(0.22 0 0)", background: "oklch(0.12 0 0)" }}
            >
              <Newspaper className="w-3 h-3" />
              Literature
            </Link>
            <Link
              href="/submit"
              className="flex items-center gap-1.5 px-3 h-7 text-[10px] font-mono uppercase tracking-[0.15em] font-bold transition-all duration-100 clip-bevel-sm"
              style={{
                color: "oklch(0.06 0 0)",
                background: "oklch(0.76 0.17 192)",
              }}
            >
              Submit Post
              <ArrowRight className="w-3 h-3" />
            </Link>
          </div>
        </header>

        {/* ── Live activity ticker — Bloomberg-style ─────────────────────── */}
        <ActivityTicker />

        {/* ── Hero panel — restrained sci-fi: one corner bracket pair, one
              subtle radial wash, no overlay clutter on the content area ──── */}
        <section
          className="relative px-8 pt-7 pb-12"
          style={{
            borderBottom: "1px solid oklch(0.19 0 0)",
            backgroundImage:
              "radial-gradient(ellipse 720px 360px at 100% 0%, oklch(0.76 0.17 192 / 0.05), transparent 60%)",
          }}
        >
          {/* A single pair of HUD corner brackets — top-right & bottom-left only.
              Rule: pick ONE distinctive sci-fi move per panel, then stop. */}
          <span className="corner-bracket tr" style={{ top: 12, right: 12 }} />
          <span className="corner-bracket bl" style={{ bottom: 12, left: 12 }} />

          {/* Coordinate / telemetry header — runs full width */}
          <div
            className="relative flex items-center justify-between mb-8 pb-3"
            style={{ borderBottom: "1px solid oklch(0.18 0 0)" }}
          >
            <div className="flex items-center gap-3 font-mono text-[9px] uppercase tracking-[0.22em] tabular" style={{ color: "oklch(0.46 0.006 60)" }}>
              <span style={{ color: "oklch(0.76 0.17 192)" }}>// CRUCIBLE.SCI</span>
              <span style={{ color: "oklch(0.26 0.006 60)" }}>·</span>
              <span>OBS-04</span>
              <span style={{ color: "oklch(0.26 0.006 60)" }}>·</span>
              <span style={{ color: "oklch(0.62 0.006 60)" }}>2025.115T09:42:18Z</span>
              <span style={{ color: "oklch(0.26 0.006 60)" }}>·</span>
              <span>SYNC OK</span>
            </div>
            <div className="flex items-center gap-2">
              <span className="tag-inverse">RX 305/h</span>
              <span className="tag-inverse-green tag-inverse">TX 2,841/h</span>
            </div>
          </div>

          <div className="grid grid-cols-12 gap-8 max-w-[1280px] relative">

            {/* ── Primary hero metric ─────────────────────────────────── */}
            <div className="col-span-12 lg:col-span-7">
              <div className="flex items-center gap-3 mb-5">
                <SectionStamp index={1} total={4} label="OVERVIEW" />
              </div>

              {/* The hero number stands alone — no flanking ASCII chart, no
                  reticle anchor. Chakra Petch at 132px IS the visual anchor. */}
              <h1
                className="font-numeric-bold leading-[0.92] text-foreground"
                style={{ fontSize: "clamp(78px, 11vw, 128px)", marginLeft: "-0.02em" }}
              >
                4,130
              </h1>

              <div className="flex flex-wrap items-center gap-3 mt-3">
                <span className="text-[10px] font-mono uppercase tracking-[0.25em] font-semibold" style={{ color: "oklch(0.74 0.006 60)" }}>
                  Posts indexed
                </span>
                <span className="text-[10px] font-mono" style={{ color: "oklch(0.28 0.006 60)" }}>·</span>
                <span className="text-[10px] font-mono uppercase tracking-[0.22em]" style={{ color: "oklch(0.50 0.006 60)" }}>
                  across 8 sectors
                </span>
                <span className="tag-inverse-green tag-inverse">
                  <ArrowUpRight className="w-2.5 h-2.5" strokeWidth={3} />
                  +18 today
                </span>
              </div>

              <div className="mt-6 flex items-end gap-5">
                <Sparkline
                  data={heroSpark}
                  width={320}
                  height={48}
                  color="oklch(0.76 0.17 192)"
                />
                <div className="flex flex-col gap-1 pb-1.5">
                  <span className="text-[8.5px] font-mono uppercase tracking-[0.28em]" style={{ color: "oklch(0.36 0.006 60)" }}>
                    past 30d
                  </span>
                  <span className="text-[10px] font-mono tabular" style={{ color: "oklch(0.58 0.006 60)" }}>
                    μ 142  ·  σ 12.3
                  </span>
                </div>
              </div>

              {/* Tagline */}
              <p
                className="mt-7 max-w-[480px] text-[13px] leading-[1.6]"
                style={{ color: "oklch(0.62 0.006 60)" }}
              >
                A rigorous, machine-readable platform for hard chemistry and physics.{" "}
                <span className="font-display italic" style={{ color: "oklch(0.85 0.008 60)" }}>
                  Derivations, open problems, experimental data
                </span>
                {" "}— and a swarm of autonomous research agents reading the literature in real time.
              </p>
            </div>

            {/* ── Secondary stats — 2x2 module cards ────────────────── */}
            <div className="col-span-12 lg:col-span-5 grid grid-cols-2 gap-2.5 self-end">
              {SECONDARY_STATS.map((s) => (
                <ModuleCard key={s.id} stat={s} />
              ))}
            </div>
          </div>
        </section>

        {/* ── ORCID banner — slimmer, integrated ─────────────────────────── */}
        <div
          className="px-8 py-3 flex items-center justify-between gap-4"
          style={{
            background: "oklch(0.70 0.18 28 / 0.04)",
            borderBottom: "1px solid oklch(0.70 0.18 28 / 0.20)",
          }}
        >
          <div className="flex items-center gap-3">
            <div
              className="w-6 h-6 rounded-full flex items-center justify-center flex-shrink-0"
              style={{ background: "oklch(0.70 0.18 28)" }}
            >
              <span className="text-[8px] font-bold" style={{ color: "oklch(0.09 0 0)" }}>iD</span>
            </div>
            <span className="tag-inverse-coral tag-inverse">AUTH REQ</span>
            <p className="text-[11px]" style={{ color: "oklch(0.74 0.006 60)" }}>
              <span className="font-semibold">ORCID verification required</span>
              <span style={{ color: "oklch(0.50 0.006 60)" }}>
                {" — posting, voting, peer review and agent spawning gated by verified researcher identity"}
              </span>
            </p>
          </div>
          <Link
            href="/auth/orcid"
            className="flex-shrink-0 flex items-center gap-1.5 px-3 h-7 text-[10px] font-mono uppercase tracking-[0.18em] font-bold transition-opacity hover:opacity-90 clip-bevel-sm"
            style={{
              background: "oklch(0.70 0.18 28)",
              color: "oklch(0.06 0 0)",
            }}
          >
            Connect <ArrowRight className="w-3 h-3" />
          </Link>
        </div>

        {/* ── Main two-column layout ─────────────────────────────────────── */}
        <div className="px-8 py-8 max-w-[1280px]">
          <div className="flex gap-6">

            {/* Feed */}
            <div className="flex-1 min-w-0">
              <div className="flex items-center justify-between mb-4">
                <div className="flex items-center gap-3">
                  <SectionStamp index={2} total={4} label="RECENT POSTS" />
                  <span className="text-[10px] font-mono" style={{ color: "oklch(0.26 0.006 60)" }}>·</span>
                  <span className="text-[10px] font-mono uppercase tracking-[0.18em]" style={{ color: "oklch(0.48 0.006 60)" }}>
                    cross-sector
                  </span>
                </div>
                <Link
                  href="/sector/quantum-chemistry"
                  className="text-[10px] font-mono uppercase tracking-[0.18em] flex items-center gap-1 transition-colors hover:text-foreground"
                  style={{ color: "oklch(0.50 0.006 60)" }}
                >
                  Browse all <ArrowRight className="w-3 h-3" />
                </Link>
              </div>

              <div className="flex flex-col gap-2.5">
                {recentPosts.map((post) => (
                  <PostCard key={post.id} post={post} />
                ))}
              </div>
            </div>

            {/* Sidebar */}
            <aside className="w-72 flex-shrink-0 flex flex-col gap-4">

              {/* Sectors */}
              <div
                className="rounded overflow-hidden relative"
                style={{
                  background: "oklch(0.115 0 0)",
                  border: "1px solid oklch(0.20 0 0)",
                }}
              >
                <div
                  className="px-4 py-2.5 flex items-center justify-between"
                  style={{ borderBottom: "1px solid oklch(0.18 0 0)" }}
                >
                  <SectionStamp index={3} total={4} label="SECTORS" />
                  <span className="text-[9px] font-mono tabular" style={{ color: "oklch(0.36 0.006 60)" }}>
                    08
                  </span>
                </div>
                <div className="flex flex-col">
                  {SECTORS.map((sector, i) => (
                    <Link
                      key={sector.id}
                      href={`/sector/${sector.id}`}
                      className="flex items-center justify-between px-4 py-2 transition-all duration-100 group"
                      style={{ background: "transparent", borderTop: "1px solid oklch(0.15 0 0)" }}
                      onMouseEnter={e => { (e.currentTarget as HTMLElement).style.background = "oklch(0.15 0 0)" }}
                      onMouseLeave={e => { (e.currentTarget as HTMLElement).style.background = "transparent" }}
                    >
                      <div className="flex items-center gap-2 min-w-0">
                        <span className="font-mono text-[8px] tabular" style={{ color: "oklch(0.32 0.006 60)" }}>
                          {String(i + 1).padStart(2, "0")}
                        </span>
                        <span className="text-[12px] text-muted-foreground group-hover:text-foreground transition-colors truncate">
                          {sector.shortLabel}
                        </span>
                      </div>
                      <span className="text-[10px] font-mono tabular" style={{ color: "oklch(0.46 0.006 60)" }}>
                        {sector.postCount}
                      </span>
                    </Link>
                  ))}
                </div>
              </div>

              {/* Swarm panel — scope readout style */}
              <div
                className="rounded px-4 py-3 relative"
                style={{
                  background: "oklch(0.115 0 0)",
                  border: "1px solid oklch(0.20 0 0)",
                }}
              >
                <div className="flex items-center justify-between mb-3">
                  <div className="flex items-center gap-2">
                    <Radio className="w-3 h-3" style={{ color: "oklch(0.70 0.18 148)" }} />
                    <SectionStamp index={4} total={4} label="SWARM" />
                  </div>
                  <span className="tag-inverse-green tag-inverse">CH-04</span>
                </div>

                {/* Live frequency preview */}
                <div
                  className="px-3 py-2.5 rounded mb-2 relative overflow-hidden"
                  style={{ background: "oklch(0.07 0 0)", border: "1px solid oklch(0.18 0 0)" }}
                >
                  <Sparkline
                    data={generateSparkData(7777, 32, 280, 50, 1.5)}
                    width={232}
                    height={36}
                    color="oklch(0.70 0.18 148)"
                  />
                  <span
                    className="absolute top-1 right-2 font-mono text-[8px] tabular"
                    style={{ color: "oklch(0.40 0.006 60)" }}
                  >
                    1.2 kHz
                  </span>
                </div>

                <div
                  className="flex flex-col gap-1.5 font-mono text-[10px] px-3 py-2.5 rounded"
                  style={{ background: "oklch(0.07 0 0)", border: "1px solid oklch(0.18 0 0)" }}
                >
                  {[
                    { k: "Research agents", v: "4 live", fill: 0.50 },
                    { k: "Lit. agents",     v: "8 live", fill: 1.00 },
                    { k: "Papers/hr",       v: "305",   fill: 0.61 },
                    { k: "KG writes/hr",    v: "2,841", fill: 0.71 },
                  ].map(r => (
                    <div key={r.k} className="grid grid-cols-[1fr_auto_auto] gap-2 items-center tabular">
                      <span style={{ color: "oklch(0.46 0.006 60)" }}>{r.k}</span>
                      <BlockBar fill={r.fill} cells={8} color="oklch(0.70 0.18 148)" />
                      <span style={{ color: "oklch(0.70 0.18 148)" }} className="text-right min-w-[44px]">{r.v}</span>
                    </div>
                  ))}
                </div>
              </div>

              {/* Active agents */}
              <div>
                <div className="flex items-center justify-between mb-2.5">
                  <div className="flex items-center gap-2">
                    <Reticle variant="cross" size={9} color="oklch(0.50 0.006 60)" />
                    <p className="text-[9px] font-mono uppercase tracking-[0.22em] font-semibold" style={{ color: "oklch(0.50 0.006 60)" }}>
                      Active Agents
                    </p>
                  </div>
                  <Link
                    href="/agents"
                    className="text-[10px] font-mono uppercase tracking-[0.15em] transition-colors hover:text-foreground"
                    style={{ color: "oklch(0.76 0.17 192)" }}
                  >
                    View all
                  </Link>
                </div>
                <div className="flex flex-col gap-2">
                  {featuredAgents.map((agent) => (
                    <AgentCard key={agent.id} agent={agent} compact />
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

/* ────────────────────────────────────────────────────────────────────────── */
/* ModuleCard — sci-fi stat tile with corner cut, ID stamp, block bar         */
/* ────────────────────────────────────────────────────────────────────────── */

function ModuleCard({ stat }: { stat: SecondaryStat }) {
  const trendArrow = stat.trendDir === "up" ? "▲" : stat.trendDir === "down" ? "▼" : "—"

  return (
    <div
      className="relative px-3.5 py-3 transition-all duration-150 clip-bevel-tl-br"
      style={{
        background: "oklch(0.115 0 0)",
        border: "1px solid oklch(0.21 0 0)",
      }}
      onMouseEnter={e => {
        (e.currentTarget as HTMLElement).style.background = "oklch(0.135 0 0)"
        ;(e.currentTarget as HTMLElement).style.borderColor = `${stat.color.replace(")", " / 0.40)")}`
      }}
      onMouseLeave={e => {
        (e.currentTarget as HTMLElement).style.background = "oklch(0.115 0 0)"
        ;(e.currentTarget as HTMLElement).style.borderColor = "oklch(0.21 0 0)"
      }}
    >
      {/* Top: module ID + label + trend tag */}
      <div className="flex items-center justify-between mb-2.5">
        <div className="flex items-center gap-2">
          <span className="font-mono text-[8px] tabular font-bold" style={{ color: stat.color }}>
            {stat.id}
          </span>
          <span className="font-mono text-[8px]" style={{ color: "oklch(0.24 0.006 60)" }}>┃</span>
          <span className="text-[9px] font-mono uppercase tracking-[0.18em] font-semibold" style={{ color: "oklch(0.56 0.006 60)" }}>
            {stat.label}
          </span>
        </div>
        <span className="font-mono text-[9px] tabular flex items-center gap-1" style={{ color: stat.color }}>
          <span style={{ fontSize: 7 }}>{trendArrow}</span>
          {stat.sublabel}
        </span>
      </div>

      {/* Number + sparkline row */}
      <div className="flex items-end justify-between gap-3 mb-2">
        <span
          className="font-numeric-bold leading-[0.9]"
          style={{ fontSize: "38px", color: "oklch(0.93 0.008 60)" }}
        >
          {stat.value}
        </span>
        <Sparkline
          data={generateSparkData(stat.sparkSeed, 24, stat.sparkBase, stat.sparkAmp, stat.sparkTrend)}
          width={84}
          height={26}
          color={stat.color}
        />
      </div>

      {/* Block bar capacity */}
      <div className="flex items-center justify-between gap-2 pt-2" style={{ borderTop: "1px solid oklch(0.18 0 0)" }}>
        <BlockBar fill={stat.fill} cells={14} color={stat.color} />
        <span className="font-mono text-[8px] tabular" style={{ color: "oklch(0.42 0.006 60)" }}>
          {Math.round(stat.fill * 100)}%
        </span>
      </div>
    </div>
  )
}
