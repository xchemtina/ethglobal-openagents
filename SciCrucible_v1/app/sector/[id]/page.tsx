import { notFound } from "next/navigation"
import { SECTORS, POSTS, getSectorById, POST_TYPE_LABELS } from "@/lib/data"
import { GlobalNav } from "@/components/global-nav"
import { PostCard } from "@/components/post-card"
import { OrcidGateBanner } from "@/components/orcid-gate"
import { LayoutGrid, List, SlidersHorizontal } from "lucide-react"

export function generateStaticParams() {
  return SECTORS.map((s) => ({ id: s.id }))
}

const SECTOR_HEADER_COLORS: Record<string, string> = {
  "quantum-chemistry": "border-[oklch(0.65_0.18_195)]/40",
  "physical-chemistry": "border-[oklch(0.65_0.14_150)]/40",
  "condensed-matter": "border-[oklch(0.65_0.16_260)]/40",
  "qm-qft": "border-[oklch(0.65_0.18_220)]/40",
  "classical-dynamics": "border-[oklch(0.65_0.14_80)]/40",
  "exp-inorganic": "border-[oklch(0.65_0.18_30)]/40",
  "exp-physical": "border-[oklch(0.65_0.18_30)]/40",
  "automated-synthesis": "border-[oklch(0.65_0.14_170)]/40",
}

const SECTOR_ACCENT_COLORS: Record<string, string> = {
  "quantum-chemistry": "text-[oklch(0.65_0.18_195)]",
  "physical-chemistry": "text-[oklch(0.65_0.14_150)]",
  "condensed-matter": "text-[oklch(0.65_0.16_260)]",
  "qm-qft": "text-[oklch(0.65_0.18_220)]",
  "classical-dynamics": "text-[oklch(0.65_0.14_80)]",
  "exp-inorganic": "text-[oklch(0.65_0.18_30)]",
  "exp-physical": "text-[oklch(0.65_0.18_30)]",
  "automated-synthesis": "text-[oklch(0.65_0.14_170)]",
}

export default async function SectorPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params
  const sector = getSectorById(id)
  if (!sector) notFound()

  // Get all posts for this sector, fall back to all posts if empty
  const sectorPosts = POSTS.filter(p => p.sectorId === id)
  const displayPosts = sectorPosts.length > 0 ? sectorPosts : POSTS

  const postTypes = Array.from(new Set(displayPosts.map(p => p.type)))

  return (
    <div className="flex min-h-screen bg-background">
      <GlobalNav />

      <main className="flex-1 ml-64 min-h-screen">
        {/* Sector header */}
        <header className={`border-b ${SECTOR_HEADER_COLORS[id] ?? "border-border"} bg-background/90 backdrop-blur sticky top-0 z-30 px-8 py-4`}>
          <div className="flex items-center justify-between max-w-6xl">
            <div>
              <div className="flex items-center gap-2 mb-0.5">
                <span className="text-[10px] font-mono uppercase tracking-widest text-muted-foreground">Sector</span>
              </div>
              <h1 className={`text-lg font-semibold ${SECTOR_ACCENT_COLORS[id] ?? "text-foreground"}`}>
                {sector.label}
              </h1>
              <p className="text-[12px] text-muted-foreground mt-0.5">{sector.description}</p>
            </div>
            <div className="flex items-center gap-2">
              <button className="p-2 rounded hover:bg-muted transition-colors">
                <List className="w-4 h-4 text-muted-foreground" />
              </button>
              <button className="p-2 rounded hover:bg-muted transition-colors">
                <LayoutGrid className="w-4 h-4 text-muted-foreground" />
              </button>
              <button className="flex items-center gap-1.5 px-3 py-1.5 rounded border border-border text-[12px] text-muted-foreground hover:text-foreground hover:bg-muted transition-colors">
                <SlidersHorizontal className="w-3.5 h-3.5" />
                Filter
              </button>
              <a
                href="/submit"
                className="px-3 py-1.5 rounded bg-primary text-primary-foreground text-[12px] font-medium hover:opacity-90 transition-opacity"
              >
                Submit
              </a>
            </div>
          </div>
        </header>

        <div className="px-8 py-6 max-w-6xl">

          {/* ORCID banner */}
          <div className="mb-5">
            <OrcidGateBanner />
          </div>

          {/* Sector stats row */}
          <div className="flex items-center gap-4 mb-5 pb-4 border-b border-border">
            <StatChip label="Posts" value={sector.postCount} />
            <StatChip label="Peer-reviewed" value={Math.floor(sector.postCount * 0.69)} />
            <StatChip label="Open problems" value={Math.floor(sector.postCount * 0.12)} />
            <StatChip label="Agent reports" value={Math.floor(sector.postCount * 0.18)} />
          </div>

          {/* Post type filters */}
          <div className="flex items-center gap-2 mb-5">
            <span className="text-[11px] text-muted-foreground font-mono">Type:</span>
            <div className="flex items-center gap-1.5 flex-wrap">
              <button className="px-2.5 py-1 rounded text-[11px] font-mono bg-primary/10 text-primary border border-primary/20 transition-colors">
                All
              </button>
              {postTypes.map(type => (
                <button
                  key={type}
                  className="px-2.5 py-1 rounded text-[11px] font-mono text-muted-foreground hover:text-foreground hover:bg-muted border border-transparent hover:border-border transition-colors"
                >
                  {POST_TYPE_LABELS[type]}
                </button>
              ))}
            </div>
          </div>

          {/* Posts feed */}
          {displayPosts.length > 0 ? (
            <div className="flex flex-col gap-3">
              {displayPosts.map(post => (
                <PostCard key={post.id} post={post} />
              ))}
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center py-24 text-center">
              <p className="text-[14px] text-muted-foreground mb-2">No posts yet in this sector.</p>
              <p className="text-[12px] text-muted-foreground/60">Be the first to submit an open problem or experimental result.</p>
            </div>
          )}
        </div>
      </main>
    </div>
  )
}

function StatChip({ label, value }: { label: string; value: number }) {
  return (
    <div className="flex items-center gap-2">
      <span className="text-[13px] font-mono font-semibold text-foreground">{value.toLocaleString()}</span>
      <span className="text-[11px] text-muted-foreground">{label}</span>
    </div>
  )
}
