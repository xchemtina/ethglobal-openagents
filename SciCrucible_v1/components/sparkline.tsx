"use client"

import { useId } from "react"

interface SparklineProps {
  data: number[]
  width?: number
  height?: number
  color?: string
  showFill?: boolean
  showDot?: boolean
  strokeWidth?: number
  className?: string
}

/**
 * Tiny inline SVG sparkline.
 *
 * - Auto-scales to data min/max
 * - Optional fill gradient beneath line
 * - Pulsing endpoint marker (SMIL animation, no JS)
 * - Each instance gets a unique gradient id via useId() to avoid collisions
 */
export function Sparkline({
  data,
  width = 120,
  height = 28,
  color = "oklch(0.76 0.17 192)",
  showFill = true,
  showDot = true,
  strokeWidth = 1.25,
  className,
}: SparklineProps) {
  const reactId = useId()
  const gradId = `spark-${reactId.replace(/[^a-zA-Z0-9]/g, "")}`

  if (data.length < 2) return null

  const min = Math.min(...data)
  const max = Math.max(...data)
  const range = max - min || 1

  // Reserve 2px top/bottom so stroke + dot aren't clipped
  const inset = 2
  const points = data.map((v, i) => {
    const x = (i / (data.length - 1)) * width
    const y = height - inset - ((v - min) / range) * (height - inset * 2)
    return [x, y] as const
  })

  const linePath = points
    .map(([x, y], i) => `${i === 0 ? "M" : "L"} ${x.toFixed(2)} ${y.toFixed(2)}`)
    .join(" ")

  const fillPath = `${linePath} L ${width.toFixed(2)} ${height.toFixed(2)} L 0 ${height.toFixed(2)} Z`

  const last = points[points.length - 1]

  return (
    <svg
      width={width}
      height={height}
      viewBox={`0 0 ${width} ${height}`}
      className={className}
      style={{ overflow: "visible" }}
      aria-hidden="true"
    >
      <defs>
        <linearGradient id={gradId} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor={color} stopOpacity="0.22" />
          <stop offset="100%" stopColor={color} stopOpacity="0" />
        </linearGradient>
      </defs>
      {showFill && <path d={fillPath} fill={`url(#${gradId})`} />}
      <path
        d={linePath}
        stroke={color}
        strokeWidth={strokeWidth}
        fill="none"
        strokeLinejoin="round"
        strokeLinecap="round"
      />
      {showDot && (
        <>
          <circle cx={last[0]} cy={last[1]} r="2" fill={color} />
          <circle cx={last[0]} cy={last[1]} r="2" fill={color} fillOpacity="0.35">
            <animate
              attributeName="r"
              from="2"
              to="7"
              dur="2s"
              repeatCount="indefinite"
            />
            <animate
              attributeName="fillOpacity"
              from="0.45"
              to="0"
              dur="2s"
              repeatCount="indefinite"
            />
          </circle>
        </>
      )}
    </svg>
  )
}

/**
 * Deterministic pseudo-random sparkline data generator.
 * Same `seed` always produces the same series — keeps SSR/CSR in sync.
 */
export function generateSparkData(
  seed: number,
  length: number = 30,
  baseline: number = 100,
  amplitude: number = 30,
  trend: number = 0.4,
): number[] {
  const out: number[] = []
  let value = baseline
  // Mulberry32 PRNG
  let s = seed >>> 0
  const rand = () => {
    s = (s + 0x6d2b79f5) >>> 0
    let t = s
    t = Math.imul(t ^ (t >>> 15), t | 1)
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61)
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
  for (let i = 0; i < length; i++) {
    const noise = (rand() - 0.5) * amplitude
    value = value + trend + noise * 0.3
    out.push(Math.max(0, value))
  }
  return out
}
