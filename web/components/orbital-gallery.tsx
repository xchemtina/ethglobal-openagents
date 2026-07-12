"use client";

import { useState } from "react";
import {
  ORBITAL_GALLERY,
  type GalleryMolecule,
  type OrbitalKind,
} from "@/lib/orbital-gallery-data";
import { OrbitalViewer3D } from "@/components/orbital-viewer-3d";
import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";

const KINDS: { id: OrbitalKind; label: string }[] = [
  { id: "homo", label: "HOMO (2D)" },
  { id: "lumo", label: "LUMO (2D)" },
  { id: "density", label: "ρ(r) (2D)" },
];

/**
 * Real cube-derived orbitals: interactive 3D field + classic 2D PNG slices.
 */
export function OrbitalGallery() {
  const [mol, setMol] = useState<GalleryMolecule>(ORBITAL_GALLERY[0]);
  const [kind, setKind] = useState<OrbitalKind>("homo");
  const [mode, setMode] = useState<"3d" | "2d">("3d");
  const data3d = `/orbitals/3d/${mol.id}.json`;

  return (
    <section
      id="orbitals"
      className="border-t border-border px-4 py-16 md:px-8 lg:px-16"
    >
      <div className="mx-auto max-w-6xl">
        <div className="mb-8 flex flex-col gap-3 md:flex-row md:items-end md:justify-between">
          <div>
            <p className="mb-2 font-mono text-xs tracking-[0.2em] text-primary">
              ORBITAL DENSITIES · REAL EXECUTION
            </p>
            <h2 className="text-2xl font-bold text-foreground md:text-3xl">
              Interactive 3D HOMO / LUMO
            </h2>
            <p className="mt-2 max-w-2xl text-sm text-muted-foreground">
              Soft isosurface fields sampled from Gaussian cubes on Olympus
              (PBE/def2-tzvp). Drag to orbit, scroll to zoom — auto-spin shows
              depth. Cyan / amber = orbital phase. Signed{" "}
              <code className="text-foreground">chem.dft.result</code> + cube
              SHA-256 remain source of truth; this is a browser projection.
            </p>
          </div>
          <div className="flex flex-wrap gap-2">
            <button
              type="button"
              onClick={() => setMode("3d")}
              className={cn(
                "rounded-full border px-3 py-1.5 font-mono text-xs",
                mode === "3d"
                  ? "border-primary bg-primary/15 text-primary"
                  : "border-border text-muted-foreground",
              )}
            >
              3D field
            </button>
            <button
              type="button"
              onClick={() => setMode("2d")}
              className={cn(
                "rounded-full border px-3 py-1.5 font-mono text-xs",
                mode === "2d"
                  ? "border-primary bg-primary/15 text-primary"
                  : "border-border text-muted-foreground",
              )}
            >
              2D slices
            </button>
            <Badge variant="outline" className="font-mono text-[10px]">
              {mol.method}
            </Badge>
          </div>
        </div>

        <div className="mb-4 flex flex-wrap gap-2">
          {ORBITAL_GALLERY.map((m) => (
            <button
              key={m.id}
              type="button"
              onClick={() => setMol(m)}
              className={cn(
                "rounded-full border px-3 py-1.5 text-xs transition-colors",
                mol.id === m.id
                  ? "border-primary bg-primary/15 text-primary"
                  : "border-border text-muted-foreground hover:border-primary/40 hover:text-foreground",
              )}
            >
              {m.label}
            </button>
          ))}
        </div>

        <div className="grid gap-6 lg:grid-cols-[1fr_280px]">
          {mode === "3d" ? (
            <OrbitalViewer3D dataUrl={data3d} />
          ) : (
            <div className="overflow-hidden rounded-xl border border-border bg-black/40">
              {/* eslint-disable-next-line @next/next/no-img-element */}
              <img
                src={mol.images[kind]}
                alt={`${mol.label} ${kind}`}
                className="aspect-square w-full object-contain"
              />
              <div className="flex flex-wrap gap-2 border-t border-border bg-card/40 p-3">
                {KINDS.map((k) => (
                  <button
                    key={k.id}
                    type="button"
                    onClick={() => setKind(k.id)}
                    className={cn(
                      "rounded-md px-3 py-1.5 font-mono text-xs",
                      kind === k.id
                        ? "bg-primary text-primary-foreground"
                        : "bg-muted text-muted-foreground hover:text-foreground",
                    )}
                  >
                    {k.label}
                  </button>
                ))}
              </div>
            </div>
          )}

          <aside className="space-y-4 rounded-xl border border-border bg-card/40 p-5 font-mono text-xs">
            <div>
              <p className="text-muted-foreground">molecule</p>
              <p className="text-lg font-semibold text-foreground">
                {mol.label}
              </p>
              <p className="text-primary">{mol.formula}</p>
            </div>
            <div>
              <p className="text-muted-foreground">artifact</p>
              <p className="break-all text-foreground">{mol.artifactId}</p>
            </div>
            <div className="grid grid-cols-2 gap-3">
              <div>
                <p className="text-muted-foreground">E</p>
                <p className="text-foreground">{mol.energyHa.toFixed(3)} Ha</p>
              </div>
              <div>
                <p className="text-muted-foreground">gap</p>
                <p className="text-foreground">{mol.gapEv.toFixed(3)} eV</p>
              </div>
            </div>
            <p className="leading-relaxed text-muted-foreground">
              Buy signed result:{" "}
              <code className="text-foreground">
                GET /v1/dft/cached?label=
                {mol.id === "propylene-glycol"
                  ? "propylene glycol"
                  : mol.id.replace(/-/g, " ")}
              </code>
            </p>
            <a
              href="#cashier"
              className="inline-flex text-primary underline-offset-4 hover:underline"
            >
              → Open cashier
            </a>
          </aside>
        </div>
      </div>
    </section>
  );
}
