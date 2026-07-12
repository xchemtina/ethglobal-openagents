"use client";

import { useState } from "react";
import { Loader2, Terminal } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  apiBase,
  fetchMoladtStub,
  displayMode,
} from "@/lib/api-gateway";
import { MoladtTry } from "@/components/moladt-try";
import { cn } from "@/lib/utils";

/**
 * Machine-first cashier surface: SMILES / labels / HTTP 402 stub.
 * Humans can poke it; agents use the same paths with curl or HTTP clients.
 */
export function AgentCashier() {
  const [dftLabel, setDftLabel] = useState("water");
  const [dftLoading, setDftLoading] = useState(false);
  const [dftError, setDftError] = useState<string | null>(null);
  const [dftResult, setDftResult] = useState<{
    id?: string;
    energy?: number;
    gap?: number;
  } | null>(null);

  async function buyCachedDft(e: React.FormEvent) {
    e.preventDefault();
    setDftLoading(true);
    setDftError(null);
    setDftResult(null);
    try {
      const url = `${apiBase()}/v1/dft/cached?label=${encodeURIComponent(dftLabel.trim())}`;
      const res = await fetch(url, {
        headers: { "PAYMENT-SIGNATURE": "stub" },
      });
      const body = await res.json().catch(() => ({}));
      if (!res.ok) {
        throw new Error(
          (body as { message?: string; error?: string }).message ||
            (body as { error?: string }).error ||
            `HTTP ${res.status}`,
        );
      }
      const b = body as {
        result_artifact_id?: string;
        summary?: { energy_hartree?: number; gap_ev?: number };
      };
      setDftResult({
        id: b.result_artifact_id,
        energy: b.summary?.energy_hartree,
        gap: b.summary?.gap_ev,
      });
    } catch (err) {
      setDftError(err instanceof Error ? err.message : String(err));
    } finally {
      setDftLoading(false);
    }
  }

  return (
    <div className="space-y-6">
      <div className="rounded-xl border border-primary/30 bg-primary/5 p-5">
        <div className="mb-2 flex items-center gap-2 font-mono text-xs tracking-[0.15em] text-primary">
          <Terminal className="size-3.5" />
          PRIMARY SURFACE · AGENTS
        </div>
        <h3 className="text-xl font-semibold text-foreground md:text-2xl">
          HTTP 402 cashier — SMILES in, signed artifact out
        </h3>
        <p className="mt-2 max-w-3xl text-sm leading-relaxed text-muted-foreground">
          Machines are the customers. There is no draw-to-structure dependency.
          POST JSON (or GET cached DFT by label), settle with{" "}
          <code className="text-foreground">PAYMENT-SIGNATURE</code> (stub now,
          USDC later), receive a content-addressed result. Mode:{" "}
          <span className="text-primary">{displayMode()}</span> · base{" "}
          <code className="text-foreground">{apiBase()}</code>
        </p>
      </div>

      <div className="grid gap-6 lg:grid-cols-2">
        <MoladtTry />

        <div className="rounded-xl border border-border bg-card/50 p-5">
          <p className="font-mono text-xs tracking-[0.15em] text-primary">
            CACHED DFT · $0.05 STUB
          </p>
          <h3 className="mt-1 text-lg font-semibold text-foreground">
            Buy a signed chem.dft.result
          </h3>
          <p className="mt-2 text-sm text-muted-foreground">
            Labels from the Olympus gallery:{" "}
            <code className="text-foreground">water</code>,{" "}
            <code className="text-foreground">methanol</code>,{" "}
            <code className="text-foreground">benzene</code>,{" "}
            <code className="text-foreground">propylene glycol</code>,{" "}
            <code className="text-foreground">caprylic acid</code>,{" "}
            <code className="text-foreground">capric acid</code>.
          </p>
          <form
            onSubmit={buyCachedDft}
            className="mt-4 flex flex-col gap-3 sm:flex-row"
          >
            <Input
              value={dftLabel}
              onChange={(e) => setDftLabel(e.target.value)}
              className="font-mono"
              aria-label="DFT cache label"
            />
            <Button
              type="submit"
              disabled={dftLoading || !dftLabel.trim()}
              className="gap-2"
            >
              {dftLoading && <Loader2 className="size-4 animate-spin" />}
              Pay stub
            </Button>
          </form>
          {dftError && (
            <p className="mt-3 text-sm text-red-400" role="alert">
              {dftError}
            </p>
          )}
          {dftResult && (
            <div className="mt-4 space-y-1 rounded-lg border border-emerald-500/30 bg-emerald-500/5 p-3 font-mono text-xs">
              <p className="text-emerald-400">ok</p>
              <p>artifact: {dftResult.id}</p>
              {dftResult.energy != null && (
                <p>E = {dftResult.energy.toFixed(6)} Ha</p>
              )}
              {dftResult.gap != null && (
                <p>gap = {dftResult.gap.toFixed(3)} eV</p>
              )}
            </div>
          )}
        </div>
      </div>

      <pre
        className={cn(
          "overflow-x-auto rounded-lg border border-border bg-muted/20 p-4",
          "font-mono text-[11px] leading-relaxed text-muted-foreground",
        )}
      >{`# Agent loop (stub — no funds)
curl -s ${apiBase()}/v1/catalog | jq '.skus[] | select(.status=="live")'
curl -s -X POST ${apiBase()}/v1/moladt \\
  -H 'content-type: application/json' \\
  -H 'PAYMENT-SIGNATURE: stub' \\
  -d '{"smiles":"c1ccccc1","no_worker":true}' | jq '.result_artifact_id'
curl -s '${apiBase()}/v1/dft/cached?label=benzene' \\
  -H 'PAYMENT-SIGNATURE: stub' | jq '.result_artifact_id,.summary'`}</pre>
    </div>
  );
}
