"use client";

import { useEffect, useState } from "react";
import {
  apiBase,
  fetchCatalog,
  type PublicCatalog,
} from "@/lib/api-gateway";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

export function ServiceCatalog() {
  const [catalog, setCatalog] = useState<PublicCatalog | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const c = await fetchCatalog();
        if (!cancelled) {
          setCatalog(c);
          setError(null);
        }
      } catch (e) {
        if (!cancelled) {
          setError(e instanceof Error ? e.message : String(e));
          setCatalog(null);
        }
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <section
      id="catalog"
      className="border-t border-border px-4 py-16 md:px-8 lg:px-16"
    >
      <div className="mx-auto max-w-6xl">
        <div className="mb-8 flex flex-col gap-2 md:flex-row md:items-end md:justify-between">
          <div>
            <p className="mb-2 font-mono text-xs tracking-[0.2em] text-primary">
              X402 CATALOG
            </p>
            <h2 className="text-2xl font-bold text-foreground md:text-3xl">
              Live service catalog
            </h2>
            <p className="mt-2 max-w-xl text-sm text-muted-foreground">
              Pulled from{" "}
              <code className="text-primary">{apiBase()}/v1/catalog</code>. Agents
              and this site share the same SKUs.
            </p>
          </div>
          {catalog && (
            <div className="font-mono text-xs text-muted-foreground">
              mode <span className="text-primary">{catalog.mode}</span> ·{" "}
              {catalog.network}
            </div>
          )}
        </div>

        {loading && (
          <p className="text-sm text-muted-foreground">Loading catalog…</p>
        )}
        {error && (
          <Card className="border-yellow-500/30 bg-yellow-500/5">
            <CardContent className="py-4 text-sm text-muted-foreground">
              Gateway offline or unreachable ({error}). Start{" "}
              <code className="text-foreground">services/api-gateway</code> with{" "}
              <code className="text-foreground">X402_MODE=stub</code> to populate
              this table. Static demo content below the fold still works.
            </CardContent>
          </Card>
        )}
        {catalog && (
          <div className="grid gap-4 sm:grid-cols-2">
            {catalog.skus.map((sku) => (
              <Card
                key={sku.sku_id}
                className="border-border/60 bg-card/40 backdrop-blur"
              >
                <CardHeader className="pb-2">
                  <div className="flex items-start justify-between gap-2">
                    <CardTitle className="text-base font-semibold">
                      {sku.title}
                    </CardTitle>
                    <Badge
                      variant={
                        sku.status === "live" ? "default" : "secondary"
                      }
                      className="shrink-0 font-mono text-[10px] uppercase"
                    >
                      {sku.status}
                    </Badge>
                  </div>
                  <p className="font-mono text-xs text-primary">{sku.sku_id}</p>
                </CardHeader>
                <CardContent className="space-y-2 text-sm text-muted-foreground">
                  <p>{sku.description}</p>
                  <div className="flex flex-wrap gap-3 font-mono text-xs text-foreground">
                    <span>{sku.price_display}</span>
                    <span>
                      {sku.http_method} {sku.path}
                    </span>
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        )}
      </div>
    </section>
  );
}
