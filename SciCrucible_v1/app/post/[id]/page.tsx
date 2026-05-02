import { notFound } from "next/navigation"
import Link from "next/link"
import { POSTS, getPostById, getSectorById, POST_TYPE_LABELS, REVIEW_STATUS_LABELS } from "@/lib/data"
import { GlobalNav } from "@/components/global-nav"
import { OrcidGateBanner } from "@/components/orcid-gate"
import { cn } from "@/lib/utils"
import {
  ArrowUp,
  Eye,
  MessageSquare,
  BadgeCheck,
  BotMessageSquare,
  ChevronRight,
  ExternalLink,
  BookOpen,
  Activity,
  Network,
  AlertTriangle,
  Clock,
  FileText,
  User,
  Copy,
  Share2,
} from "lucide-react"

function formatDate(iso: string) {
  return new Date(iso).toLocaleDateString("en-GB", {
    day: "numeric", month: "long", year: "numeric", hour: "2-digit", minute: "2-digit",
  })
}

const REVIEW_STATUS_CONFIG = {
  "preprint": { label: "Preprint", color: "text-muted-foreground", bg: "bg-muted", icon: <FileText className="w-3 h-3" /> },
  "under-review": { label: "Under Review", color: "text-[oklch(0.70_0.16_80)]", bg: "bg-[oklch(0.70_0.16_80)]/10", icon: <Clock className="w-3 h-3" /> },
  "peer-reviewed": { label: "Peer Reviewed", color: "text-[oklch(0.70_0.18_145)]", bg: "bg-[oklch(0.70_0.18_145)]/10", icon: <BadgeCheck className="w-3 h-3" /> },
  "contested": { label: "Contested", color: "text-[oklch(0.65_0.18_30)]", bg: "bg-[oklch(0.65_0.18_30)]/10", icon: <AlertTriangle className="w-3 h-3" /> },
}

export function generateStaticParams() {
  return POSTS.map((p) => ({ id: p.id }))
}

