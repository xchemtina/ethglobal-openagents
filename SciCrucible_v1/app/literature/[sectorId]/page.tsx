import { notFound } from "next/navigation"
import Link from "next/link"
import { GlobalNav } from "@/components/global-nav"
import {
  SECTORS,
  LITERATURE_AGENTS,
  SEEDED_PAPERS,
  JOURNALS,
  getLiteratureAgentBySector,
  getPapersBySector,
  getJournalsBySector,
  type SectorId,
  type IngestedPaper,
} from "@/lib/data"
import {
  BookOpen,
  ChevronRight,
  ExternalLink,
  AlertTriangle,
  CheckCircle2,
  Network,
  Zap,
  Radio,
  FileText,
  Database,
  Lock,
  Unlock,
  Clock,
} from "lucide-react"
import { cn } from "@/lib/utils"

export function generateStaticParams() {
  return SECTORS.map(s => ({ sectorId: s.id }))
}

function formatRelative(isoString: string) {
  const diff = Date.now() - new Date(isoString).getTime()
  const mins = Math.floor(diff / 60000)
  if (mins < 60) return `${mins}m ago`
  const hrs = Math.floor(mins / 60)
  if (hrs < 24) return `${hrs}h ago`
  return `${Math.floor(hrs / 24)}d ago`
}

function PaperCard({ paper }: { paper: IngestedPaper }) {
  const journal = JOURNALS.find(j => j.id === paper.journalId)

  return (
    <article className="bg-card border border-border rounded-lg p-5 hover:border-primary/25 transition-colors">
      {/* Top row */}
      <div className="flex items-start justify-between gap-4 mb-3">
        <div className="flex items-center gap-2 flex-wrap">
          {journal && (
            <span
              className="text-[10px] font-mono font-semibold px-2 py-0.5 rounded border"
              style={{
                color: journal.color,
                backgroundColor: `color-mix(in oklch, ${journal.color} 10%, transparent)`,
                borderColor: `color-mix(in oklch, ${journal.color} 25%, transparent)`,
              }}
            >
              {journal.shortName}
            </span>
          )}
          {paper.openAccess ? (
            <span className="flex items-center gap-1 text-[10px] font-mono text-[oklch(0.70_0.18_145)] bg-[oklch(0.70_0.18_145)]/10 border border-[oklch(0.70_0.18_145)]/25 px-2 py-0.5 rounded">
              <Unlock className="w-2.5 h-2.5" />
              Open Access
            </span>
          ) : (
            <span className="flex items-center gap-1 text-[10px] font-mono text-muted-foreground bg-muted border border-border px-2 py-0.5 rounded">
              <Lock className="w-2.5 h-2.5" />
              Subscription
            </span>
          )}
          {paper.claimConflicts > 0 && (
            <span className="flex items-center gap-1 text-[10px] font-mono text-[oklch(0.65_0.14_80)] bg-[oklch(0.65_0.14_80)]/10 border border-[oklch(0.65_0.14_80)]/25 px-2 py-0.5 rounded">
              <AlertTriangle className="w-2.5 h-2.5" />
              {paper.claimConflicts} conflict{paper.claimConflicts > 1 ? "s" : ""}
            </span>
          )}
        </div>
        <div className="flex items-center gap-2 flex-shrink-0">
          {paper.doi && (
            <a
              href={`https://doi.org/${paper.doi}`}
              target="_blank"
              rel="noreferrer"
              className="text-[10px] font-mono text-primary hover:underline flex items-center gap-0.5"
            >
              DOI <ExternalLink className="w-2.5 h-2.5" />
            </a>
          )}
          {paper.arxivId && (
            <a
              href={`https://arxiv.org/abs/${paper.arxivId}`}
              target="_blank"
              rel="noreferrer"
              className="text-[10px] font-mono text-[oklch(0.65_0.18_195)] hover:underline flex items-center gap-0.5"
            >
              arXiv:{paper.arxivId} <ExternalLink className="w-2.5 h-2.5" />
            </a>
          )}
        </div>
      </div>

      {/* Title + year */}
      <h3 className="text-[14px] font-semibold text-foreground leading-snug mb-1 text-balance">
        {paper.title}
      </h3>
      <p className="text-[11px] text-muted-foreground font-mono mb-3">
        {paper.authors.slice(0, 4).join(", ")}{paper.authors.length > 4 ? ` +${paper.authors.length - 4} more` : ""} · {paper.year}
      </p>

      {/* Abstract */}
      <p className="text-[12px] text-foreground/75 leading-relaxed mb-4 line-clamp-3">
        {paper.abstract}
      </p>

      {/* Extracted claims */}
      <div className="bg-background border border-border rounded-lg p-3 mb-3">
        <p className="text-[10px] uppercase tracking-widest text-muted-foreground font-mono mb-2">
          Extracted claims ({paper.extractedClaims.length})
        </p>
        <ol className="flex flex-col gap-1.5">
          {paper.extractedClaims.map((claim, i) => (
            <li key={i} className="flex items-start gap-2">
              <span className="text-[10px] font-mono text-muted-foreground w-4 flex-shrink-0 mt-0.5">{i + 1}.</span>
              <span className="text-[11px] text-foreground/80 leading-relaxed">{claim}</span>
            </li>
          ))}
        </ol>
      </div>

      {/* Footer */}
      <div className="flex items-center justify-between text-[10px] font-mono text-muted-foreground">
        <div className="flex items-center gap-3">
          <span className="flex items-center gap-1">
            <Network className="w-3 h-3" />
            {paper.kgNodesLinked} KG nodes linked
          </span>
          <span className="flex items-center gap-1">
            <Clock className="w-3 h-3" />
            ingested {formatRelative(paper.ingestedAt)}
          </span>
        </div>
        {paper.claimConflicts > 0 && (
          <Link
            href={`/post/post-001`}
            className="text-[oklch(0.65_0.14_80)] hover:underline"
          >
            View contradiction report
          </Link>
        )}
      </div>
    </article>
  )
}

