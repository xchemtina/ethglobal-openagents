"use client";

import { useEffect, useState } from "react";
import {
  apiBase,
  displayMode,
  fetchHealth,
  type GatewayHealth,
} from "@/lib/api-gateway";
import { cn } from "@/lib/utils";
import { Copy, ExternalLink, ChevronDown } from "lucide-react";

/**
 * Green when API is reachable.
 * Primary click → catalog (new tab). Chevron expands endpoint list.
 */
export function GatewayBadge({ className }: { className?: string }) {
  const [health, setHealth] = useState<GatewayHealth | null>(null);
  const [error, setError] = useState(false);
  const [open, setOpen] = useState(false);
  const [copied, setCopied] = useState(false);
  const base = apiBase();
  const catalogUrl = `${base}/v1/catalog`;

  useEffect(() => {
    let cancelled = false;
    fetchHealth()
      .then((h) => {
        if (!cancelled) {
          setHealth(h);
          setError(false);
        }
      })
      .catch(() => {
        if (!cancelled) setError(true);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    const onDoc = (e: MouseEvent) => {
      const el = document.getElementById("gateway-badge-root");
      if (el && !el.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("keydown", onKey);
    document.addEventListener("mousedown", onDoc);
    return () => {
      document.removeEventListener("keydown", onKey);
      document.removeEventListener("mousedown", onDoc);
    };
  }, [open]);

  const mode = health?.mode ?? displayMode();
  const live = Boolean(health?.ok && !error);

  const links = [
    { href: `${base}/health`, label: "GET /health" },
    { href: `${base}/v1/catalog`, label: "GET /v1/catalog" },
    { href: `${base}/v1/dft/index`, label: "GET /v1/dft/index" },
    { href: `${base}/v1/payment-methods`, label: "GET /v1/payment-methods" },
    { href: `${base}/.well-known/x402`, label: "GET /.well-known/x402" },
    { href: `${base}/openapi.json`, label: "GET /openapi.json" },
  ];

  async function copyBase() {
    try {
      await navigator.clipboard.writeText(base);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      /* ignore */
    }
  }

  return (
    <div
      id="gateway-badge-root"
      className={cn("relative hidden sm:flex", className)}
    >
      <div
        className={cn(
          "flex items-stretch overflow-hidden rounded-full border font-mono text-[10px] tracking-wide",
          live
            ? "border-emerald-500/40 bg-emerald-500/10 text-emerald-400"
            : "border-border bg-muted/40 text-muted-foreground",
        )}
      >
        {/* Primary: always navigates to live catalog */}
        <a
          href={catalogUrl}
          target="_blank"
          rel="noreferrer"
          title={`Open ${catalogUrl}`}
          className="flex items-center gap-2 px-3 py-1 transition-colors hover:bg-emerald-500/15"
        >
          <span
            className={cn(
              "size-1.5 shrink-0 rounded-full",
              live ? "bg-emerald-400 animate-pulse" : "bg-muted-foreground",
            )}
          />
          <span>{live ? `API ${mode}` : "API offline"}</span>
          <ExternalLink className="size-3 opacity-70" />
        </a>
        <button
          type="button"
          aria-expanded={open}
          aria-label="API endpoints menu"
          onClick={() => setOpen((v) => !v)}
          className={cn(
            "border-l px-2 transition-colors",
            live
              ? "border-emerald-500/30 hover:bg-emerald-500/15"
              : "border-border hover:bg-muted/60",
          )}
        >
          <ChevronDown
            className={cn("size-3.5 transition-transform", open && "rotate-180")}
          />
        </button>
      </div>

      {open && (
        <div
          role="menu"
          className="absolute right-0 top-[calc(100%+6px)] z-[100] w-80 rounded-md border border-border bg-popover p-1 text-popover-foreground shadow-lg"
        >
          <div className="px-2 py-1.5">
            <p className="text-xs font-medium text-foreground">Cashier API</p>
            <p className="mt-1 break-all font-mono text-[10px] text-muted-foreground">
              {base}
            </p>
          </div>
          <div className="my-1 h-px bg-border" />
          <button
            type="button"
            role="menuitem"
            onClick={() => {
              void copyBase();
            }}
            className="flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-left text-xs hover:bg-accent"
          >
            <Copy className="size-3.5" />
            {copied ? "Copied base URL" : "Copy base URL"}
          </button>
          <div className="my-1 h-px bg-border" />
          {links.map((l) => (
            <a
              key={l.href}
              role="menuitem"
              href={l.href}
              target="_blank"
              rel="noreferrer"
              onClick={() => setOpen(false)}
              className="flex items-center gap-2 rounded-sm px-2 py-1.5 font-mono text-[11px] hover:bg-accent"
            >
              <ExternalLink className="size-3.5 shrink-0" />
              {l.label}
            </a>
          ))}
        </div>
      )}
    </div>
  );
}
