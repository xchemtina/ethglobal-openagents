"use client";

import { apiBase, displayMode, displayPayTo } from "@/lib/api-gateway";

const base = () => apiBase();

export function AgentApiPanel() {
  const b = base();
  const snippets = [
    {
      title: "Health",
      code: `curl -s ${b}/health | jq .`,
    },
    {
      title: "Catalog (free)",
      code: `curl -s ${b}/v1/catalog | jq '.skus[] | {sku_id, price_display, status}'`,
    },
    {
      title: "x402 discovery",
      code: `curl -s ${b}/.well-known/x402 | jq .`,
    },
    {
      title: "Cached DFT index (free)",
      code: `curl -s ${b}/v1/dft/index | jq '.items[].label'`,
    },
    {
      title: "MolADT paid stub",
      code: `curl -s -X POST ${b}/v1/moladt \\\n  -H 'content-type: application/json' \\\n  -H 'PAYMENT-SIGNATURE: stub' \\\n  -d '{"smiles":"O","no_worker":true}' | jq '.result_artifact_id'`,
    },
    {
      title: "Cached DFT paid stub",
      code: `curl -s '${b}/v1/dft/cached?label=water' \\\n  -H 'PAYMENT-SIGNATURE: stub' | jq '.result_artifact_id,.summary'`,
    },
    {
      title: "Payment methods (Stripe / Revolut / x402)",
      code: `curl -s ${b}/v1/payment-methods | jq '.methods[] | {id,status}'`,
    },
    {
      title: "OpenAPI",
      code: `curl -s ${b}/openapi.json | jq '.paths | keys'`,
    },
  ];

  return (
    <div className="mt-10 space-y-4">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div>
          <p className="font-mono text-xs tracking-[0.15em] text-primary">
            FOR AGENTS
          </p>
          <h3 className="text-xl font-semibold text-foreground">
            Cashier API (live contract)
          </h3>
        </div>
        <div className="font-mono text-[11px] text-muted-foreground">
          mode <span className="text-primary">{displayMode()}</span> · pay_to{" "}
          <span className="text-foreground">{displayPayTo().slice(0, 10)}…</span>
        </div>
      </div>
      <div className="grid gap-3 md:grid-cols-2">
        {snippets.map((s) => (
          <div
            key={s.title}
            className="rounded-lg border border-border bg-muted/20 p-3"
          >
            <p className="mb-2 text-xs font-medium text-foreground">{s.title}</p>
            <pre className="overflow-x-auto whitespace-pre-wrap font-mono text-[11px] leading-relaxed text-muted-foreground">
              {s.code}
            </pre>
          </div>
        ))}
      </div>
    </div>
  );
}
