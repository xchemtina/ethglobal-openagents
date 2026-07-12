"use client";

import { useEffect, useState } from "react";
import {
  defaultTickerMolecules,
  formatEnergy,
  type EvidenceMolecule,
} from "@/lib/real-evidence";
import { fetchDftIndex } from "@/lib/api-gateway";

function toTickerItem(m: EvidenceMolecule) {
  return {
    name: m.name,
    formula: m.formula,
    artifactId: m.artifactId,
    energy: formatEnergy(m.energyHa),
  };
}

export function MoleculeTicker() {
  const [molecules, setMolecules] = useState(() =>
    defaultTickerMolecules().map(toTickerItem),
  );

  useEffect(() => {
    fetchDftIndex()
      .then((idx) => {
        if (!idx.items?.length) return;
        const fromGateway = idx.items.map((item) => ({
          name: item.label,
          formula: item.smiles ?? undefined,
          artifactId: item.artifact_id,
          energy:
            item.energy_hartree != null
              ? `${item.energy_hartree.toFixed(2)} Ha`
              : undefined,
        }));
        // Prefer gateway cache labels; keep gallery static if index empty-ish
        if (fromGateway.length >= 3) {
          setMolecules(fromGateway);
        }
      })
      .catch(() => {
        /* keep static real evidence */
      });
  }, []);

  const doubled = [...molecules, ...molecules];

  return (
    <div className="w-full overflow-hidden border-y border-border bg-muted/30 py-3">
      <div className="ticker-scroll flex gap-8 whitespace-nowrap">
        {doubled.map((mol, i) => (
          <div key={`${mol.artifactId}-${i}`} className="flex items-center gap-4 text-sm">
            <span className="font-medium text-foreground">{mol.name}</span>
            {mol.formula && (
              <span className="text-muted-foreground">{mol.formula}</span>
            )}
            <code className="rounded bg-primary/5 px-2 py-0.5 font-mono text-xs text-primary/70">
              {mol.artifactId}
            </code>
            {mol.energy && (
              <span className="font-mono text-xs text-accent">{mol.energy}</span>
            )}
            <span className="text-border">•</span>
          </div>
        ))}
      </div>
    </div>
  );
}
