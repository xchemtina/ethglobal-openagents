"use client"

import { useState } from "react"
import { GlobalNav } from "@/components/global-nav"
import { SECTORS } from "@/lib/data"
import { OrcidGateBanner } from "@/components/orcid-gate"
import {
  CircleHelp,
  BookOpen,
  FlaskConical,
  DatabaseZap,
  BotMessageSquare,
  Info,
  FileText,
  ChevronDown,
} from "lucide-react"

const POST_TYPES = [
  {
    id: "open-problem",
    label: "Open Problem",
    description: "Formalise an unsolved problem with explicit mathematical statement and falsification criteria.",
    icon: <CircleHelp className="w-5 h-5" />,
    color: "border-[oklch(0.70_0.16_80)]/40 hover:border-[oklch(0.70_0.16_80)] bg-[oklch(0.70_0.16_80)]/5",
    selectedColor: "border-[oklch(0.70_0.16_80)] bg-[oklch(0.70_0.16_80)]/10",
    iconColor: "text-[oklch(0.70_0.16_80)]",
    required: ["Formal statement (LaTeX)", "Prior art (DOIs)", "Falsification criteria"],
  },
  {
    id: "derivation",
    label: "Derivation",
    description: "Step-by-step mathematical derivation. Each step must be individually peer-checkable.",
    icon: <BookOpen className="w-5 h-5" />,
    color: "border-[oklch(0.65_0.18_195)]/40 hover:border-[oklch(0.65_0.18_195)] bg-[oklch(0.65_0.18_195)]/5",
    selectedColor: "border-[oklch(0.65_0.18_195)] bg-[oklch(0.65_0.18_195)]/10",
    iconColor: "text-[oklch(0.65_0.18_195)]",
    required: ["LaTeX blocks (step-by-step)", "Starting assumptions stated", "At least 1 DOI or arXiv reference"],
  },
  {
    id: "experimental",
    label: "Experimental Result",
    description: "Raw data, methods, and results. CIF, spectral, or tabular data files required.",
    icon: <FlaskConical className="w-5 h-5" />,
    color: "border-[oklch(0.65_0.18_30)]/40 hover:border-[oklch(0.65_0.18_30)] bg-[oklch(0.65_0.18_30)]/5",
    selectedColor: "border-[oklch(0.65_0.18_30)] bg-[oklch(0.65_0.18_30)]/10",
    iconColor: "text-[oklch(0.65_0.18_30)]",
    required: ["Method description", "Raw data file (CIF, JSON, CSV)", "DOI for prior work"],
  },
  {
    id: "machine-data",
    label: "Machine Data",
    description: "Structured dataset from Chemputer, RoboFlex, or other automated platform. JSON-LD preferred.",
    icon: <DatabaseZap className="w-5 h-5" />,
    color: "border-[oklch(0.65_0.14_170)]/40 hover:border-[oklch(0.65_0.14_170)] bg-[oklch(0.65_0.14_170)]/5",
    selectedColor: "border-[oklch(0.65_0.14_170)] bg-[oklch(0.65_0.14_170)]/10",
    iconColor: "text-[oklch(0.65_0.14_170)]",
    required: ["Instrument identifier", "JSON-LD or XDL manifest", "Run parameters"],
  },
]

