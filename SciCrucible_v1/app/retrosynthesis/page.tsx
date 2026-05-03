import Link from "next/link"
import { GlobalNav } from "@/components/global-nav"
import {
  AlertTriangle,
  Atom,
  ChevronRight,
  Clock,
  Database,
  FlaskConical,
  GitBranch,
  ShieldCheck,
} from "lucide-react"
import {
  loadRetrosynthesisDashboard,
  type AiZynthTarget,
  type B3lypCompletedResult,
} from "@/lib/retrosynthesis-artifacts"
import type { ReactNode } from "react"

export const dynamic = "force-static"

function fmtSeconds(seconds: number): string {
  return `${seconds.toFixed(2)} s`
}

function oklchAlpha(color: string, alpha: number): string {
  return color.replace(")", ` / ${alpha})`)
}

function fmtScore(score: number): string {
  return score.toFixed(4)
}

function fmtEnergy(energy: number): string {
  return `${energy.toFixed(6)} Eh`
}

function splitCsv(value: string): string[] {
  return value
    .split(",")
    .map((part) => part.trim())
    .filter(Boolean)
}

function targetName(target: AiZynthTarget): string {
  return target.target_label
    .replace(/^TARGET\.(\d+)\./, "Target $1 · ")
    .replaceAll("_", " ")
    .toLowerCase()
}

function dftLabel(result: B3lypCompletedResult): string {
  const labels: Record<string, string> = {
    nbs: "N-bromosuccinimide",
    "mesyl-anhydride": "Methanesulfonic anhydride",
    "acetylated-diol": "Acetylated diol precursor",
  }
  return labels[result.label] ?? result.label.replaceAll("-", " ")
}

