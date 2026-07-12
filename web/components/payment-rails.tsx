"use client";

import { useEffect, useState } from "react";
import { apiBase } from "@/lib/api-gateway";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ExternalLink, Loader2 } from "lucide-react";

interface Method {
  id: string;
  status: string;
  title: string;
  description: string;
  details: Record<string, string | boolean | null>;
}

interface MethodsResponse {
  deployable: boolean;
  honesty: string[];
  methods: Method[];
  agent_default: string;
}

/**
 * Human rails: Stripe Payment Links / Checkout + Revolut.
 * Agents stay on x402.
 */
export function PaymentRails() {
  const [data, setData] = useState<MethodsResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [checkoutBusy, setCheckoutBusy] = useState<string | null>(null);
  const [checkoutMsg, setCheckoutMsg] = useState<string | null>(null);

  useEffect(() => {
    fetch(`${apiBase()}/v1/payment-methods`)
      .then((r) => {
        if (!r.ok) throw new Error(`HTTP ${r.status}`);
        return r.json();
      })
      .then((j) => setData(j as MethodsResponse))
      .catch((e) => setError(e instanceof Error ? e.message : String(e)));
  }, []);

  async function startCheckout(skuId: string, method: "stripe" | "revolut") {
    setCheckoutBusy(`${method}:${skuId}`);
    setCheckoutMsg(null);
    try {
      const res = await fetch(`${apiBase()}/v1/checkout`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ sku_id: skuId, method }),
      });
      const body = (await res.json()) as {
        url?: string;
        error?: string;
        message?: string;
        next?: string;
        dashboard_create?: string;
        payment_signature_ref?: string;
      };
      if (!res.ok) {
        throw new Error(
          body.message ||
            body.error ||
            `checkout HTTP ${res.status}${
              body.dashboard_create
                ? ` — create links at ${body.dashboard_create}`
                : ""
            }`,
        );
      }
      if (body.url) {
        window.open(body.url, "_blank", "noopener,noreferrer");
        setCheckoutMsg(`Opened ${method} for ${skuId}`);
      } else {
        setCheckoutMsg(body.next || body.message || "Checkout response OK (no URL)");
      }
    } catch (e) {
      setCheckoutMsg(e instanceof Error ? e.message : String(e));
    } finally {
      setCheckoutBusy(null);
    }
  }

  const stripe = data?.methods.find((m) => m.id === "stripe");
  const stripeReady = stripe?.status === "configured";

  return (
    <section
      id="payments"
      className="border-t border-border px-4 py-16 md:px-8 lg:px-16"
    >
      <div className="mx-auto max-w-6xl">
        <p className="mb-2 font-mono text-xs tracking-[0.2em] text-primary">
          PAYMENT RAILS
        </p>
        <h2 className="mb-2 text-2xl font-bold text-foreground md:text-3xl">
          Agents: x402 · Humans: Stripe or Revolut
        </h2>
        <p className="mb-8 max-w-2xl text-sm text-muted-foreground">
          Machine settlement stays HTTP 402. Card payments use Stripe Payment
          Links (Dashboard) or Checkout Sessions — configured only on the
          gateway, never in the browser.
        </p>

        {error && (
          <p className="mb-4 text-sm text-muted-foreground">
            Gateway offline ({error}). Expected when API is not running.
          </p>
        )}

        {data && (
          <>
            <div className="mb-6 grid gap-4 md:grid-cols-3">
              {data.methods.map((m) => (
                <Card key={m.id} className="border-border/60 bg-card/40">
                  <CardHeader className="pb-2">
                    <div className="flex items-start justify-between gap-2">
                      <CardTitle className="text-base">{m.title}</CardTitle>
                      <Badge
                        variant={
                          m.status === "unconfigured" ? "secondary" : "default"
                        }
                        className="font-mono text-[10px] uppercase"
                      >
                        {m.status}
                      </Badge>
                    </div>
                  </CardHeader>
                  <CardContent className="space-y-2 text-sm text-muted-foreground">
                    <p>{m.description}</p>
                    {m.id === "stripe" && (
                      <div className="space-y-2 pt-1">
                        {m.details.payment_link_moladt && (
                          <a
                            href={String(m.details.payment_link_moladt)}
                            target="_blank"
                            rel="noreferrer"
                            className="flex items-center gap-1 text-xs text-primary underline-offset-4 hover:underline"
                          >
                            <ExternalLink className="size-3" />
                            MolADT Payment Link
                          </a>
                        )}
                        {m.details.payment_link_dft_cached && (
                          <a
                            href={String(m.details.payment_link_dft_cached)}
                            target="_blank"
                            rel="noreferrer"
                            className="flex items-center gap-1 text-xs text-primary underline-offset-4 hover:underline"
                          >
                            <ExternalLink className="size-3" />
                            Cached DFT Payment Link
                          </a>
                        )}
                        {m.status === "unconfigured" && (
                          <a
                            href="https://dashboard.stripe.com/acct_1PsRXKLWiL2XLIJa/payment-links/create"
                            target="_blank"
                            rel="noreferrer"
                            className="flex items-center gap-1 text-xs text-primary underline-offset-4 hover:underline"
                          >
                            <ExternalLink className="size-3" />
                            Create Payment Links in Stripe
                          </a>
                        )}
                      </div>
                    )}
                    {m.id === "revolut" && m.details.pay_to && (
                      <p className="font-mono text-xs text-foreground">
                        pay_to: {String(m.details.pay_to)}
                      </p>
                    )}
                    {m.id === "revolut" && m.details.payment_link && (
                      <a
                        href={String(m.details.payment_link)}
                        className="text-xs text-primary underline-offset-4 hover:underline"
                        target="_blank"
                        rel="noreferrer"
                      >
                        Revolut payment link
                      </a>
                    )}
                  </CardContent>
                </Card>
              ))}
            </div>

            <div className="mb-6 rounded-xl border border-border bg-card/30 p-5">
              <p className="mb-3 font-mono text-xs tracking-[0.15em] text-primary">
                HUMAN CHECKOUT · POST /v1/checkout
              </p>
              <p className="mb-4 max-w-2xl text-sm text-muted-foreground">
                {stripeReady
                  ? "Stripe is configured. These open your Payment Link or a Checkout Session."
                  : "Stripe is unconfigured until you paste buy.stripe.com links into gateway env. Create them in the Dashboard (link below), then set STRIPE_PAYMENT_LINK_MOLADT / STRIPE_PAYMENT_LINK_DFT_CACHED."}
              </p>
              <div className="flex flex-wrap gap-3">
                <Button
                  size="sm"
                  disabled={!!checkoutBusy}
                  onClick={() => startCheckout("moladt.geometry", "stripe")}
                  className="gap-2"
                >
                  {checkoutBusy === "stripe:moladt.geometry" && (
                    <Loader2 className="size-3.5 animate-spin" />
                  )}
                  Pay MolADT (Stripe)
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  disabled={!!checkoutBusy}
                  onClick={() => startCheckout("dft.cached_result", "stripe")}
                  className="gap-2"
                >
                  {checkoutBusy === "stripe:dft.cached_result" && (
                    <Loader2 className="size-3.5 animate-spin" />
                  )}
                  Pay cached DFT (Stripe)
                </Button>
                <Button
                  size="sm"
                  variant="secondary"
                  disabled={!!checkoutBusy}
                  onClick={() => startCheckout("dft.cached_result", "revolut")}
                  className="gap-2"
                >
                  {checkoutBusy === "revolut:dft.cached_result" && (
                    <Loader2 className="size-3.5 animate-spin" />
                  )}
                  Revolut instructions
                </Button>
              </div>
              {checkoutMsg && (
                <p className="mt-3 font-mono text-xs text-muted-foreground">
                  {checkoutMsg}
                </p>
              )}
            </div>

            <div className="mb-6 rounded-xl border border-dashed border-border/80 bg-muted/10 p-4 font-mono text-[11px] leading-relaxed text-muted-foreground">
              <p className="mb-2 text-foreground">Stripe Payment Links — create now</p>
              <ol className="list-decimal space-y-1 pl-4">
                <li>
                  Open{" "}
                  <a
                    className="text-primary underline-offset-2 hover:underline"
                    href="https://dashboard.stripe.com/acct_1PsRXKLWiL2XLIJa/payment-links/create"
                    target="_blank"
                    rel="noreferrer"
                  >
                    Payment links → Create
                  </a>{" "}
                  (test mode first)
                </li>
                <li>
                  Product 1: <span className="text-foreground">MolADT geometry</span>{" "}
                  — $0.50+ one-time
                </li>
                <li>
                  Product 2:{" "}
                  <span className="text-foreground">Cached DFT result</span> — $0.50–$1
                  one-time
                </li>
                <li>
                  Copy each <span className="text-foreground">buy.stripe.com/…</span>{" "}
                  URL
                </li>
                <li>
                  On gateway:{" "}
                  <span className="text-foreground">
                    STRIPE_PAYMENT_LINK_MOLADT=… STRIPE_PAYMENT_LINK_DFT_CACHED=…
                  </span>
                </li>
              </ol>
            </div>

            <ul className="space-y-1 text-xs text-muted-foreground">
              {data.honesty.map((h) => (
                <li key={h}>· {h}</li>
              ))}
            </ul>
          </>
        )}
      </div>
    </section>
  );
}