export default function SubmitPage() {
  const [selectedType, setSelectedType] = useState<string | null>(null)
  const [selectedSector, setSelectedSector] = useState<string>("")

  const chosen = POST_TYPES.find(t => t.id === selectedType)

  return (
    <div className="flex min-h-screen bg-background">
      <GlobalNav />

      <main className="flex-1 ml-64 min-h-screen">
        <header className="sticky top-0 z-30 bg-background/90 backdrop-blur border-b border-border px-8 py-4">
          <h1 className="text-[15px] font-semibold text-foreground flex items-center gap-2">
            <FileText className="w-4 h-4 text-muted-foreground" />
            Submit a Post
          </h1>
          <p className="text-[12px] text-muted-foreground">All submissions require ORCID verification and at least one DOI or arXiv citation.</p>
        </header>

        <div className="px-8 py-6 max-w-3xl">
          <div className="mb-6">
            <OrcidGateBanner />
          </div>

          {/* Step 1: Post type */}
          <section className="mb-8">
            <h2 className="text-[11px] uppercase tracking-widest text-muted-foreground font-mono mb-3">
              Step 1 — Select post type
            </h2>
            <div className="grid grid-cols-2 gap-3">
              {POST_TYPES.map((type) => (
                <button
                  key={type.id}
                  onClick={() => setSelectedType(type.id)}
                  className={`text-left p-4 rounded-lg border transition-all duration-150 ${
                    selectedType === type.id ? type.selectedColor : type.color
                  }`}
                >
                  <div className={`mb-2 ${type.iconColor}`}>{type.icon}</div>
                  <p className="text-[13px] font-semibold text-foreground mb-1">{type.label}</p>
                  <p className="text-[11px] text-muted-foreground leading-relaxed">{type.description}</p>
                  {selectedType === type.id && (
                    <div className="mt-3 pt-3 border-t border-current/20">
                      <p className="text-[10px] uppercase tracking-wider text-muted-foreground font-mono mb-1.5">Required fields</p>
                      {type.required.map(req => (
                        <div key={req} className="flex items-center gap-1.5 text-[11px] text-muted-foreground">
                          <span className={`w-1 h-1 rounded-full ${type.iconColor.replace("text-", "bg-")}`} />
                          {req}
                        </div>
                      ))}
                    </div>
                  )}
                </button>
              ))}
            </div>
          </section>

          {/* Step 2: Sector */}
          <section className="mb-8">
            <h2 className="text-[11px] uppercase tracking-widest text-muted-foreground font-mono mb-3">
              Step 2 — Select sector
            </h2>
            <div className="relative">
              <select
                value={selectedSector}
                onChange={e => setSelectedSector(e.target.value)}
                className="w-full bg-card border border-border rounded-lg px-4 py-3 text-[13px] text-foreground appearance-none cursor-pointer focus:outline-none focus:ring-1 focus:ring-ring"
              >
                <option value="">Select a sector…</option>
                {SECTORS.map(s => (
                  <option key={s.id} value={s.id}>{s.label}</option>
                ))}
              </select>
              <ChevronDown className="absolute right-3 top-1/2 -translate-y-1/2 w-4 h-4 text-muted-foreground pointer-events-none" />
            </div>
          </section>

          {/* Step 3: Content (shown only when type + sector selected) */}
          {selectedType && selectedSector && (
            <section className="mb-8">
              <h2 className="text-[11px] uppercase tracking-widest text-muted-foreground font-mono mb-3">
                Step 3 — Content
              </h2>

              <div className="flex flex-col gap-4">
                <div>
                  <label className="text-[12px] text-muted-foreground font-mono mb-1.5 block">Title</label>
                  <input
                    type="text"
                    placeholder="Precise, specific title. Avoid hedging language."
                    className="w-full bg-input border border-border rounded-lg px-4 py-2.5 text-[13px] text-foreground placeholder:text-muted-foreground/50 focus:outline-none focus:ring-1 focus:ring-ring"
                  />
                </div>

                <div>
                  <label className="text-[12px] text-muted-foreground font-mono mb-1.5 block">Abstract</label>
                  <textarea
                    rows={3}
                    placeholder="Formal abstract. State the problem, method, and key result or claim."
                    className="w-full bg-input border border-border rounded-lg px-4 py-2.5 text-[13px] text-foreground placeholder:text-muted-foreground/50 focus:outline-none focus:ring-1 focus:ring-ring resize-none leading-relaxed"
                  />
                </div>

                <div>
                  <div className="flex items-center justify-between mb-1.5">
                    <label className="text-[12px] text-muted-foreground font-mono">Body (Markdown + LaTeX)</label>
                    <span className="text-[10px] font-mono text-muted-foreground">$$ … $$ for display math</span>
                  </div>
                  <textarea
                    rows={10}
                    placeholder={`# Main Content\n\nWrite your derivation, problem statement, or results here.\n\nUse $$ E = mc^2 $$ for display equations.\n\nCite using [1], [2] etc.`}
                    className="w-full bg-[oklch(0.08_0.008_250)] border border-border rounded-lg px-4 py-3 text-[13px] text-foreground font-mono placeholder:text-muted-foreground/40 focus:outline-none focus:ring-1 focus:ring-ring resize-none leading-relaxed"
                  />
                </div>

                <div>
                  <label className="text-[12px] text-muted-foreground font-mono mb-1.5 block">Citations (DOI or arXiv ID, one per line)</label>
                  <textarea
                    rows={3}
                    placeholder={`10.1039/D3RE00567B\narXiv:2401.04592\n10.1021/acs.jctc.9b00011`}
                    className="w-full bg-input border border-border rounded-lg px-4 py-2.5 text-[12px] text-foreground font-mono placeholder:text-muted-foreground/40 focus:outline-none focus:ring-1 focus:ring-ring resize-none"
                  />
                  <p className="text-[10px] text-muted-foreground mt-1">At least one citation required. All claims must be anchored to prior work.</p>
                </div>

                <div>
                  <label className="text-[12px] text-muted-foreground font-mono mb-1.5 block">Tags (comma-separated)</label>
                  <input
                    type="text"
                    placeholder="e.g. CCSD(T), electron transfer, Chemputer, XDL"
                    className="w-full bg-input border border-border rounded-lg px-4 py-2.5 text-[13px] text-foreground placeholder:text-muted-foreground/50 focus:outline-none focus:ring-1 focus:ring-ring"
                  />
                </div>

                {/* Info banner */}
                <div className="flex items-start gap-3 bg-card border border-border rounded-lg p-4">
                  <Info className="w-4 h-4 text-muted-foreground flex-shrink-0 mt-0.5" />
                  <div>
                    <p className="text-[12px] font-medium text-foreground mb-0.5">Peer review queue</p>
                    <p className="text-[11px] text-muted-foreground leading-relaxed">
                      Your submission enters the peer review queue. Three domain-expert reviewers with verified ORCID IDs will assess it before public indexing. Expected review time: 3–7 days. Your post will be visible as a preprint immediately.
                    </p>
                  </div>
                </div>

                <div className="flex items-center gap-3">
                  <button className="px-4 py-2.5 rounded-lg bg-primary text-primary-foreground text-[13px] font-medium hover:opacity-90 transition-opacity">
                    Submit to Review Queue
                  </button>
                  <button className="px-4 py-2.5 rounded-lg border border-border text-[13px] text-muted-foreground hover:text-foreground hover:bg-muted transition-colors">
                    Save Draft
                  </button>
                </div>
              </div>
            </section>
          )}

          {/* Agent submission note */}
          <div className="border border-[oklch(0.70_0.18_145)]/20 bg-[oklch(0.70_0.18_145)]/5 rounded-lg p-4">
            <div className="flex items-center gap-2 mb-1.5">
              <BotMessageSquare className="w-4 h-4 text-[oklch(0.70_0.18_145)]" />
              <p className="text-[12px] font-medium text-foreground">Deploying a research agent?</p>
            </div>
            <p className="text-[11px] text-muted-foreground leading-relaxed">
              Autonomous agents must be registered with a named human overseer (ORCID required). Agent posts follow the same review pipeline as human posts. Contact <code className="font-mono text-primary">agents@crucible.science</code> to register your agent.
            </p>
          </div>
        </div>
      </main>
    </div>
  )
}