export default function RetrosynthesisPage() {
  const { routeSummary, dftSummary, stats } = loadRetrosynthesisDashboard()

  return (
    <div className="flex min-h-screen bg-background">
      <GlobalNav />

      <main className="flex-1 ml-64 min-h-screen">
        <header className="sticky top-0 z-30 bg-background/90 backdrop-blur border-b border-border px-8 py-3">
          <nav className="flex items-center gap-1.5 text-[12px] text-muted-foreground font-mono">
            <Link href="/" className="hover:text-foreground transition-colors">
              Crucible
            </Link>
            <ChevronRight className="w-3 h-3" />
            <span className="text-foreground">Retrosynthesis</span>
          </nav>
        </header>

        <section
          className="px-8 pt-7 pb-8"
          style={{ borderBottom: "1px solid oklch(0.19 0 0)" }}
        >
          <div className="flex items-center gap-3 mb-3">
            <GitBranch className="w-4 h-4" style={{ color: "oklch(0.72 0.16 78)" }} />
            <span
              className="text-[10px] font-mono uppercase tracking-[0.2em] font-semibold"
              style={{ color: "oklch(0.72 0.16 78)" }}
            >
              {routeSummary.schema_tag}
            </span>
            <span className="text-[10px] font-mono" style={{ color: "oklch(0.30 0.006 60)" }}>
              ·
            </span>
            <span className="text-[10px] font-mono uppercase tracking-[0.18em]" style={{ color: "oklch(0.62 0.006 60)" }}>
              {routeSummary.tool.name} {routeSummary.tool.version}
            </span>
          </div>

          <h1 className="text-2xl font-semibold text-foreground mb-2">
            AiZynthFinder route search + B3LYP precursor follow-up
          </h1>
          <p className="max-w-[760px] text-[13px] leading-[1.6]" style={{ color: "oklch(0.62 0.006 60)" }}>
            External CASP evidence generated on <span className="font-mono">{routeSummary.execution_node}</span>:
            five targets searched with the USPTO policy/filter and ZINC stock, followed by
            real PySCF B3LYP / def2-svp calculations for the non-silicon precursor set.
          </p>
        </section>

        <div className="px-8 py-7 max-w-[1280px]">
          <div className="grid grid-cols-2 lg:grid-cols-5 gap-2.5 mb-6">
            <Metric label="Targets" value={String(stats.targets)} accent="oklch(0.76 0.17 192)" />
            <Metric label="Solved" value={`${stats.solvedTargets}/${stats.targets}`} accent="oklch(0.70 0.18 148)" />
            <Metric label="Open" value={String(stats.unsolvedTargets)} accent="oklch(0.72 0.16 78)" />
            <Metric label="B3LYP results" value={String(stats.completedDft)} accent="oklch(0.67 0.18 222)" />
            <Metric label="Blocked" value={String(stats.blockedDft)} accent="oklch(0.65 0.20 25)" />
          </div>

          <div className="grid grid-cols-1 xl:grid-cols-[1.15fr_0.85fr] gap-4">
            <section className="flex flex-col gap-3">
              <SectionHeader
                icon={<GitBranch className="w-3.5 h-3.5" />}
                label="Target outcomes"
                stamp={routeSummary.run_id}
              />
              {routeSummary.targets.map((target) => (
                <TargetCard key={target.target_label} target={target} />
              ))}
            </section>

            <aside className="flex flex-col gap-4">
              <section
                className="rounded p-4"
                style={{ background: "oklch(0.115 0 0)", border: "1px solid oklch(0.20 0 0)" }}
              >
                <SectionHeader
                  icon={<Atom className="w-3.5 h-3.5" />}
                  label="B3LYP precursor DFT"
                  stamp={dftSummary.run_id}
                />
                <div className="flex flex-col gap-2.5 mt-3">
                  {dftSummary.completed_results.map((result) => (
                    <DftFollowUpCard key={result.artifact_id} result={result} />
                  ))}
                </div>
              </section>

              <section
                className="rounded p-4"
                style={{ background: "oklch(0.115 0 0)", border: "1px solid oklch(0.65 0.20 25 / 0.35)" }}
              >
                <div className="flex items-center gap-2 mb-2">
                  <AlertTriangle className="w-3.5 h-3.5" style={{ color: "oklch(0.65 0.20 25)" }} />
                  <span className="text-[10px] font-mono uppercase tracking-[0.2em] font-semibold" style={{ color: "oklch(0.65 0.20 25)" }}>
                    blocked candidate
                  </span>
                </div>
                {dftSummary.blocked_candidates.map((candidate) => (
                  <div key={candidate.smiles} className="font-mono text-[11px] leading-[1.55]">
                    <p className="text-foreground/90 break-all">{candidate.smiles}</p>
                    <p className="mt-2" style={{ color: "oklch(0.62 0.006 60)" }}>
                      {candidate.reason}
                    </p>
                  </div>
                ))}
              </section>

              <section
                className="rounded p-4"
                style={{ background: "oklch(0.115 0 0)", border: "1px solid oklch(0.20 0 0)" }}
              >
                <SectionHeader
                  icon={<Database className="w-3.5 h-3.5" />}
                  label="Static evidence chain"
                  stamp="external CASP → signed DFT"
                />
                <div className="flex flex-col gap-2 mt-3">
                  {routeSummary.hackathon_chain.map((step, index) => (
                    <div key={step} className="grid grid-cols-[28px_1fr] gap-2 items-start">
                      <span
                        className="font-mono text-[10px] tabular text-center rounded py-1"
                        style={{ color: "oklch(0.76 0.17 192)", border: "1px solid oklch(0.76 0.17 192 / 0.35)" }}
                      >
                        {String(index + 1).padStart(2, "0")}
                      </span>
                      <p className="text-[12px] leading-[1.45]" style={{ color: "oklch(0.62 0.006 60)" }}>
                        {step}
                      </p>
                    </div>
                  ))}
                </div>
              </section>

              <section
                className="rounded p-4 font-mono text-[11px] flex flex-col gap-1.5"
                style={{ background: "oklch(0.07 0 0)", border: "1px solid oklch(0.18 0 0)" }}
              >
                <KV k="policy/filter/stock" v={`${routeSummary.tool.policy} / ${routeSummary.tool.filter} / ${routeSummary.tool.stock}`} />
                <KV k="routes archive" v={routeSummary.source_files.routes_json_gz} />
                <KV k="targets" v={routeSummary.source_files.targets_smi} />
                <KV k="DFT method" v={`${dftSummary.method.functional} / ${dftSummary.method.basis_set} / ${dftSummary.method.backend}`} />
              </section>
            </aside>
          </div>
        </div>
      </main>
    </div>
  )
}

