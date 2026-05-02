import { notFound } from "next/navigation"
import Link from "next/link"
import Image from "next/image"
import { GlobalNav } from "@/components/global-nav"
import {
  loadAllDftRecords,
  getDftRecord,
  recordDisplayName,
  type DftRecord,
} from "@/lib/dft-artifacts"
import {
  ChevronRight,
  ShieldCheck,
  Atom,
  Cpu,
  FileSignature,
  Hash,
} from "lucide-react"

export const dynamic = "force-static"

export function generateStaticParams() {
  return loadAllDftRecords().map((r) => ({ id: r.resultArtifactId }))
}

function fmt6(n: number): string {
  return n.toFixed(6)
}

function fmt3(n: number): string {
  return n.toFixed(3)
}

function fmtSeconds(s: number): string {
  if (s < 60) return `${s.toFixed(1)} s`
  const m = Math.floor(s / 60)
  const rem = s - m * 60
  return `${m}m ${rem.toFixed(1)}s`
}

function shortHash(h: string): string {
  return `${h.slice(0, 8)}…${h.slice(-6)}`
}

export default async function DftDetailPage({
  params,
}: {
  params: Promise<{ id: string }>
}) {
  const { id } = await params
  const record = getDftRecord(id)
  if (!record) notFound()

  const name = recordDisplayName(record)
  const formula = record.request.molecule.molecular_formula
  const result = record.result
  const request = record.request
  const molecule = record.molecule
  const sig = record.rawSignedArtifact

  return (
    <div className="flex min-h-screen bg-background">
      <GlobalNav />

      <main className="flex-1 ml-64 min-h-screen">
        {/* Breadcrumb */}
        <header className="sticky top-0 z-30 bg-background/90 backdrop-blur border-b border-border px-8 py-3">
          <nav className="flex items-center gap-1.5 text-[12px] text-muted-foreground font-mono">
            <Link href="/" className="hover:text-foreground transition-colors">
              Crucible
            </Link>
            <ChevronRight className="w-3 h-3" />
            <Link href="/dft" className="hover:text-foreground transition-colors">
              DFT Results
            </Link>
            <ChevronRight className="w-3 h-3" />
            <span className="text-foreground capitalize">{name}</span>
          </nav>
        </header>

        <div className="px-8 py-6 max-w-5xl">
          {/* Title block */}
          <div className="flex items-start justify-between gap-4 mb-5">
            <div>
              <div className="flex items-center gap-2 mb-2">
                <Atom className="w-3.5 h-3.5" style={{ color: "oklch(0.76 0.17 192)" }} />
                <span
                  className="text-[10px] font-mono uppercase tracking-[0.2em]"
                  style={{ color: "oklch(0.76 0.17 192)" }}
                >
                  chem.dft.result
                </span>
                <span className="text-[10px] font-mono" style={{ color: "oklch(0.30 0.006 60)" }}>
                  ·
                </span>
                <span
                  className="font-mono text-[10px] tabular"
                  style={{ color: "oklch(0.50 0.006 60)" }}
                >
                  {record.resultArtifactId}
                </span>
                <SignedTag />
              </div>
              <h1 className="text-2xl font-semibold text-foreground capitalize">{name}</h1>
              <p className="text-[12px] text-muted-foreground font-mono mt-1">
                {formula} ·{" "}
                {request.molecule.canonical_smiles ? (
                  <span>SMILES <span className="text-foreground/70">{request.molecule.canonical_smiles}</span></span>
                ) : (
                  "no SMILES"
                )}
              </p>
            </div>
          </div>

          {/* Top metrics strip */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-2.5 mb-5">
            <Metric
              label="Total energy"
              value={`${fmt6(result.energy_hartree)} Eh`}
              accent="oklch(0.76 0.17 192)"
            />
            {result.orbitals && (
              <Metric
                label="HOMO–LUMO gap"
                value={`${fmt3(result.orbitals.gap_ev)} eV`}
                accent="oklch(0.67 0.18 222)"
              />
            )}
            {result.dipole && (
              <Metric
                label="|μ| dipole"
                value={`${fmt3(result.dipole.magnitude_debye)} D`}
                accent="oklch(0.72 0.16 78)"
              />
            )}
            <Metric
              label="SCF cycles"
              value={`${result.convergence.n_cycles} ${result.convergence.converged ? "✓" : "✗"}`}
              accent="oklch(0.70 0.18 148)"
            />
          </div>

          {/* Cubes */}
          <SectionHeader label="Orbital density cubes" />
          <p className="text-[11px] text-muted-foreground font-mono mb-3">
            Signed-max-amplitude projection along z. Each PNG is rendered from a
            Gaussian cube file (50³ grid) embedded inline in the result payload.
          </p>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-3 mb-7">
            {record.cubes.map((c) => (
              <CubeCard
                key={c.cubeSha256Prefix}
                label={c.label}
                src={c.pngPublicPath}
                shaPrefix={c.cubeSha256Prefix}
              />
            ))}
          </div>

          {/* Method */}
          <SectionHeader label="Method" />
          <div
            className="rounded p-4 mb-6 grid grid-cols-2 md:grid-cols-3 gap-3 font-mono text-[12px]"
            style={{
              background: "oklch(0.115 0 0)",
              border: "1px solid oklch(0.20 0 0)",
            }}
          >
            <KV k="functional" v={result.functional} />
            <KV k="basis_set" v={result.basis_set} />
            <KV k="backend" v={result.backend} />
            <KV k="charge" v={String(result.total_charge)} />
            <KV k="multiplicity" v={String(result.multiplicity)} />
            {request.method.grid_level !== null && (
              <KV k="grid_level" v={String(request.method.grid_level)} />
            )}
          </div>

          {/* Orbitals */}
          {result.orbitals && (
            <>
              <SectionHeader label="Frontier orbitals" />
              <div
                className="rounded p-4 mb-6 grid grid-cols-2 md:grid-cols-4 gap-3 font-mono text-[12px]"
                style={{
                  background: "oklch(0.115 0 0)",
                  border: "1px solid oklch(0.20 0 0)",
                }}
              >
                <KV k="HOMO" v={`${fmt6(result.orbitals.homo_hartree)} Eh`} />
                <KV k="LUMO" v={`${fmt6(result.orbitals.lumo_hartree)} Eh`} />
                <KV k="ΔE (Eh)" v={fmt6(result.orbitals.gap_hartree)} />
                <KV k="ΔE (eV)" v={fmt3(result.orbitals.gap_ev)} />
              </div>
            </>
          )}

          {/* Dipole */}
          {result.dipole && (
            <>
              <SectionHeader label="Dipole moment" />
              <div
                className="rounded p-4 mb-6 grid grid-cols-2 md:grid-cols-4 gap-3 font-mono text-[12px]"
                style={{
                  background: "oklch(0.115 0 0)",
                  border: "1px solid oklch(0.20 0 0)",
                }}
              >
                <KV k="μx" v={`${fmt3(result.dipole.x_debye)} D`} />
                <KV k="μy" v={`${fmt3(result.dipole.y_debye)} D`} />
                <KV k="μz" v={`${fmt3(result.dipole.z_debye)} D`} />
                <KV k="|μ|" v={`${fmt3(result.dipole.magnitude_debye)} D`} />
              </div>
            </>
          )}

          {/* Geometry */}
          <SectionHeader label="Geometry (canonical MolADT projection)" />
          <p className="text-[11px] text-muted-foreground font-mono mb-2">
            Atomic coordinates from the parent <span className="text-foreground/80">chem.molecule.adt</span>{" "}
            artifact <span className="text-foreground/80">{record.moleculeArtifactId}</span>.
          </p>
          <div
            className="rounded p-4 mb-6 overflow-x-auto"
            style={{
              background: "oklch(0.07 0 0)",
              border: "1px solid oklch(0.18 0 0)",
            }}
          >
            <pre className="font-mono text-[11px] leading-[1.5] text-foreground/85">
              {Object.values(molecule.atoms)
                .sort((a, b) => a.atom_id - b.atom_id)
                .map(
                  (atom) =>
                    `${atom.attributes.symbol.padEnd(2)}  ${fmt6(atom.coordinate.x_angstrom).padStart(11)}  ${fmt6(atom.coordinate.y_angstrom).padStart(11)}  ${fmt6(atom.coordinate.z_angstrom).padStart(11)}`,
                )
                .join("\n")}
            </pre>
          </div>

          {/* Provenance */}
          <SectionHeader label="Provenance" />
          <div
            className="rounded p-4 mb-6 flex flex-col gap-1.5 font-mono text-[11px]"
            style={{
              background: "oklch(0.115 0 0)",
              border: "1px solid oklch(0.20 0 0)",
            }}
          >
            <KV k="source" v={`${result.provenance.source_kind} (${result.provenance.source_ref})`} />
            {result.provenance.host && <KV k="host" v={result.provenance.host} />}
            {result.provenance.pyscf_version && (
              <KV k="pyscf" v={result.provenance.pyscf_version} />
            )}
            <KV k="wall" v={fmtSeconds(result.timings.wall_seconds)} />
            <KV
              k="threshold"
              v={
                result.convergence.scf_threshold !== null
                  ? `${result.convergence.scf_threshold.toExponential(1)}`
                  : "n/a"
              }
            />
          </div>

          {/* Signed-artifact lineage */}
          <SectionHeader label="Signed-artifact lineage" />
          <div
            className="rounded p-4 mb-10 flex flex-col gap-2 font-mono text-[11px]"
            style={{
              background: "oklch(0.07 0 0)",
              border: "1px solid oklch(0.18 0 0)",
            }}
          >
            <LineageRow
              icon={<Hash className="w-3 h-3" />}
              label="result.id"
              v={record.resultArtifactId}
            />
            <LineageRow
              icon={<Hash className="w-3 h-3" />}
              label="request.id"
              v={record.requestArtifactId}
              parentTag
            />
            <LineageRow
              icon={<Hash className="w-3 h-3" />}
              label="molecule.id"
              v={record.moleculeArtifactId}
              parentTag
            />
            <LineageRow
              icon={<FileSignature className="w-3 h-3" />}
              label="signing key"
              v={shortHash(sig.signing_public_key)}
            />
            <LineageRow
              icon={<FileSignature className="w-3 h-3" />}
              label="signature"
              v={shortHash(sig.signature)}
            />
            <LineageRow
              icon={<Cpu className="w-3 h-3" />}
              label="agent"
              v={sig.agent}
            />
            <LineageRow
              icon={<Cpu className="w-3 h-3" />}
              label="skill"
              v={sig.skill}
            />
            {sig.schema_tags.length > 0 && (
              <LineageRow
                icon={<Cpu className="w-3 h-3" />}
                label="schema_tags"
                v={sig.schema_tags.join(", ")}
              />
            )}
          </div>
        </div>
      </main>
    </div>
  )
}

/* ---------------- helpers ---------------- */

function SignedTag() {
  return (
    <span
      className="ml-1 inline-flex items-center gap-1 text-[9px] font-mono uppercase tracking-[0.18em] px-1.5 py-0.5 rounded"
      style={{
        color: "oklch(0.70 0.18 148)",
        background: "oklch(0.70 0.18 148 / 0.08)",
        border: "1px solid oklch(0.70 0.18 148 / 0.30)",
      }}
    >
      <ShieldCheck className="w-2.5 h-2.5" />
      signed
    </span>
  )
}

function SectionHeader({ label }: { label: string }) {
  return (
    <div className="flex items-center gap-2 mb-2">
      <span
        className="text-[10px] font-mono uppercase tracking-[0.2em] font-semibold"
        style={{ color: "oklch(0.74 0.006 60)" }}
      >
        {label}
      </span>
      <span className="flex-1 h-px" style={{ background: "oklch(0.18 0 0)" }} />
    </div>
  )
}

function Metric({
  label,
  value,
  accent,
}: {
  label: string
  value: string
  accent: string
}) {
  return (
    <div
      className="rounded px-3.5 py-3 flex flex-col gap-1.5"
      style={{
        background: "oklch(0.115 0 0)",
        border: "1px solid oklch(0.21 0 0)",
      }}
    >
      <span
        className="text-[9px] font-mono uppercase tracking-[0.18em]"
        style={{ color: "oklch(0.56 0.006 60)" }}
      >
        {label}
      </span>
      <span
        className="font-numeric-bold leading-[0.95]"
        style={{ fontSize: "22px", color: accent }}
      >
        {value}
      </span>
    </div>
  )
}

function CubeCard({
  label,
  src,
  shaPrefix,
}: {
  label: "HOMO" | "LUMO" | "TOTAL_DENSITY"
  src: string
  shaPrefix: string
}) {
  const accent =
    label === "HOMO"
      ? "oklch(0.76 0.17 192)"
      : label === "LUMO"
        ? "oklch(0.70 0.18 28)"
        : "oklch(0.70 0.18 148)"
  return (
    <div
      className="rounded overflow-hidden flex flex-col"
      style={{
        background: "oklch(0.115 0 0)",
        border: "1px solid oklch(0.20 0 0)",
      }}
    >
      <div
        className="px-3 py-2 flex items-center justify-between"
        style={{ borderBottom: "1px solid oklch(0.18 0 0)" }}
      >
        <span
          className="text-[10px] font-mono uppercase tracking-[0.2em] font-semibold"
          style={{ color: accent }}
        >
          {label.replace("_", " ")}
        </span>
        <span className="font-mono text-[9px] tabular" style={{ color: "oklch(0.46 0.006 60)" }}>
          {shaPrefix.slice(0, 12)}
        </span>
      </div>
      <div
        className="relative aspect-square flex items-center justify-center"
        style={{ background: "oklch(0.07 0 0)" }}
      >
        <Image
          src={src}
          alt={label}
          width={520}
          height={520}
          className="object-contain w-full h-full"
          unoptimized
        />
      </div>
    </div>
  )
}

function KV({ k, v }: { k: string; v: string }) {
  return (
    <div className="flex flex-col gap-0.5">
      <span
        className="text-[9px] font-mono uppercase tracking-[0.18em]"
        style={{ color: "oklch(0.50 0.006 60)" }}
      >
        {k}
      </span>
      <span className="text-[12px] text-foreground/90 break-all">{v}</span>
    </div>
  )
}

function LineageRow({
  icon,
  label,
  v,
  parentTag,
}: {
  icon: React.ReactNode
  label: string
  v: string
  parentTag?: boolean
}) {
  return (
    <div className="flex items-center gap-2 min-w-0">
      <span style={{ color: "oklch(0.50 0.006 60)" }}>{icon}</span>
      <span
        className="text-[10px] uppercase tracking-[0.18em]"
        style={{ color: "oklch(0.50 0.006 60)", minWidth: 110 }}
      >
        {label}
      </span>
      <span className="text-foreground/85 truncate" title={v}>
        {v}
      </span>
      {parentTag && (
        <span
          className="ml-auto text-[8px] font-mono uppercase tracking-[0.18em] px-1.5 py-0.5 rounded"
          style={{
            color: "oklch(0.62 0.006 60)",
            background: "oklch(0.13 0 0)",
            border: "1px solid oklch(0.22 0 0)",
          }}
        >
          parent
        </span>
      )}
    </div>
  )
}

// Avoid unused warning for DftRecord re-export.
export type _DftRecord = DftRecord