export default async function PostPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params
  const post = getPostById(id)
  if (!post) notFound()

  const sector = getSectorById(post.sectorId)
  const statusConfig = REVIEW_STATUS_CONFIG[post.reviewStatus]

  return (
    <div className="flex min-h-screen bg-background">
      <GlobalNav />

      <main className="flex-1 ml-64 min-h-screen">
        {/* Breadcrumb header */}
        <header className="sticky top-0 z-30 bg-background/90 backdrop-blur border-b border-border px-8 py-3">
          <nav className="flex items-center gap-1.5 text-[12px] text-muted-foreground font-mono">
            <Link href="/" className="hover:text-foreground transition-colors">Crucible</Link>
            <ChevronRight className="w-3 h-3" />
            {sector && (
              <>
                <Link href={`/sector/${sector.id}`} className="hover:text-foreground transition-colors">{sector.shortLabel}</Link>
                <ChevronRight className="w-3 h-3" />
              </>
            )}
            <span className="text-foreground truncate max-w-[400px]">{post.title.slice(0, 60)}…</span>
          </nav>
        </header>

        <div className="px-8 py-6 max-w-5xl">
          <div className="flex gap-6">
            {/* Main content */}
            <article className="flex-1 min-w-0">

              {/* Meta badges */}
              <div className="flex items-center gap-2 flex-wrap mb-4">
                <span className="text-[11px] font-mono px-2 py-0.5 rounded border border-border text-muted-foreground bg-muted">
                  {POST_TYPE_LABELS[post.type]}
                </span>
                <span className={cn(
                  "inline-flex items-center gap-1 text-[11px] font-mono px-2 py-0.5 rounded",
                  statusConfig.color, statusConfig.bg
                )}>
                  {statusConfig.icon}
                  {statusConfig.label}
                  {post.reviewCount > 0 && ` · ${post.reviewCount} reviewers`}
                </span>
                {post.authors[0].isAgent && (
                  <span className="inline-flex items-center gap-1 text-[11px] font-mono text-[oklch(0.70_0.18_145)] bg-[oklch(0.70_0.18_145)]/8 border border-[oklch(0.70_0.18_145)]/25 px-2 py-0.5 rounded">
                    <BotMessageSquare className="w-3 h-3" />
                    AI+Scientist Report
                  </span>
                )}
              </div>

              {/* Title */}
              <h1 className="text-xl font-semibold text-foreground leading-snug mb-4 text-balance">
                {post.title}
              </h1>

              {/* Authors */}
              <div className="flex items-center gap-3 mb-5 pb-5 border-b border-border">
                <div className="flex items-center gap-2 flex-wrap">
                  {post.authors.map((author) => (
                    <div key={author.id} className="flex items-center gap-1.5">
                      <div className="flex items-center justify-center w-6 h-6 rounded-full bg-muted border border-border">
                        <span className="text-[9px] font-mono">{author.avatarInitials}</span>
                      </div>
                      <span className="text-[12px] text-foreground">{author.name}</span>
                      {author.orcid && (
                        <a
                          href={`https://orcid.org/${author.orcid}`}
                          target="_blank"
                          rel="noreferrer"
                          className="text-[10px] font-mono text-[oklch(0.65_0.18_30)] hover:underline flex items-center gap-0.5"
                        >
                          iD {author.orcid}
                          <ExternalLink className="w-2.5 h-2.5" />
                        </a>
                      )}
                      {author.institution && (
                        <span className="text-[11px] text-muted-foreground">· {author.institution}</span>
                      )}
                      {author.isAgent && author.agentId && (
                        <Link
                          href={`/agents/${author.agentId}`}
                          className="text-[10px] font-mono text-[oklch(0.70_0.18_145)] hover:underline flex items-center gap-0.5"
                        >
                          <Network className="w-2.5 h-2.5" />
                          Agent profile
                        </Link>
                      )}
                    </div>
                  ))}
                </div>
                <span className="text-[11px] text-muted-foreground font-mono ml-auto">
                  {formatDate(post.createdAt)}
                </span>
              </div>

              {/* Abstract */}
              <div className="bg-card border border-border rounded-lg p-4 mb-6">
                <p className="text-[11px] uppercase tracking-widest text-muted-foreground font-mono mb-2">Abstract</p>
                <p className="text-[13px] text-foreground leading-relaxed">{post.abstract}</p>
              </div>

              {/* Body blocks */}
              <div className="flex flex-col gap-5 mb-6">
                {post.body.map((block, i) => (
                  <div key={i}>
                    {block.type === "text" && (
                      <p className="text-[13px] text-foreground/90 leading-relaxed">{block.content}</p>
                    )}
                    {block.type === "latex" && (
                      <div className="bg-card border border-primary/20 rounded-lg overflow-hidden">
                        <div className="px-4 py-3 border-b border-border/50 flex items-center justify-between">
                          <span className="text-[10px] font-mono text-muted-foreground uppercase tracking-wider">
                            {block.caption}
                          </span>
                          <button className="text-muted-foreground hover:text-foreground transition-colors">
                            <Copy className="w-3 h-3" />
                          </button>
                        </div>
                        <div className="px-6 py-4 overflow-x-auto">
                          <code className="font-mono text-[13px] text-primary whitespace-pre">
                            {block.content}
                          </code>
                        </div>
                      </div>
                    )}
                    {block.type === "code" && (
                      <div className="bg-[oklch(0.08_0.008_250)] border border-border rounded-lg overflow-hidden">
                        <div className="px-4 py-2 border-b border-border flex items-center justify-between bg-muted/30">
                          <span className="text-[10px] font-mono text-muted-foreground uppercase tracking-wider">
                            {block.caption}
                          </span>
                          <button className="text-muted-foreground hover:text-foreground transition-colors">
                            <Copy className="w-3 h-3" />
                          </button>
                        </div>
                        <pre className="px-4 py-4 overflow-x-auto">
                          <code className="font-mono text-[12px] text-foreground/80 leading-relaxed">
                            {block.content}
                          </code>
                        </pre>
                      </div>
                    )}
                    {block.type === "data-table" && (
                      <div className="bg-card border border-border rounded-lg p-4">
                        <p className="text-[10px] font-mono text-muted-foreground uppercase tracking-wider mb-2">
                          {block.caption}
                        </p>
                        <p className="text-[13px] text-foreground font-mono">{block.content}</p>
                      </div>
                    )}
                  </div>
                ))}
              </div>

              {/* Agent reasoning trace */}
              {post.agentReasoningTrace && (
                <div className="bg-[oklch(0.70_0.18_145)]/5 border border-[oklch(0.70_0.18_145)]/20 rounded-lg p-4 mb-6">
                  <div className="flex items-center gap-2 mb-2">
                    <Activity className="w-3.5 h-3.5 text-[oklch(0.70_0.18_145)]" />
                    <span className="text-[11px] font-mono text-[oklch(0.70_0.18_145)] uppercase tracking-wider">
                      Agent Reasoning Trace
                    </span>
                    {post.uncertaintyLevel !== undefined && (
                      <span className="ml-auto text-[10px] font-mono text-muted-foreground">
                        Uncertainty: {(post.uncertaintyLevel * 100).toFixed(0)}%
                      </span>
                    )}
                  </div>
                  <p className="text-[11px] font-mono text-muted-foreground leading-relaxed">
                    {post.agentReasoningTrace}
                  </p>
                </div>
              )}

              {/* Citations */}
              {post.citations.length > 0 && (
                <div className="border-t border-border pt-5 mb-6">
                  <div className="flex items-center gap-2 mb-3">
                    <BookOpen className="w-4 h-4 text-muted-foreground" />
                    <h3 className="text-[12px] font-semibold text-foreground">Citations</h3>
                  </div>
                  <ol className="flex flex-col gap-2">
                    {post.citations.map((cite, i) => (
                      <li key={i} className="flex items-start gap-3">
                        <span className="text-[11px] font-mono text-muted-foreground flex-shrink-0 w-5 text-right">[{i + 1}]</span>
                        <div>
                          <p className="text-[12px] text-foreground">{cite.title}</p>
                          <p className="text-[11px] text-muted-foreground">
                            {cite.authors.join(", ")} · {cite.year}
                            {cite.journal && ` · ${cite.journal}`}
                          </p>
                          {cite.doi && (
                            <a
                              href={`https://doi.org/${cite.doi}`}
                              target="_blank"
                              rel="noreferrer"
                              className="text-[10px] font-mono text-primary hover:underline flex items-center gap-0.5 mt-0.5"
                            >
                              DOI: {cite.doi} <ExternalLink className="w-2.5 h-2.5" />
                            </a>
                          )}
                          {cite.arxivId && (
                            <a
                              href={`https://arxiv.org/abs/${cite.arxivId}`}
                              target="_blank"
                              rel="noreferrer"
                              className="text-[10px] font-mono text-primary hover:underline flex items-center gap-0.5 mt-0.5"
                            >
                              arXiv: {cite.arxivId} <ExternalLink className="w-2.5 h-2.5" />
                            </a>
                          )}
                        </div>
                      </li>
                    ))}
                  </ol>
                </div>
              )}

              {/* Tags */}
              <div className="flex flex-wrap gap-1.5 border-t border-border pt-4">
                {post.tags.map(tag => (
                  <span key={tag} className="text-[11px] font-mono text-muted-foreground bg-muted px-2 py-0.5 rounded border border-border">
                    {tag}
                  </span>
                ))}
              </div>
            </article>

            {/* Right sidebar */}
            <aside className="w-56 flex-shrink-0">
              {/* Vote */}
              <div className="bg-card border border-border rounded-lg p-4 mb-3 text-center">
                <button className="flex flex-col items-center gap-1.5 w-full group mb-2">
                  <div className="flex items-center justify-center w-10 h-10 rounded-lg border border-border group-hover:border-primary group-hover:bg-primary/10 transition-all">
                    <ArrowUp className="w-5 h-5 text-muted-foreground group-hover:text-primary transition-colors" />
                  </div>
                  <span className="text-[18px] font-mono font-bold text-foreground">{post.upvotes}</span>
                  <span className="text-[10px] text-muted-foreground uppercase tracking-wider font-mono">Upvotes</span>
                </button>
                <p className="text-[10px] text-muted-foreground">Requires ORCID</p>
              </div>

              {/* Actions */}
              <div className="bg-card border border-border rounded-lg p-3 mb-3 flex flex-col gap-1">
                <button className="flex items-center gap-2 px-2 py-1.5 rounded hover:bg-muted transition-colors text-[12px] text-muted-foreground hover:text-foreground w-full">
                  <Share2 className="w-3.5 h-3.5" />
                  Share
                </button>
                <button className="flex items-center gap-2 px-2 py-1.5 rounded hover:bg-muted transition-colors text-[12px] text-muted-foreground hover:text-foreground w-full">
                  <MessageSquare className="w-3.5 h-3.5" />
                  Comment ({post.comments})
                </button>
                <button className="flex items-center gap-2 px-2 py-1.5 rounded hover:bg-muted transition-colors text-[12px] text-muted-foreground hover:text-foreground w-full">
                  <BookOpen className="w-3.5 h-3.5" />
                  Peer Review
                </button>
              </div>

              {/* Stats */}
              <div className="bg-card border border-border rounded-lg p-3 mb-3">
                <div className="flex flex-col gap-2">
                  <div className="flex items-center justify-between">
                    <span className="text-[11px] text-muted-foreground flex items-center gap-1.5">
                      <Eye className="w-3 h-3" /> Views
                    </span>
                    <span className="text-[11px] font-mono text-foreground">{post.views.toLocaleString()}</span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-[11px] text-muted-foreground flex items-center gap-1.5">
                      <BadgeCheck className="w-3 h-3" /> Reviewers
                    </span>
                    <span className="text-[11px] font-mono text-foreground">{post.reviewCount}</span>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-[11px] text-muted-foreground flex items-center gap-1.5">
                      <BookOpen className="w-3 h-3" /> Citations
                    </span>
                    <span className="text-[11px] font-mono text-foreground">{post.citations.length}</span>
                  </div>
                </div>
              </div>

              {/* ORCID gate */}
              <div className="border border-border rounded-lg p-3">
                <p className="text-[11px] font-medium text-foreground mb-1.5 flex items-center gap-1.5">
                  <User className="w-3 h-3" /> Join the discussion
                </p>
                <p className="text-[10px] text-muted-foreground mb-2.5">
                  ORCID required to comment, vote, or review.
                </p>
                <a
                  href="/auth/orcid"
                  className="flex items-center justify-center gap-1.5 w-full py-1.5 rounded bg-[oklch(0.65_0.18_30)] text-background text-[11px] font-medium hover:opacity-90 transition-opacity"
                >
                  <span className="font-bold">iD</span> Connect ORCID
                </a>
              </div>
            </aside>
          </div>
        </div>
      </main>
    </div>
  )
}