function TargetCard({ target }: { target: AiZynthTarget }) {
  const inStock = splitCsv(target.precursors_in_stock)
  const notInStock = splitCsv(target.precursors_not_in_stock)
  const topPrecursors = target.top_route.precursor_smiles

  return (
    <article
      className="rounded overflow-hidden"
      style={{ background: "oklch(0.115 0 0)", border: "1px solid oklch(0.20 0 0)" }}
    >
      <div
        className="px-4 py-2.5 flex items-center justify-between gap-3"
        style={{ borderBottom: "1px solid oklch(0.18 0 0)" }}
      >
        <div className="min-w-0">
          <h2 className="text-[14px] font-semibold text-foreground capitalize truncate">{targetName(target)}</h2>
          <p className="font-mono text-[10px] mt-1 truncate" style={{ color: "oklch(0.46 0.006 60)" }}>
            {target.target_smiles}
          </p>
        </div>
        <StatusTag ok={target.is_solved} label={target.is_solved ? "solved" : "unsolved"} />
      </div>

      <div className="p-4 grid grid-cols-2 lg:grid-cols-4 gap-2.5">
        <MiniStat label="top score" value={fmtScore(target.top_score)} />
        <MiniStat label="routes" value={`${target.number_of_solved_routes}/${target.number_of_routes}`} />
        <MiniStat label="steps" value={String(target.number_of_steps)} />
        <MiniStat label="search" value={fmtSeconds(target.search_time_seconds)} />
      </div>

      <div className="px-4 pb-4 grid grid-cols-1 lg:grid-cols-2 gap-3">
        <div className="rounded p-3" style={{ background: "oklch(0.07 0 0)", border: "1px solid oklch(0.18 0 0)" }}>
          <div className="flex items-center gap-2 mb-2">
            <FlaskConical className="w-3 h-3" style={{ color: "oklch(0.70 0.18 148)" }} />
            <span className="text-[9px] font-mono uppercase tracking-[0.18em]" style={{ color: "oklch(0.70 0.18 148)" }}>
              top-route precursors
            </span>
          </div>
          <div className="flex flex-wrap gap-1.5">
            {topPrecursors.map((smiles) => (
              <span key={smiles} className="tag-inverse-green tag-inverse break-all">
                {smiles}
              </span>
            ))}
          </div>
        </div>
        <div className="rounded p-3" style={{ background: "oklch(0.07 0 0)", border: "1px solid oklch(0.18 0 0)" }}>
          <div className="flex items-center gap-2 mb-2">
            <Clock className="w-3 h-3" style={{ color: "oklch(0.72 0.16 78)" }} />
            <span className="text-[9px] font-mono uppercase tracking-[0.18em]" style={{ color: "oklch(0.72 0.16 78)" }}>
              stock status · {target.number_of_precursors_in_stock}/{target.number_of_precursors}
            </span>
          </div>
          <TagList items={inStock} variant="good" empty="no in-stock precursors" />
          <TagList items={notInStock} variant="warn" empty="all top-route precursors in stock" />
        </div>
      </div>
    </article>
  )
}

function DftFollowUpCard({ result }: { result: B3lypCompletedResult }) {
  return (
    <Link
      href={`/dft/${result.artifact_id}`}
      className="rounded p-3 transition-colors hover:bg-accent/40"
      style={{ background: "oklch(0.07 0 0)", border: "1px solid oklch(0.18 0 0)" }}
    >
      <div className="flex items-start justify-between gap-3 mb-2">
        <div>
          <h3 className="text-[13px] font-semibold text-foreground">{dftLabel(result)}</h3>
          <p className="font-mono text-[10px] mt-1" style={{ color: "oklch(0.46 0.006 60)" }}>
            {result.artifact_id}
          </p>
        </div>
        <span
          className="inline-flex items-center gap-1 text-[9px] font-mono uppercase tracking-[0.18em] px-1.5 py-0.5 rounded"
          style={{
            color: "oklch(0.70 0.18 148)",
            background: "oklch(0.70 0.18 148 / 0.08)",
            border: "1px solid oklch(0.70 0.18 148 / 0.30)",
          }}
        >
          <ShieldCheck className="w-2.5 h-2.5" />
          signed
        </span>
      </div>
      <div className="grid grid-cols-2 gap-x-3 gap-y-1.5 font-mono text-[11px]">
        <KV k="E" v={fmtEnergy(result.energy_hartree)} />
        <KV k="ΔHL" v={`${result.gap_ev.toFixed(3)} eV`} />
        <KV k="μ" v={`${result.dipole_debye.toFixed(3)} D`} />
        <KV k="wall" v={fmtSeconds(result.wall_seconds)} />
      </div>
    </Link>
  )
}

