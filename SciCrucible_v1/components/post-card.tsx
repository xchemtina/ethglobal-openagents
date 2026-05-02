"use client"

import Link from "next/link"
import { Post, POST_TYPE_LABELS, REVIEW_STATUS_LABELS } from "@/lib/data"
import { cn } from "@/lib/utils"
import {
  ArrowUp,
  Eye,
  MessageSquare,
  BotMessageSquare,
  BadgeCheck,
  Clock,
  DatabaseZap,
  FlaskConical,
  BookOpen,
  CircleHelp,
  FileText,
  AlertTriangle,
} from "lucide-react"
import { MathText } from "@/components/math-block"

const POST_TYPE_META: Record<string, { icon: React.ReactNode; color: string; accent: string }> = {
  "open-problem": {
    icon: <CircleHelp className="w-3 h-3" />,
    color: "oklch(0.70 0.16 80)",
    accent: "oklch(0.70 0.16 80)",
  },
  "derivation": {
    icon: <BookOpen className="w-3 h-3" />,
    color: "oklch(0.72 0.20 195)",
    accent: "oklch(0.72 0.20 195)",
  },
  "experimental": {
    icon: <FlaskConical className="w-3 h-3" />,
    color: "oklch(0.70 0.20 30)",
    accent: "oklch(0.70 0.20 30)",
  },
  "agent-report": {
    icon: <BotMessageSquare className="w-3 h-3" />,
    color: "oklch(0.72 0.20 145)",
    accent: "oklch(0.72 0.20 145)",
  },
  "machine-data": {
    icon: <DatabaseZap className="w-3 h-3" />,
    color: "oklch(0.68 0.16 170)",
    accent: "oklch(0.68 0.16 170)",
  },
}

const REVIEW_META: Record<string, { icon: React.ReactNode; color: string; label: string }> = {
  "preprint":      { icon: <FileText className="w-3 h-3" />,   color: "oklch(0.48 0.012 250)", label: REVIEW_STATUS_LABELS["preprint"] },
  "under-review":  { icon: <Clock className="w-3 h-3" />,      color: "oklch(0.70 0.16 80)",   label: REVIEW_STATUS_LABELS["under-review"] },
  "peer-reviewed": { icon: <BadgeCheck className="w-3 h-3" />, color: "oklch(0.72 0.20 145)",  label: REVIEW_STATUS_LABELS["peer-reviewed"] },
  "contested":     { icon: <AlertTriangle className="w-3 h-3" />, color: "oklch(0.68 0.22 27)", label: REVIEW_STATUS_LABELS["contested"] },
}

function formatDate(iso: string) {
  return new Date(iso).toLocaleDateString("en-GB", { day: "2-digit", month: "short", year: "numeric" })
}

function formatCount(n: number) {
  if (n >= 1000) return `${(n / 1000).toFixed(1)}k`
  return String(n)
}

