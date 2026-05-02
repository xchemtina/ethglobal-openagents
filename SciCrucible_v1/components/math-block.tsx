"use client"

import { useEffect, useRef } from "react"

// KaTeX is loaded from CDN to avoid SSR issues and bundle size.
// We inject it once and render into a div via dangerouslySetInnerHTML.

let katexLoaded = false
let katexLoadPromise: Promise<void> | null = null

function loadKatex(): Promise<void> {
  if (katexLoaded) return Promise.resolve()
  if (katexLoadPromise) return katexLoadPromise

  katexLoadPromise = new Promise((resolve, reject) => {
    if (typeof window === "undefined") { resolve(); return }

    // Stylesheet
    if (!document.querySelector('link[href*="katex"]')) {
      const link = document.createElement("link")
      link.rel = "stylesheet"
      link.href = "https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.css"
      link.crossOrigin = "anonymous"
      document.head.appendChild(link)
    }

    // Script
    const script = document.createElement("script")
    script.src = "https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.js"
    script.crossOrigin = "anonymous"
    script.defer = true
    script.onload = () => { katexLoaded = true; resolve() }
    script.onerror = reject
    document.head.appendChild(script)
  })

  return katexLoadPromise
}

interface MathBlockProps {
  /** Raw LaTeX string — do NOT include $ or $$ delimiters */
  math: string
  /** true = display (block) mode, false = inline mode */
  display?: boolean
  className?: string
}

export function MathBlock({ math, display = true, className }: MathBlockProps) {
  const ref = useRef<HTMLDivElement | HTMLSpanElement>(null)

  useEffect(() => {
    if (!ref.current) return
    let cancelled = false

    loadKatex().then(() => {
      if (cancelled || !ref.current) return
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const katex = (window as any).katex
      if (!katex) return
      try {
        katex.render(math, ref.current, {
          displayMode: display,
          throwOnError: false,
          errorColor: "oklch(0.65 0.20 25)",
          trust: false,
          strict: "warn",
        })
      } catch {
        if (ref.current) ref.current.textContent = math
      }
    })

    return () => { cancelled = true }
  }, [math, display])

  if (display) {
    return (
      <div
        ref={ref as React.RefObject<HTMLDivElement>}
        className={className}
        style={{
          overflowX: "auto",
          padding: "0.75rem 1rem",
          background: "oklch(0.07 0 0)",
          border: "1px solid oklch(0.19 0 0)",
          borderRadius: "0.1875rem",
          color: "oklch(0.91 0.008 60)",
        }}
        aria-label={`Math: ${math}`}
      />
    )
  }

  return (
    <span
      ref={ref as React.RefObject<HTMLSpanElement>}
      className={className}
      style={{ color: "oklch(0.91 0.008 60)" }}
      aria-label={`Math: ${math}`}
    />
  )
}

/**
 * Renders a string that may contain inline LaTeX delimited by $...$
 * and display LaTeX delimited by $$...$$.
 * Splits on delimiters and renders each segment appropriately.
 */
export function MathText({ text, className }: { text: string; className?: string }) {
  // Split on $$...$$ first (display), then $...$ (inline)
  const parts: Array<{ type: "text" | "inline" | "display"; content: string }> = []
  const displayRe = /\$\$([\s\S]+?)\$\$/g
  const inlineRe = /\$([^$\n]+?)\$/g

  let lastIndex = 0
  let match: RegExpExecArray | null

  // Display blocks first
  const displayParts: string[] = []
  let d: RegExpExecArray | null
  // eslint-disable-next-line no-cond-assign
  while ((d = displayRe.exec(text)) !== null) {
    displayParts.push(text.slice(lastIndex, d.index))
    displayParts.push(`\x00display\x01${d[1]}\x02`)
    lastIndex = d.index + d[0].length
  }
  displayParts.push(text.slice(lastIndex))
  const merged = displayParts.join("")

  // Now split inline
  lastIndex = 0
  const inlineParts = merged.split(/(\x00display\x01[\s\S]*?\x02|\$[^$\n]+?\$)/)

  for (const seg of inlineParts) {
    if (seg.startsWith("\x00display\x01")) {
      parts.push({ type: "display", content: seg.slice(9, seg.length - 1) })
    } else if (/^\$[^$]/.test(seg) && seg.endsWith("$")) {
      parts.push({ type: "inline", content: seg.slice(1, -1) })
    } else if (seg) {
      parts.push({ type: "text", content: seg })
    }
  }

  return (
    <span className={className}>
      {parts.map((p, i) =>
        p.type === "text"    ? <span key={i}>{p.content}</span> :
        p.type === "inline"  ? <MathBlock key={i} math={p.content} display={false} /> :
                               <MathBlock key={i} math={p.content} display={true} />
      )}
    </span>
  )
}
