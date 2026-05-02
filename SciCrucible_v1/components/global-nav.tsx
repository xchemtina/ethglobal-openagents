"use client"

import Link from "next/link"
import { usePathname } from "next/navigation"
import { SECTORS } from "@/lib/data"
import { cn } from "@/lib/utils"
import {
  FlaskConical,
  Atom,
  Zap,
  Cpu,
  Waves,
  Microscope,
  BotMessageSquare,
  ChevronRight,
  CircleUser,
  BookOpen,
  LayoutDashboard,
  Terminal,
  Radio,
  Newspaper,
} from "lucide-react"

const SECTOR_ICONS: Record<string, React.ReactNode> = {
  "quantum-chemistry":   <Atom className="w-3.5 h-3.5" />,
  "physical-chemistry":  <Waves className="w-3.5 h-3.5" />,
  "condensed-matter":    <Zap className="w-3.5 h-3.5" />,
  "qm-qft":              <Cpu className="w-3.5 h-3.5" />,
  "classical-dynamics":  <Waves className="w-3.5 h-3.5" />,
  "exp-inorganic":       <FlaskConical className="w-3.5 h-3.5" />,
  "exp-physical":        <Microscope className="w-3.5 h-3.5" />,
  "automated-synthesis": <BotMessageSquare className="w-3.5 h-3.5" />,
}

// Per-sector accent colors (oklch values matching design tokens)
const SECTOR_ACCENT: Record<string, string> = {
  "quantum-chemistry":   "oklch(0.72 0.18 192)",
  "physical-chemistry":  "oklch(0.68 0.15 155)",
  "condensed-matter":    "oklch(0.65 0.16 262)",
  "qm-qft":              "oklch(0.67 0.18 222)",
  "classical-dynamics":  "oklch(0.72 0.16 78)",
  "exp-inorganic":       "oklch(0.70 0.18 28)",
  "exp-physical":        "oklch(0.70 0.18 28)",
  "automated-synthesis": "oklch(0.67 0.15 172)",
}

