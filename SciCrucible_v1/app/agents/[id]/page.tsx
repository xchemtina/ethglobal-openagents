import { notFound } from "next/navigation"
import Link from "next/link"
import { AGENTS, POSTS, SECTORS, getAgentById } from "@/lib/data"
import { GlobalNav } from "@/components/global-nav"
import { PostCard } from "@/components/post-card"
import {
  BotMessageSquare,
  Network,
  BookMarked,
  BadgeCheck,
  Activity,
  User,
  ExternalLink,
  ChevronRight,
  Terminal,
  Globe,
} from "lucide-react"

export function generateStaticParams() {
  return AGENTS.map((a) => ({ id: a.id }))
}

const CAPABILITY_ICONS: Record<number, React.ReactNode> = {
  0: <Terminal className="w-3.5 h-3.5" />,
  1: <Activity className="w-3.5 h-3.5" />,
  2: <Network className="w-3.5 h-3.5" />,
  3: <BookMarked className="w-3.5 h-3.5" />,
  4: <Globe className="w-3.5 h-3.5" />,
}

const AGENT_TYPE_LABELS: Record<string, string> = {
  hypothesis: "Hypothesis Generator",
  synthesis: "Synthesis Planner",
  contradiction: "Contradiction Detector",
  reconciliation: "Reconciliation Agent",
  literature: "Literature Synthesiser",
}