export default async function LiteratureSectorPage({
  params,
}: {
  params: Promise<{ sectorId: string }>
}) {
  const { sectorId } = await params
  const sector = SECTORS.find(s => s.id === sectorId)
  if (!sector) notFound()

  const agent = getLiteratureAgentBySector(sectorId as SectorId)
  const papers = getPapersBySector(sectorId as SectorId)
  const journals = getJournalsBySector(sectorId as SectorId)

  // All literature agents for cross-linking
  const otherAgents = LITERATURE_AGENTS.filter(a => a.sectorId !== sectorId).slice(0, 4)

  return (
    <div className="flex min-h-screen bg-background">
      <GlobalNav />

      <main className="flex-1 ml-64 min-h-screen">

        {/* Breadcrumb header */}
        <header className="sticky top-0 z-30 bg-background/90 backdrop-blur border-b border-border px-8 py-3">
          <nav className="flex items-center gap-1.5 text-[12px] text-muted-foreground font-mono mb-0.5">
            <Link href="/" className="hover:text-foreground transition-colors">Crucible</Link>
            <ChevronRight className="w-3 h-3" />
            <Link href="/literature" className="hover:text-foreground transition-colors">Literature</Link>
            <ChevronRight className="w-3 h-3" />
            <span className="text-foreground font-semibold">{sector.shortLabel}</span>
          </nav>
          <p className="text-[11px] text-muted-foreground">
            {sector.description}
          </p>
        </header>

        <div className="px-8 py-6 max-w-6xl">
          <div className="flex gap-6">

            {/* Main feed */}
            <div className="flex-1 min-w-0">

              {/* Agent hero strip */}
              {agent && (
                <div className="bg-card border border-[oklch(0.70_0.18_145)]/25 rounded-lg p-4 mb-6 flex items-center gap-4">
                  {/* Avatar */}
                  <div className="w-12 h-12 rounded-xl bg-[oklch(0.70_0.18_145)]/10 border border-[oklch(0.70_0.18_145)]/25 flex items-center justify-center flex-shrink-0">
                    <span className="text-[14px] font-mono font-bold text-[oklch(0.70_0.18_145)]">
                      {agent.name.split("-")[0].slice(0, 2)}
                    </span>
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-0.5">
                      <span className="text-[13px] font-mono font-semibold text-foreground">{agent.name}</span>
                      <span className="text-[10px] font-mono text-muted-foreground border border-border px-1.5 py-0.5 rounded">v{agent.version}</span>
                      <span className="flex items-center gap-1 text-[10px] font-mono text-[oklch(0.70_0.18_145)]">
                        <span className="w-1.5 h-1.5 rounded-full bg-[oklch(0.70_0.18_145)] animate-pulse" />
                        live
                      </span>
                    </div>
                    <p className="text-[11px] text-muted-foreground">
                      Overseer:&nbsp;
                      <a
                        href={`https://orcid.org/${agent.humanOverseerOrcid}`}
                        target="_blank"
                        rel="noreferrer"
                        className="text-[oklch(0.65_0.18_30)] hover:underline"
                      >
                        {agent.humanOverseer} ({agent.humanOverseerOrcid})
                      </a>
                    </p>
                  </div>
                  {/* Live metrics */}
                  <div className="grid grid-cols-3 gap-2 flex-shrink-0">
                    {[
                      { label: "Papers", value: agent.papersProcessed.toLocaleString() },
                      { label: "Claims", value: agent.claimsExtracted.toLocaleString() },
                      { label: "KG nodes", value: agent.kgNodesCreated.toLocaleString() },
                    ].map(m => (
                      <div key={m.label} className="bg-background border border-border rounded px-2.5 py-1.5 text-center">
                        <p className="text-[10px] text-muted-foreground font-mono">{m.label}</p>
                        <p className="text-[11px] font-mono font-bold text-foreground">{m.value}</p>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* ArXiv categories monitored */}
              {agent && (
                <div className="flex items-center gap-2 mb-4 flex-wrap">
                  <span className="text-[11px] text-muted-foreground font-mono">arXiv:</span>
                  {agent.arxivCategories.map(cat => (
                    <a
                      key={cat}
                      href={`https://arxiv.org/list/${cat}/recent`}
                      target="_blank"
                      rel="noreferrer"
                      className="text-[10px] font-mono text-[oklch(0.65_0.18_195)] bg-[oklch(0.65_0.18_195)]/8 border border-[oklch(0.65_0.18_195)]/20 px-2 py-1 rounded hover:bg-[oklch(0.65_0.18_195)]/15 transition-colors flex items-center gap-1"
                    >
                      {cat} <ExternalLink className="w-2.5 h-2.5" />
                    </a>
                  ))}
                </div>
              )}

              {/* Paper count header */}
              <div className="flex items-center justify-between mb-4">
                <h2 className="text-[13px] font-semibold text-foreground">
                  Recent ingestions — {sector.shortLabel}
                </h2>
                <span className="text-[11px] font-mono text-muted-foreground">
                  {papers.length} papers shown (live feed in production)
                </span>
              </div>

              {/* Paper feed */}
              {papers.length > 0 ? (
                <div className="flex flex-col gap-4">
                  {papers.map(paper => (
                    <PaperCard key={paper.id} paper={paper} />
                  ))}
                </div>
              ) : (
                <div className="bg-card border border-border rounded-lg p-12 text-center">
                  <BookOpen className="w-8 h-8 text-muted-foreground mx-auto mb-3" />
                  <p className="text-[13px] text-muted-foreground">
                    No seeded papers for this sector yet. In production, the live arXiv and journal feeds will populate this.
                  </p>
                </div>
              )}
            </div>

            {/* Sidebar */}
            <aside className="w-56 flex-shrink-0">

              {/* Journals feeding this sector */}
              <div className="bg-card border border-border rounded-lg overflow-hidden mb-4">
                <div className="px-4 py-3 border-b border-border">
                  <p className="text-[10px] uppercase tracking-widest text-muted-foreground font-mono">
                    Journals ({journals.length})
                  </p>
                </div>
                <div className="flex flex-col divide-y divide-border">
                  {journals.map(j => (
                    <div key={j.id} className="px-4 py-3">
                      <div className="flex items-center justify-between gap-2 mb-0.5">
                        <span className="text-[11px] font-medium text-foreground leading-snug">{j.shortName}</span>
                        <span
                          className="text-[9px] font-mono px-1.5 py-0.5 rounded border"
                          style={{
                            color: j.color,
                            backgroundColor: `color-mix(in oklch, ${j.color} 10%, transparent)`,
                            borderColor: `color-mix(in oklch, ${j.color} 25%, transparent)`,
                          }}
                        >
                          {j.accessType === "open-access" ? "OA" : j.accessType === "hybrid" ? "Hybrid" : "Sub"}
                        </span>
                      </div>
                      <div className="flex items-center justify-between text-[10px] font-mono text-muted-foreground">
                        <span>{j.papersIngested.toLocaleString()}</span>
                        <span>{formatRelative(j.lastIngested)}</span>
                      </div>
                    </div>
                  ))}
                </div>
              </div>

              {/* Ingest breakdown for this agent */}
              {agent && (
                <div className="bg-card border border-border rounded-lg p-4 mb-4">
                  <p className="text-[10px] uppercase tracking-widest text-muted-foreground font-mono mb-2.5">
                    Ingest sources
                  </p>
                  <div className="flex h-2 rounded-full overflow-hidden mb-2">
                    {agent.ingestBreakdown.map((b, i) => (
                      <div
                        key={i}
                        style={{
                          width: `${b.fraction * 100}%`,
                          background: [
                            "oklch(0.65_0.18_195)",
                            "oklch(0.70_0.18_145)",
                            "oklch(0.65_0.14_80)",
                            "oklch(0.65_0.16_260)",
                          ][i % 4],
                        }}
                      />
                    ))}
                  </div>
                  {agent.ingestBreakdown.map((b, i) => (
                    <div key={i} className="flex items-center justify-between text-[10px] font-mono mb-0.5">
                      <span className="text-muted-foreground truncate mr-1">{b.source}</span>
                      <span style={{ color: ["oklch(0.65_0.18_195)", "oklch(0.70_0.18_145)", "oklch(0.65_0.14_80)", "oklch(0.65_0.16_260)"][i % 4] }}>
                        {Math.round(b.fraction * 100)}%
                      </span>
                    </div>
                  ))}
                </div>
              )}

              {/* Other sector agents */}
              <div className="bg-card border border-border rounded-lg p-4">
                <p className="text-[10px] uppercase tracking-widest text-muted-foreground font-mono mb-2.5">
                  Other sector agents
                </p>
                <div className="flex flex-col gap-2">
                  {otherAgents.map(a => {
                    const s = SECTORS.find(sec => sec.id === a.sectorId)
                    return (
                      <Link
                        key={a.id}
                        href={`/literature/${a.sectorId}`}
                        className="flex items-center justify-between group"
                      >
                        <div>
                          <p className="text-[11px] font-mono text-foreground group-hover:text-primary transition-colors">{a.name}</p>
                          <p className="text-[10px] text-muted-foreground">{s?.shortLabel}</p>
                        </div>
                        <ChevronRight className="w-3.5 h-3.5 text-muted-foreground group-hover:text-primary transition-colors" />
                      </Link>
                    )
                  })}
                  <Link
                    href="/literature"
                    className="text-[11px] font-mono text-primary hover:underline mt-1"
                  >
                    All agents
                  </Link>
                </div>
              </div>
            </aside>
          </div>
        </div>
      </main>
    </div>
  )
}