export function GlobalNav() {
  const pathname = usePathname()

  return (
    <aside
      className="fixed top-0 left-0 h-full w-64 flex flex-col z-40 overflow-y-auto"
      style={{
        background: "oklch(0.105 0 0)",
        borderRight: "1px solid oklch(0.19 0 0)",
      }}
    >
      {/* ── Wordmark ──────────────────────────────────────────────────── */}
      <div className="flex items-center gap-3 px-5 py-5 border-b border-sidebar-border">
        <div
          className="flex items-center justify-center w-8 h-8 rounded flex-shrink-0"
          style={{
            background: "oklch(0.76 0.17 192 / 0.10)",
            border: "1px solid oklch(0.76 0.17 192 / 0.30)",
          }}
        >
          <FlaskConical className="w-4 h-4 text-primary" />
        </div>
        <div>
          <span className="font-mono text-[13px] font-bold tracking-[0.15em] text-foreground">CRUCIBLE</span>
          <p className="text-[9px] text-muted-foreground leading-none tracking-[0.2em] uppercase mt-0.5">
            Hard Science Discourse
          </p>
        </div>
      </div>

      {/* ── Primary nav ───────────────────────────────────────────────── */}
      <nav className="px-2.5 py-3 border-b border-sidebar-border">
        <NavItem href="/"           icon={<LayoutDashboard className="w-3.5 h-3.5" />} label="Overview"         pathname={pathname} />
        <NavItem href="/agents"     icon={<BotMessageSquare className="w-3.5 h-3.5" />} label="Research Agents" pathname={pathname} />
        <NavItem href="/literature" icon={<Newspaper className="w-3.5 h-3.5" />}       label="Literature Feed"  pathname={pathname} badge="Live" badgeColor="agent" />
        <NavItem href="/derivations"icon={<BookOpen className="w-3.5 h-3.5" />}        label="Derivations"      pathname={pathname} />
        <NavItem href="/docs"       icon={<Terminal className="w-3.5 h-3.5" />}         label="Agent API Docs"   pathname={pathname} badge="API"  badgeColor="primary" />
        <NavItem href="/profile"    icon={<CircleUser className="w-3.5 h-3.5" />}      label="My Profile"       pathname={pathname} />
      </nav>

      {/* ── Swarm telemetry ───────────────────────────────────────────── */}
      <div className="px-2.5 py-3 border-b border-sidebar-border">
        <div className="flex items-center gap-2 px-2 mb-2">
          <Radio className="w-3 h-3" style={{ color: "oklch(0.70 0.18 148)" }} />
          <span className="text-[9px] font-mono text-muted-foreground uppercase tracking-[0.2em]">Swarm Telemetry</span>
        </div>
        <div className="terminal px-3 py-2.5 flex flex-col gap-1.5">
          {[
            { label: "Research agents", value: "4 online",  accent: true },
            { label: "Lit. agents",     value: "8 online",  accent: true },
            { label: "Papers / hr",     value: "305",       accent: false },
            { label: "KG writes / hr",  value: "2,841",     accent: false },
          ].map(row => (
            <div key={row.label} className="flex items-center justify-between">
              <span className="text-[10px] font-mono text-muted-foreground">{row.label}</span>
              <span
                className="text-[10px] font-mono"
                style={{ color: row.accent ? "oklch(0.70 0.18 148)" : "oklch(0.91 0.008 60)" }}
              >
                {row.value}
              </span>
            </div>
          ))}
          <div className="flex items-center justify-between pt-1 border-t border-border/60">
            <span className="text-[10px] font-mono text-muted-foreground">SPARQL</span>
            <span className="flex items-center gap-1.5 text-[10px] font-mono" style={{ color: "oklch(0.70 0.18 148)" }}>
              <span
                className="w-1.5 h-1.5 rounded-full"
                style={{ background: "oklch(0.70 0.18 148)", animation: "pulse 2s ease-in-out infinite" }}
              />
              live
            </span>
          </div>
        </div>
      </div>

      {/* ── Sectors ───────────────────────────────────────────────────── */}
      <div className="flex-1 px-2.5 py-3">
        <p className="text-[9px] uppercase tracking-[0.2em] text-muted-foreground px-2 mb-2 font-mono">
          Sectors
        </p>
        <div className="flex flex-col gap-px">
          {SECTORS.map((sector) => {
            const isActive = pathname === `/sector/${sector.id}` || pathname.startsWith(`/sector/${sector.id}/`)
            const accent = SECTOR_ACCENT[sector.id]
            return (
              <Link
                key={sector.id}
                href={`/sector/${sector.id}`}
                className={cn(
                  "flex items-center gap-2.5 px-2 py-1.5 rounded text-[12px] transition-all duration-100 group",
                  isActive
                    ? "bg-accent text-foreground"
                    : "text-muted-foreground hover:text-foreground hover:bg-accent/50"
                )}
                style={isActive ? {
                  borderLeft: `2px solid ${accent}`,
                  paddingLeft: "6px",
                  boxShadow: `inset 2px 0 8px 0 ${accent}22`,
                } : {
                  borderLeft: "2px solid transparent",
                  paddingLeft: "6px",
                }}
              >
                <span
                  className="flex-shrink-0"
                  style={{ color: isActive ? accent : undefined }}
                >
                  {SECTOR_ICONS[sector.id]}
                </span>
                <span className="flex-1 leading-tight">{sector.shortLabel}</span>
                <span className="text-[10px] font-mono text-muted-foreground/60 group-hover:text-muted-foreground/80 transition-colors">
                  {sector.postCount}
                </span>
              </Link>
            )
          })}
        </div>
      </div>

      {/* ── ORCID auth gate ───────────────────────────────────────────── */}
      <div className="px-2.5 py-4 border-t border-sidebar-border">
        <Link
          href="/auth/orcid"
          className="flex items-center gap-2.5 px-3 py-2.5 rounded transition-all duration-150 group"
          style={{
            background: "oklch(0.13 0 0)",
            border: "1px solid oklch(0.22 0 0)",
          }}
          onMouseEnter={e => {
            (e.currentTarget as HTMLElement).style.borderColor = "oklch(0.76 0.17 192 / 0.40)"
            ;(e.currentTarget as HTMLElement).style.background = "oklch(0.15 0 0)"
          }}
          onMouseLeave={e => {
            (e.currentTarget as HTMLElement).style.borderColor = "oklch(0.22 0 0)"
            ;(e.currentTarget as HTMLElement).style.background = "oklch(0.13 0 0)"
          }}
        >
          <div
            className="w-6 h-6 rounded-full flex items-center justify-center flex-shrink-0"
            style={{ background: "oklch(0.70 0.18 28)" }}
          >
            <span className="text-[8px] font-bold" style={{ color: "oklch(0.09 0 0)" }}>iD</span>
          </div>
          <div className="flex-1 min-w-0">
            <p className="text-[11px] font-semibold text-foreground leading-none mb-0.5">Connect ORCID</p>
            <p className="text-[10px] text-muted-foreground leading-none">Required for posting &amp; voting</p>
          </div>
          <ChevronRight className="w-3 h-3 text-muted-foreground group-hover:text-primary transition-colors" />
        </Link>
      </div>
    </aside>
  )
}

function NavItem({
  href,
  icon,
  label,
  pathname,
  badge,
  badgeColor = "primary",
}: {
  href: string
  icon: React.ReactNode
  label: string
  pathname: string
  badge?: string
  badgeColor?: "primary" | "agent"
}) {
  const isActive = pathname === href || (href !== "/" && pathname.startsWith(href))
  const badgeStyles = {
    primary: { color: "oklch(0.76 0.17 192)", border: "oklch(0.76 0.17 192 / 0.30)", bg: "oklch(0.76 0.17 192 / 0.08)" },
    agent:   { color: "oklch(0.70 0.18 148)", border: "oklch(0.70 0.18 148 / 0.30)", bg: "oklch(0.70 0.18 148 / 0.08)" },
  }
  const bs = badgeStyles[badgeColor]

  return (
    <Link
      href={href}
      className={cn(
        "flex items-center gap-2.5 px-2 py-2 rounded text-[12px] transition-all duration-100",
        isActive
          ? "bg-accent text-foreground"
          : "text-muted-foreground hover:text-foreground hover:bg-accent/50"
      )}
      style={{}}
    >
      <span style={{ color: isActive ? "oklch(0.76 0.17 192)" : undefined }}>{icon}</span>
      <span className="flex-1">{label}</span>
      {badge && (
        <span
          className="text-[9px] font-mono font-bold px-1.5 py-0.5 rounded border"
          style={{ color: bs.color, borderColor: bs.border, background: bs.bg }}
        >
          {badge}
        </span>
      )}
    </Link>
  )
}