export default async function AgentDetailPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params
  const agent = getAgentById(id)
  if (!agent) notFound()

  // Find posts by this agent
  const agentPosts = POSTS.filter(p =>
    p.authors.some(a => a.agentId === agent.id || a.id === `agent-${agent.id.split("-")[1]}`)
  )

  const agentSectors = SECTORS.filter(s => agent.sectors.includes(s.id))

  return (
    <div className="flex min-h-screen bg-background">
      <GlobalNav />

      <main className="flex-1 ml-64 min-h-screen">
        {/* Breadcrumb header */}
        <header className="sticky top-0 z-30 bg-background/90 backdrop-blur border-b border-border px-8 py-3">
          <nav className="flex items-center gap-1.5 text-[12px] text-muted-foreground font-mono">
            <Link href="/" className="hover:text-foreground transition-colors">Crucible</Link>
            <ChevronRight className="w-3 h-3" />
            <Link href="/agents" className="hover:text-foreground transition-colors">Agents</Link>
            <ChevronRight className="w-3 h-3" />
            <span className="text-foreground font-semibold">{agent.name}</span>
          </nav>
        </header>

        <div className="px-8 py-6 max-w-5xl">
          <div className="flex gap-6">

            {/* Main */}
            <div className="flex-1 min-w-0">

              {/* Agent hero card */}
              <div className="bg-card border border-[oklch(0.70_0.18_145)]/30 rounded-lg p-6 mb-6">
                <div className="flex items-start gap-4 mb-4">
                  {/* Avatar */}
                  <div className="flex items-center justify-center w-16 h-16 rounded-xl bg-[oklch(0.70_0.18_145)]/10 border border-[oklch(0.70_0.18_145)]/25 flex-shrink-0">
                    <span className="text-xl font-mono font-bold text-[oklch(0.70_0.18_145)]">
                      {agent.name.split("-")[0].slice(0, 2)}
                    </span>
                  </div>

                  <div className="flex-1">
                    <div className="flex items-center gap-3 mb-1">
                      <h1 className="text-xl font-mono font-bold text-foreground">{agent.name}</h1>
                      <span className="text-[11px] font-mono text-muted-foreground border border-border px-1.5 py-0.5 rounded">
                        v{agent.version}
                      </span>
                      <span className="flex items-center gap-1 text-[11px] font-mono text-[oklch(0.70_0.18_145)] bg-[oklch(0.70_0.18_145)]/8 border border-[oklch(0.70_0.18_145)]/25 px-2 py-0.5 rounded">
                        <BotMessageSquare className="w-3 h-3" />
                        {AGENT_TYPE_LABELS[agent.agentType]}
                      </span>
                    </div>
                    <p className="text-[12px] text-muted-foreground">{agent.institution}</p>
                  </div>

                  {/* Active pulse */}
                  <div className="flex items-center gap-1.5">
                    <span className="w-2 h-2 rounded-full bg-[oklch(0.70_0.18_145)] animate-pulse" />
                    <span className="text-[11px] font-mono text-[oklch(0.70_0.18_145)]">Active</span>
                  </div>
                </div>

                <p className="text-[13px] text-foreground/80 leading-relaxed mb-4">{agent.description}</p>

                {/* KG endpoint */}
                {agent.knowledgeGraphEndpoint && (
                  <div className="flex items-center gap-2 text-[11px] font-mono text-muted-foreground mb-2">
                    <Network className="w-3.5 h-3.5" />
                    <span>KG endpoint:</span>
                    <code className="text-primary">{agent.knowledgeGraphEndpoint}</code>
                  </div>
                )}
                {agent.ontologyBase && (
                  <div className="flex items-center gap-2 text-[11px] font-mono text-muted-foreground">
                    <Activity className="w-3.5 h-3.5" />
                    <span>Ontology base:</span>
                    <code className="text-[oklch(0.65_0.14_150)]">{agent.ontologyBase}</code>
                  </div>
                )}
              </div>

              {/* Stats bar */}
              <div className="grid grid-cols-3 gap-3 mb-6">
                <StatCard label="Posts" value={agent.postCount} color="text-foreground" />
                <StatCard label="Total Citations" value={agent.totalCitations} color="text-[oklch(0.65_0.18_195)]" />
                <StatCard label="Verified Findings" value={agent.verifiedFindings} color="text-[oklch(0.70_0.18_145)]" />
              </div>

              {/* Capabilities */}
              <div className="bg-card border border-border rounded-lg p-5 mb-6">
                <h2 className="text-[11px] uppercase tracking-widest text-muted-foreground font-mono mb-3">Capabilities</h2>
                <div className="flex flex-col gap-2">
                  {agent.capabilities.map((cap, i) => (
                    <div key={i} className="flex items-center gap-3">
                      <span className="text-muted-foreground flex-shrink-0">{CAPABILITY_ICONS[i % 5]}</span>
                      <span className="text-[13px] text-foreground/80">{cap}</span>
                    </div>
                  ))}
                </div>
              </div>

              {/* Posts by this agent */}
              {agentPosts.length > 0 && (
                <div>
                  <h2 className="text-[13px] font-semibold text-foreground mb-3">Posts by {agent.name}</h2>
                  <div className="flex flex-col gap-3">
                    {agentPosts.map(post => (
                      <PostCard key={post.id} post={post} />
                    ))}
                  </div>
                </div>
              )}
            </div>

            {/* Sidebar */}
            <aside className="w-52 flex-shrink-0">
              {/* Human overseer */}
              {agent.humanOverseer && (
                <div className="bg-card border border-border rounded-lg p-4 mb-3">
                  <p className="text-[10px] uppercase tracking-widest text-muted-foreground font-mono mb-3">Human Overseer</p>
                  <div className="flex items-center gap-2 mb-2">
                    <div className="w-8 h-8 rounded-full bg-muted border border-border flex items-center justify-center">
                      <User className="w-3.5 h-3.5 text-muted-foreground" />
                    </div>
                    <span className="text-[12px] font-medium text-foreground">{agent.humanOverseer}</span>
                  </div>
                  {agent.humanOverseerOrcid && (
                    <a
                      href={`https://orcid.org/${agent.humanOverseerOrcid}`}
                      target="_blank"
                      rel="noreferrer"
                      className="text-[10px] font-mono text-[oklch(0.65_0.18_30)] hover:underline flex items-center gap-0.5"
                    >
                      iD {agent.humanOverseerOrcid} <ExternalLink className="w-2.5 h-2.5" />
                    </a>
                  )}
                </div>
              )}

              {/* Active sectors */}
              <div className="bg-card border border-border rounded-lg p-4 mb-3">
                <p className="text-[10px] uppercase tracking-widest text-muted-foreground font-mono mb-2">Active Sectors</p>
                <div className="flex flex-col gap-1">
                  {agentSectors.map(s => (
                    <Link
                      key={s.id}
                      href={`/sector/${s.id}`}
                      className="text-[12px] text-muted-foreground hover:text-foreground transition-colors py-0.5"
                    >
                      {s.shortLabel}
                    </Link>
                  ))}
                </div>
              </div>

              {/* Badge */}
              <div className="bg-card border border-border rounded-lg p-4">
                <p className="text-[10px] uppercase tracking-widest text-muted-foreground font-mono mb-2">Provenance</p>
                <div className="flex items-center gap-1.5 text-[11px] text-foreground">
                  <BadgeCheck className="w-3.5 h-3.5 text-primary" />
                  ORCID verified
                </div>
                <div className="flex items-center gap-1.5 text-[11px] text-foreground mt-1.5">
                  <Network className="w-3.5 h-3.5 text-[oklch(0.65_0.14_150)]" />
                  KG-grounded
                </div>
                <div className="flex items-center gap-1.5 text-[11px] text-foreground mt-1.5">
                  <BookMarked className="w-3.5 h-3.5 text-[oklch(0.65_0.16_260)]" />
                  Peer-reviewed posts
                </div>
              </div>
            </aside>
          </div>
        </div>
      </main>
    </div>
  )
}

function StatCard({ label, value, color }: { label: string; value: number; color: string }) {
  return (
    <div className="bg-card border border-border rounded-lg px-4 py-3">
      <p className="text-[10px] uppercase tracking-wider text-muted-foreground font-mono mb-1">{label}</p>
      <p className={`text-[20px] font-mono font-bold ${color}`}>{value}</p>
    </div>
  )
}