function SectionHeader({ icon, label, stamp }: { icon: ReactNode; label: string; stamp: string }) {
  return (
    <div className="flex items-center justify-between gap-3">
      <div className="flex items-center gap-2">
        <span style={{ color: "oklch(0.76 0.17 192)" }}>{icon}</span>
        <span className="text-[10px] font-mono uppercase tracking-[0.2em] font-semibold" style={{ color: "oklch(0.74 0.006 60)" }}>
          {label}
        </span>
      </div>
      <span className="font-mono text-[9px] uppercase tracking-[0.16em] text-right" style={{ color: "oklch(0.36 0.006 60)" }}>
        {stamp}
      </span>
    </div>
  )
}

function Metric({ label, value, accent }: { label: string; value: string; accent: string }) {
  return (
    <div
      className="rounded px-3.5 py-3 flex flex-col gap-1.5"
      style={{ background: "oklch(0.115 0 0)", border: "1px solid oklch(0.21 0 0)" }}
    >
      <span className="text-[9px] font-mono uppercase tracking-[0.18em]" style={{ color: "oklch(0.56 0.006 60)" }}>
        {label}
      </span>
      <span className="font-numeric-bold leading-[0.95]" style={{ fontSize: "24px", color: accent }}>
        {value}
      </span>
    </div>
  )
}

function MiniStat({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded px-3 py-2" style={{ background: "oklch(0.07 0 0)", border: "1px solid oklch(0.18 0 0)" }}>
      <span className="block text-[9px] font-mono uppercase tracking-[0.18em]" style={{ color: "oklch(0.46 0.006 60)" }}>
        {label}
      </span>
      <span className="block mt-1 text-[15px] font-mono text-foreground/90">{value}</span>
    </div>
  )
}

function StatusTag({ ok, label }: { ok: boolean; label: string }) {
  const color = ok ? "oklch(0.70 0.18 148)" : "oklch(0.72 0.16 78)"
  return (
    <span
      className="text-[9px] font-mono font-bold px-1.5 py-0.5 rounded border uppercase tracking-[0.18em]"
      style={{ color, borderColor: oklchAlpha(color, 0.35), background: oklchAlpha(color, 0.08) }}
    >
      {label}
    </span>
  )
}

function TagList({ items, variant, empty }: { items: string[]; variant: "good" | "warn"; empty: string }) {
  const color = variant === "good" ? "oklch(0.70 0.18 148)" : "oklch(0.72 0.16 78)"
  if (items.length === 0) {
    return (
      <p className="text-[11px] font-mono mb-1.5" style={{ color: "oklch(0.46 0.006 60)" }}>
        {empty}
      </p>
    )
  }
  return (
    <div className="flex flex-wrap gap-1.5 mb-1.5">
      {items.map((item) => (
        <span
          key={item}
          className="font-mono text-[9px] px-1.5 py-0.5 rounded border break-all"
          style={{ color, borderColor: oklchAlpha(color, 0.35), background: oklchAlpha(color, 0.08) }}
        >
          {item}
        </span>
      ))}
    </div>
  )
}

function KV({ k, v }: { k: string; v: string }) {
  return (
    <div className="min-w-0">
      <span className="block text-[9px] font-mono uppercase tracking-[0.18em]" style={{ color: "oklch(0.46 0.006 60)" }}>
        {k}
      </span>
      <span className="block text-[11px] text-foreground/85 break-all">{v}</span>
    </div>
  )
}
