/**
 * Reticle — small HUD crosshair marker. Used at panel intersections and as
 * orientation anchors. Pure SVG, deterministic, no animation by default.
 *
 * Variants:
 *   "cross"  — simple + crosshair (default)
 *   "target" — concentric rings + crosshair (for hero anchor points)
 *   "tick"   — short vertical tick mark with horizontal cap
 *   "id"     — crosshair with adjacent monospace ID label
 */

import type { ReactNode } from "react"

type Variant = "cross" | "target" | "tick" | "id"

interface ReticleProps {
  variant?: Variant
  size?: number
  color?: string
  className?: string
  style?: React.CSSProperties
  children?: ReactNode  // used as ID label when variant="id"
}

export function Reticle({
  variant = "cross",
  size = 12,
  color = "oklch(0.76 0.17 192 / 0.55)",
  className,
  style,
  children,
}: ReticleProps) {
  if (variant === "id") {
    return (
      <span
        className={`inline-flex items-center gap-1.5 ${className ?? ""}`}
        style={style}
      >
        <Reticle variant="cross" size={size} color={color} />
        <span
          className="font-mono text-[9px] uppercase tracking-[0.22em]"
          style={{ color: "oklch(0.46 0.006 60)" }}
        >
          {children}
        </span>
      </span>
    )
  }

  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 12 12"
      className={className}
      style={style}
      aria-hidden
    >
      {variant === "target" && (
        <>
          <circle cx="6" cy="6" r="5"   fill="none" stroke={color} strokeWidth="0.5" />
          <circle cx="6" cy="6" r="2.5" fill="none" stroke={color} strokeWidth="0.5" />
          <circle cx="6" cy="6" r="0.6" fill={color} />
        </>
      )}
      {(variant === "cross" || variant === "target") && (
        <>
          <line x1="6" y1="0" x2="6" y2="3.5" stroke={color} strokeWidth="0.7" />
          <line x1="6" y1="8.5" x2="6" y2="12" stroke={color} strokeWidth="0.7" />
          <line x1="0" y1="6" x2="3.5" y2="6" stroke={color} strokeWidth="0.7" />
          <line x1="8.5" y1="6" x2="12" y2="6" stroke={color} strokeWidth="0.7" />
        </>
      )}
      {variant === "tick" && (
        <>
          <line x1="6" y1="0" x2="6" y2="9" stroke={color} strokeWidth="0.7" />
          <line x1="3" y1="9" x2="9" y2="9" stroke={color} strokeWidth="0.7" />
        </>
      )}
    </svg>
  )
}

/**
 * SectionStamp — small "01 / 04 — OVERVIEW" marker. Used in panel headers.
 */
export function SectionStamp({
  index,
  total,
  label,
  color = "oklch(0.76 0.17 192)",
}: {
  index: number
  total: number
  label: string
  color?: string
}) {
  return (
    <span className="inline-flex items-center gap-2 font-mono text-[9px] uppercase tracking-[0.22em]">
      <span style={{ color }} className="font-semibold tabular">
        {String(index).padStart(2, "0")}
      </span>
      <span style={{ color: "oklch(0.30 0.006 60)" }}>/</span>
      <span style={{ color: "oklch(0.40 0.006 60)" }} className="tabular">
        {String(total).padStart(2, "0")}
      </span>
      <span style={{ color: "oklch(0.24 0.006 60)" }}>—</span>
      <span style={{ color: "oklch(0.62 0.006 60)" }} className="font-semibold">
        {label}
      </span>
    </span>
  )
}

/**
 * BlockBar — block-character capacity/utilisation bar.
 * Renders ▮▮▮▮▯▯▯▯▯▯ given fill (0..1) and total cells.
 */
export function BlockBar({
  fill,
  cells = 12,
  color = "oklch(0.76 0.17 192)",
  emptyColor = "oklch(0.22 0 0)",
}: {
  fill: number
  cells?: number
  color?: string
  emptyColor?: string
}) {
  const filled = Math.round(Math.max(0, Math.min(1, fill)) * cells)
  return (
    <span className="bar-blocks tabular" aria-hidden>
      <span style={{ color }}>{"▮".repeat(filled)}</span>
      <span style={{ color: emptyColor }}>{"▮".repeat(cells - filled)}</span>
    </span>
  )
}