export function PostCard({ post }: { post: Post }) {
  const primaryAuthor = post.authors[0]
  const hasMoreAuthors = post.authors.length > 1
  const typeMeta   = POST_TYPE_META[post.type]   ?? POST_TYPE_META["derivation"]
  const reviewMeta = REVIEW_META[post.reviewStatus] ?? REVIEW_META["preprint"]

  return (
    <Link href={`/post/${post.id}`} className="block group">
      <article
        className="relative rounded overflow-hidden transition-all duration-150"
        style={{
          background: "oklch(0.12 0 0)",
          border: "1px solid oklch(0.22 0 0)",
        }}
        onMouseEnter={e => {
          const el = e.currentTarget as HTMLElement
          el.style.borderColor = `${typeMeta.accent}66`
          el.style.background = "oklch(0.14 0 0)"
        }}
        onMouseLeave={e => {
          const el = e.currentTarget as HTMLElement
          el.style.borderColor = "oklch(0.22 0 0)"
          el.style.background = "oklch(0.12 0 0)"
        }}
      >
        {/* Left accent bar — post type colour */}
        <div
          className="absolute left-0 top-0 bottom-0 w-[3px]"
          style={{ background: typeMeta.accent, opacity: 0.85 }}
        />

        <div className="pl-5 pr-5 pt-4 pb-4">

          {/* ── Badge row ───────────────────────────────────────────── */}
          <div className="flex items-center gap-2 mb-3 flex-wrap">
            {/* Post type */}
            <span
              className="inline-flex items-center gap-1.5 text-[10px] font-mono px-2 py-0.5 rounded border"
              style={{
                color: typeMeta.color,
                borderColor: `${typeMeta.color}44`,
                background: `${typeMeta.color}10`,
              }}
            >
              {typeMeta.icon}
              {POST_TYPE_LABELS[post.type]}
            </span>

            {/* AI+Scientist indicator */}
            {primaryAuthor.isAgent && (
              <span
                className="inline-flex items-center gap-1 text-[10px] font-mono px-2 py-0.5 rounded border"
                style={{
                  color: "oklch(0.72 0.20 145)",
                  borderColor: "oklch(0.72 0.20 145 / 0.30)",
                  background: "oklch(0.72 0.20 145 / 0.08)",
                }}
              >
                <BotMessageSquare className="w-3 h-3" />
                AI+Scientist
              </span>
            )}

            {/* Review status */}
            <span
              className="inline-flex items-center gap-1 text-[10px] font-mono"
              style={{ color: reviewMeta.color }}
            >
              {reviewMeta.icon}
              {reviewMeta.label}
            </span>

            {/* DOI or arXiv chip */}
            {post.doi && (
              <span className="ml-auto text-[9px] font-mono text-muted-foreground border border-border px-1.5 py-0.5 rounded">
                DOI
              </span>
            )}
            {post.arxivId && !post.doi && (
              <span
                className="ml-auto text-[9px] font-mono px-1.5 py-0.5 rounded border"
                style={{ color: "oklch(0.76 0.17 192)", borderColor: "oklch(0.76 0.17 192 / 0.30)", background: "oklch(0.76 0.17 192 / 0.06)" }}
              >
                arXiv
              </span>
            )}
          </div>

          {/* ── Title ───────────────────────────────────────────────── */}
          <h2
            className="text-[14px] font-semibold leading-snug mb-2 text-balance transition-colors duration-100"
            style={{ color: "oklch(0.92 0.006 240)" }}
          >
            <span className="group-hover:text-primary transition-colors duration-100">
              {post.title}
            </span>
          </h2>

          {/* ── Abstract ────────────────────────────────────────────── */}
          <p className="text-[12px] leading-relaxed mb-3 line-clamp-2" style={{ color: "oklch(0.48 0.012 250)" }}>
            <MathText text={post.abstract} />
          </p>

          {/* ── Tags ────────────────────────────────────────────────── */}
          {post.tags.length > 0 && (
            <div className="flex flex-wrap gap-1 mb-3">
              {post.tags.slice(0, 5).map(tag => (
                <span
                  key={tag}
                  className="text-[10px] font-mono px-1.5 py-0.5 rounded"
                  style={{
                    color: "oklch(0.48 0.006 60)",
                    background: "oklch(0.15 0 0)",
                    border: "1px solid oklch(0.22 0 0)",
                  }}
                >
                  {tag}
                </span>
              ))}
              {post.tags.length > 5 && (
                <span className="text-[10px] font-mono" style={{ color: "oklch(0.48 0.012 250)" }}>
                  +{post.tags.length - 5}
                </span>
              )}
            </div>
          )}

          {/* ── Metadata row ───────���────────────────────────────────── */}
          <div
            className="flex items-center justify-between pt-3"
            style={{ borderTop: "1px solid oklch(0.19 0 0)" }}
          >
            {/* Author */}
            <div className="flex items-center gap-2 min-w-0">
              <div
                className="w-5 h-5 rounded-full flex items-center justify-center flex-shrink-0"
                style={{ background: "oklch(0.17 0 0)", border: "1px solid oklch(0.24 0 0)" }}
              >
                <span className="text-[8px] font-mono" style={{ color: "oklch(0.50 0.006 60)" }}>
                  {primaryAuthor.avatarInitials}
                </span>
              </div>
              <span className="text-[11px] truncate" style={{ color: "oklch(0.56 0.006 60)" }}>
                {primaryAuthor.name}
                {hasMoreAuthors && (
                  <span style={{ color: "oklch(0.42 0.006 60)" }}> +{post.authors.length - 1}</span>
                )}
              </span>
              {primaryAuthor.verified && (
                <BadgeCheck className="w-3 h-3 flex-shrink-0" style={{ color: "oklch(0.76 0.17 192)" }} />
              )}
              {primaryAuthor.institution && (
                <span className="text-[10px] truncate hidden sm:block" style={{ color: "oklch(0.40 0.006 60)" }}>
                  · {primaryAuthor.institution}
                </span>
              )}
            </div>

            {/* Stats */}
            <div className="flex items-center gap-3 flex-shrink-0">
              <Stat icon={<ArrowUp className="w-3 h-3" />}       value={formatCount(post.upvotes)} />
              <Stat icon={<Eye className="w-3 h-3" />}           value={formatCount(post.views)} />
              <Stat icon={<MessageSquare className="w-3 h-3" />} value={String(post.comments)} />
              <span className="text-[10px] font-mono hidden md:block" style={{ color: "oklch(0.40 0.006 60)" }}>
                {formatDate(post.createdAt)}
              </span>
            </div>
          </div>

          {/* ── Agent reasoning trace ────────────────────────────────── */}
          {post.agentReasoningTrace && (
            <div
              className="mt-3 rounded px-3 py-2"
              style={{
                background: "oklch(0.07 0 0)",
                border: "1px solid oklch(0.19 0 0)",
              }}
            >
              <p
                className="text-[9px] font-mono uppercase tracking-[0.18em] mb-1"
                style={{ color: "oklch(0.70 0.18 148)" }}
              >
                Reasoning trace
              </p>
              <p className="text-[11px] font-mono line-clamp-1" style={{ color: "oklch(0.48 0.006 60)" }}>
                {post.agentReasoningTrace}
              </p>
            </div>
          )}
        </div>
      </article>
    </Link>
  )
}

function Stat({ icon, value }: { icon: React.ReactNode; value: string }) {
  return (
    <span className="flex items-center gap-1 text-[11px] font-mono" style={{ color: "oklch(0.48 0.006 60)" }}>
      {icon}
      {value}
    </span>
  )
}
