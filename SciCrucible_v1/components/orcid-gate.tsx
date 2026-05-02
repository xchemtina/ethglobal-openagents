"use client"

import { BadgeCheck, Lock, ExternalLink, FlaskConical, ArrowRight } from "lucide-react"

export function OrcidGateBanner() {
  return (
    <div className="border border-[oklch(0.65_0.18_30)]/30 bg-[oklch(0.65_0.18_30)]/5 rounded-lg p-4 flex items-start gap-3">
      <Lock className="w-4 h-4 text-[oklch(0.65_0.18_30)] flex-shrink-0 mt-0.5" />
      <div className="flex-1">
        <p className="text-[13px] font-medium text-foreground mb-0.5">ORCID verification required</p>
        <p className="text-[12px] text-muted-foreground leading-relaxed">
          Upvoting, commenting, posting, and peer review are gated behind ORCID verification. This ensures all discourse is attributable to real researchers.
        </p>
      </div>
      <a
        href="/auth/orcid"
        className="flex items-center gap-1.5 px-3 py-1.5 rounded bg-[oklch(0.65_0.18_30)] text-background text-[12px] font-medium flex-shrink-0 hover:opacity-90 transition-opacity"
      >
        Connect <ArrowRight className="w-3 h-3" />
      </a>
    </div>
  )
}

export function OrcidGatePage() {
  return (
    <div className="flex-1 flex items-center justify-center px-6 py-24">
      <div className="max-w-md w-full">
        <div className="flex items-center justify-center w-16 h-16 rounded-2xl bg-card border border-border mb-6 mx-auto">
          <FlaskConical className="w-7 h-7 text-primary" />
        </div>

        <h1 className="text-xl font-semibold text-foreground text-center mb-2">
          Verify your researcher identity
        </h1>
        <p className="text-[13px] text-muted-foreground text-center leading-relaxed mb-8">
          Crucible requires ORCID verification to post, comment, vote, and conduct peer review. All contributions are permanently attributed to your researcher identity.
        </p>

        <div className="bg-card border border-border rounded-lg p-5 mb-5">
          <h2 className="text-[13px] font-medium text-foreground mb-3">What ORCID verification unlocks</h2>
          <div className="flex flex-col gap-2.5">
            {[
              "Submit open problems, derivations, and experimental results",
              "Peer review submitted posts (reputation-gated)",
              "Vote on posts with weighted researcher reputation",
              "Comment on and contest claims",
              "Spawn and oversee autonomous research agents",
              "Access machine-readable knowledge graph API",
            ].map((item) => (
              <div key={item} className="flex items-start gap-2.5">
                <BadgeCheck className="w-4 h-4 text-primary flex-shrink-0 mt-0.5" />
                <span className="text-[12px] text-muted-foreground">{item}</span>
              </div>
            ))}
          </div>
        </div>

        <button className="w-full flex items-center justify-center gap-2.5 px-4 py-3 rounded-lg bg-[oklch(0.65_0.18_30)] hover:opacity-90 text-background font-medium text-[14px] transition-opacity mb-3">
          <div className="w-5 h-5 rounded-full bg-background/20 flex items-center justify-center">
            <span className="text-[9px] font-bold">iD</span>
          </div>
          Connect with ORCID
          <ExternalLink className="w-3.5 h-3.5 opacity-70" />
        </button>

        <p className="text-[11px] text-muted-foreground text-center">
          No ORCID yet?{" "}
          <a href="https://orcid.org/register" target="_blank" rel="noreferrer" className="text-primary underline underline-offset-2">
            Register free at orcid.org
          </a>
        </p>
      </div>
    </div>
  )
}
