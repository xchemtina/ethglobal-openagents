"use client";

import { useEffect, useState } from "react";
import { apiBase } from "@/lib/api-gateway";
import { checkoutLink } from "@/lib/checkout-link";
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

interface CheckoutInstructions {
  amount?: string;
  pay_to?: string;
  reference?: string;
  payment_link?: string;
  note?: string;
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
  const [checkoutUrl, setCheckoutUrl] = useState<string | null>(null);
  const [instructions, setInstructions] = useState<CheckoutInstructions | null>(null);

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
    setCheckoutUrl(null);
    setInstructions(null);
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
        instructions?: CheckoutInstructions;
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
        const url = checkoutLink(body.url);
        if (!url) throw new Error("The gateway returned an invalid checkout link.");
        setCheckoutUrl(url);
        setCheckoutMsg(`Checkout link ready for ${skuId}. Review the provider's price before paying. Fulfillment requires an operator.`);
      } else {
        if (body.instructions) {
          setInstructions(body.instructions);
          setCheckoutUrl(checkoutLink(body.instructions.payment_link));
        }
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
          HTTP 402 is available as a payment demo; live USDC settlement is unavailable.
          Configured Stripe or Revolut payments require operator fulfillment.
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
                          m.status === "configured" ? "default" : "secondary"
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
                        {checkoutLink(m.details.payment_link_moladt) && (
                          <a
                            href={checkoutLink(m.details.payment_link_moladt)!}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="flex items-center gap-1 text-xs text-primary underline-offset-4 hover:underline"
                          >
                            <ExternalLink className="size-3" />
                            MolADT Payment Link
                          </a>
                        )}
                        {checkoutLink(m.details.payment_link_dft_cached) && (
                          <a
                            href={checkoutLink(m.details.payment_link_dft_cached)!}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="flex items-center gap-1 text-xs text-primary underline-offset-4 hover:underline"
                          >
                            <ExternalLink className="size-3" />
                            Cached DFT Payment Link
                          </a>
                        )}

                      </div>
                    )}
                    {m.id === "revolut" && m.details.pay_to && (
                      <p className="font-mono text-xs text-foreground">
                        pay_to: {String(m.details.pay_to)}
                      </p>
                    )}
                    {m.id === "revolut" && checkoutLink(m.details.payment_link) && (
                      <a
                        href={checkoutLink(m.details.payment_link)!}
                        className="text-xs text-primary underline-offset-4 hover:underline"
                        target="_blank"
                        rel="noopener noreferrer"
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
                  ? "Request a checkout link, then open it to review the provider's price. Payment does not automatically deliver an artifact."
                  : "Stripe checkout is unavailable. An operator must configure a payment method before you can pay."}
              </p>
              <div className="flex flex-wrap gap-3">
                <Button
                  size="sm"
                  disabled={!!checkoutBusy || !stripeReady}
                  onClick={() => startCheckout("moladt.geometry", "stripe")}
                  className="gap-2"
                >
                  {checkoutBusy === "stripe:moladt.geometry" && (
                    <Loader2 className="size-3.5 animate-spin" />
                  )}
                  MolADT checkout link
                </Button>
                <Button
                  size="sm"
                  variant="outline"
                  disabled={!!checkoutBusy || !stripeReady}
                  onClick={() => startCheckout("dft.cached_result", "stripe")}
                  className="gap-2"
                >
                  {checkoutBusy === "stripe:dft.cached_result" && (
                    <Loader2 className="size-3.5 animate-spin" />
                  )}
                  Cached DFT checkout link
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
                <p role="status" className="mt-3 font-mono text-xs text-muted-foreground">
                  {checkoutMsg}
                </p>
              )}
              {checkoutUrl && (
                <Button asChild size="sm" className="mt-3 gap-2">
                  <a href={checkoutUrl} target="_blank" rel="noopener noreferrer">
                    Review checkout on {new URL(checkoutUrl).hostname}
                    <ExternalLink className="size-3.5" />
                  </a>
                </Button>
              )}
              {instructions && (
                <dl className="mt-3 space-y-1 text-sm">
                  {instructions.amount && <div><dt className="inline text-muted-foreground">Amount: </dt><dd className="inline">{instructions.amount}</dd></div>}
                  {instructions.pay_to && <div><dt className="inline text-muted-foreground">Recipient: </dt><dd className="inline">{instructions.pay_to}</dd></div>}
                  {instructions.reference && <div><dt className="inline text-muted-foreground">Payment reference: </dt><dd className="inline font-mono">{instructions.reference}</dd></div>}
                </dl>
              )}
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
