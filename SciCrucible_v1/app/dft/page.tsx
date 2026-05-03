import Link from "next/link"
import Image from "next/image"
import { GlobalNav } from "@/components/global-nav"
import { loadAllDftRecords, recordDisplayName, type DftRecord } from "@/lib/dft-artifacts"
import { Atom, ChevronRight, ShieldCheck, FlaskConical } from "lucide-react"

export const dynamic = "force-static"

function fmtEnergy(h: number): string {
  return `${h.toFixed(6)} Eh`
}

function fmtGapEv(record: DftRecord): string | null {
  return record.result.orbitals
    ? `${record.result.orbitals.gap_ev.toFixed(3)} eV`
    : null
}

function fmtDipole(record: DftRecord): string | null {
  return record.result.dipole
    ? `${record.result.dipole.magnitude_debye.toFixed(3)} D`
    : null
}

export default function DftIndexPage() {
  const records = loadAllDftRecords()

  return (
    <div className="flex min-h-screen bg-background">
      <GlobalNav />

      <main className="flex-1 ml-64 min-h-screen">
        {/* Breadcrumb header */}
        <header className="sticky top-0 z-30 bg-background/90 backdrop-blur border-b border-border px-8 py-3">
          <nav className="flex items-center gap-1.5 text-[12px] text-muted-foreground font-mono">
            <Link href="/" className="hover:text-foreground transition-colors">
              Crucible
            </Link>
            <ChevronRight className="w-3 h-3" />
            <span className="text-foreground">DFT Results</span>
          </nav>
        </header>

        <section
          className="px-8 pt-7 pb-8"
          style={{ borderBottom: "1px solid oklch(0.19 0 0)" }}
        >
          <div className="flex items-center gap-3 mb-3">
            <Atom className="w-4 h-4" style={{ color: "oklch(0.76 0.17 192)" }} />
            <span
              className="text-[10px] font-mono uppercase tracking-[0.2em] font-semibold"
              style={{ color: "oklch(0.76 0.17 192)" }}
            >
              chem.dft.result
            </span>
            <span className="text-[10px] font-mono" style={{ color: "oklch(0.30 0.006 60)" }}>
              ·
            </span>
            <span className="text-[10px] font-mono uppercase tracking-[0.18em]" style={{ color: "oklch(0.62 0.006 60)" }}>
              {records.length} signed artifacts
            </span>
          </div>

          <h1 className="text-2xl font-semibold text-foreground mb-2">
            Density-Functional Theory results
          </h1>
          <p className="max-w-[640px] text-[13px] leading-[1.6]" style={{ color: "oklch(0.62 0.006 60)" }}>
            Signed PySCF calculations from the lab worker: the original PBE /
            def2-tzvp molecule set with orbital-density cubes, plus B3LYP /
            def2-svp precursor follow-up from the AiZynthFinder route-search run.
            Each result is signed by <span className="font-mono">chimiaclaw-cli</span>{" "}
            and parented to a <span className="font-mono">chem.molecule.adt</span>{" "}
            canonical geometry.
          </p>
        </section>

        <div className="px-8 py-7 max-w-[1280px]">
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
            {records.map((r) => (
              <DftCard key={r.resultArtifactId} record={r} />
            ))}
          </div>
        </div>
      </main>
    </div>
  )
}

function DftCard({ record }: { record: DftRecord }) {
  const name = recordDisplayName(record)
  const formula = record.request.molecule.molecular_formula
  const homoLumo = fmtGapEv(record)
  const dipole = fmtDipole(record)
  const homoCube = record.cubes.find((c) => c.label === "HOMO")

  return (
    <Link
      href={`/dft/${record.resultArtifactId}`}
      className="group rounded-md overflow-hidden flex flex-col transition-colors duration-100"
      style={{
        background: "oklch(0.115 0 0)",
        border: "1px solid oklch(0.20 0 0)",
      }}
    >
      <div
        className="px-4 py-2.5 flex items-center justify-between"
        style={{ borderBottom: "1px solid oklch(0.18 0 0)" }}
      >
        <div className="flex items-center gap-2 min-w-0">
          <FlaskConical className="w-3.5 h-3.5 flex-shrink-0" style={{ color: "oklch(0.76 0.17 192)" }} />
          <span className="text-[13px] font-semibold text-foreground capitalize truncate">{name}</span>
        </div>
        <span className="font-mono text-[10px] tabular" style={{ color: "oklch(0.50 0.006 60)" }}>
          {formula}
        </span>
      </div>

      {/* HOMO preview */}
      <div
        className="relative aspect-square overflow-hidden flex items-center justify-center"
        style={{ background: "oklch(0.07 0 0)" }}
      >
        {homoCube ? (
          <Image
            src={homoCube.pngPublicPath}
            alt={`${name} HOMO`}
            width={400}
            height={400}
            className="object-contain w-full h-full"
            unoptimized
          />
        ) : (
          <span className="text-[10px] font-mono text-muted-foreground">no cube</span>
        )}
        <span
          className="absolute top-2 left-2 font-mono text-[8px] tabular px-1.5 py-0.5 rounded"
          style={{
            color: "oklch(0.76 0.17 192)",
            background: "oklch(0.07 0 0)",
            border: "1px solid oklch(0.76 0.17 192 / 0.30)",
          }}
        >
          HOMO
        </span>
      </div>

      {/* Stats grid */}
      <div className="px-4 py-3 flex flex-col gap-1.5 font-mono text-[11px] tabular">
        <Row k="E" v={fmtEnergy(record.result.energy_hartree)} />
        {homoLumo && <Row k="ΔHL" v={homoLumo} />}
        {dipole && <Row k="μ" v={dipole} />}
        <Row
          k="method"
          v={`${record.result.functional} / ${record.result.basis_set}`}
        />
      </div>

      {/* Footer */}
      <div
        className="px-4 py-2 flex items-center justify-between"
        style={{ borderTop: "1px solid oklch(0.18 0 0)" }}
      >
        <span
          className="flex items-center gap-1.5 text-[9px] font-mono uppercase tracking-[0.18em]"
          style={{ color: "oklch(0.70 0.18 148)" }}
        >
          <ShieldCheck className="w-3 h-3" />
          signed
        </span>
        <span className="font-mono text-[9px] tabular" style={{ color: "oklch(0.46 0.006 60)" }}>
          {record.resultArtifactId}
        </span>
      </div>
    </Link>
  )
}

function Row({ k, v }: { k: string; v: string }) {
  return (
    <div className="flex items-center justify-between gap-3">
      <span style={{ color: "oklch(0.50 0.006 60)" }}>{k}</span>
      <span className="text-foreground/90 truncate">{v}</span>
    </div>
  )
}
