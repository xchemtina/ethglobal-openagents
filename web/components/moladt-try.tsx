"use client";

import { useState } from "react";
import { Loader2, Sparkles } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { apiBase, fetchMoladtStub } from "@/lib/api-gateway";

/**
 * Real x402 stub cashier call: POST /v1/moladt with PAYMENT-SIGNATURE: stub.
 * Uses curated-library path (no_worker) so local demos work without RDKit.
 */
export function MoladtTry() {
  const [smiles, setSmiles] = useState("O");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<{
    artifactId?: string;
    price?: string;
    mode?: string;
    notes?: string[];
  } | null>(null);

  async function onSubmit(e: React.FormEvent) {
    e.preventDefault();
    setLoading(true);
    setError(null);
    setResult(null);
    try {
      const body = (await fetchMoladtStub(smiles.trim())) as {
        result_artifact_id?: string;
        settlement?: { price_display?: string; mode?: string };
        result?: { notes?: string[]; artifact?: { id?: string } };
      };
      const id =
        body.result_artifact_id ||
        (typeof body.result?.artifact?.id === "string"
          ? body.result.artifact.id
          : undefined);
      setResult({
        artifactId: id,
        price: body.settlement?.price_display,
        mode: body.settlement?.mode,
        notes: body.result?.notes,
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="rounded-xl border border-border bg-card/50 p-5">
      <div className="mb-3 flex items-center justify-between gap-2">
        <div>
          <p className="font-mono text-xs tracking-[0.15em] text-primary">
            LIVE STUB CASHIER
          </p>
          <h3 className="text-lg font-semibold text-foreground">
            Try MolADT via x402
          </h3>
        </div>
        <code className="hidden font-mono text-[10px] text-muted-foreground sm:block">
          {apiBase()}/v1/moladt
        </code>
      </div>
      <p className="mb-4 text-sm text-muted-foreground">
        Sends a real HTTP request with{" "}
        <code className="text-foreground">PAYMENT-SIGNATURE: stub</code> (no
        funds). Curated SMILES only when{" "}
        <code className="text-foreground">no_worker</code> is set — try{" "}
        <code className="text-foreground">O</code>,{" "}
        <code className="text-foreground">CO</code>,{" "}
        <code className="text-foreground">c1ccccc1</code>.
      </p>
      <form onSubmit={onSubmit} className="flex flex-col gap-3 sm:flex-row">
        <Input
          value={smiles}
          onChange={(e) => setSmiles(e.target.value)}
          placeholder="SMILES"
          className="font-mono"
          aria-label="SMILES"
        />
        <Button type="submit" disabled={loading || !smiles.trim()} className="gap-2">
          {loading ? (
            <Loader2 className="size-4 animate-spin" />
          ) : (
            <Sparkles className="size-4" />
          )}
          Pay stub $0.01
        </Button>
      </form>
      {error && (
        <p className="mt-3 text-sm text-red-400" role="alert">
          {error}
        </p>
      )}
      {result && (
        <div className="mt-4 space-y-1 rounded-lg border border-emerald-500/30 bg-emerald-500/5 p-3 font-mono text-xs">
          <p className="text-emerald-400">ok · mode {result.mode}</p>
          <p className="text-foreground">
            artifact: {result.artifactId ?? "(see response)"}
          </p>
          {result.price && (
            <p className="text-muted-foreground">price: {result.price}</p>
          )}
        </div>
      )}
    </div>
  );
}
