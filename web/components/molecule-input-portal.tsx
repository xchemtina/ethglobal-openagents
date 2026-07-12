'use client'

import { useRef, useState } from 'react'
import { Camera, Check, CircleDot, Eraser, ImagePlus, MousePointer2, PencilLine, RotateCcw, Sparkles } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'
import { MoladtTry } from '@/components/moladt-try'

const tools = [
  { label: 'Select', icon: MousePointer2 },
  { label: 'Bond', icon: PencilLine },
  { label: 'Atom', icon: CircleDot },
  { label: 'Erase', icon: Eraser },
]

export function MoleculeInputPortal() {
  const inputRef = useRef<HTMLInputElement>(null)
  const [mode, setMode] = useState<'draw' | 'image'>('draw')
  const [activeTool, setActiveTool] = useState('Bond')
  const [imageName, setImageName] = useState<string | null>(null)

  return (
    <div className="overflow-hidden rounded-xl border border-border bg-card shadow-2xl shadow-primary/5">
      <div className="flex flex-col gap-4 border-b border-border px-4 py-4 sm:flex-row sm:items-center sm:justify-between sm:px-6">
        <div>
          <div className="flex items-center gap-2">
            <span className="font-mono text-xs text-primary">01 / INPUT</span>
            <span className="rounded-full border border-primary/20 bg-primary/10 px-2 py-0.5 text-xs text-primary">Small molecules</span>
          </div>
          <h3 className="mt-1 text-lg font-semibold text-foreground">Give the agent a molecular structure</h3>
        </div>
        <div className="flex rounded-lg border border-border bg-background p-1" aria-label="Input method">
          <button
            type="button"
            onClick={() => setMode('draw')}
            className={cn('flex items-center gap-2 rounded-md px-3 py-2 text-sm transition-colors', mode === 'draw' ? 'bg-primary text-primary-foreground' : 'text-muted-foreground hover:text-foreground')}
          >
            <PencilLine className="size-4" /> Draw
          </button>
          <button
            type="button"
            onClick={() => setMode('image')}
            className={cn('flex items-center gap-2 rounded-md px-3 py-2 text-sm transition-colors', mode === 'image' ? 'bg-primary text-primary-foreground' : 'text-muted-foreground hover:text-foreground')}
          >
            <Camera className="size-4" /> Picture
          </button>
        </div>
      </div>

      <div className="grid lg:grid-cols-[1fr_280px]">
        <div className="min-h-96 border-b border-border lg:border-b-0 lg:border-r">
          {mode === 'draw' ? (
            <div className="flex min-h-96 flex-col">
              <div className="flex flex-wrap items-center gap-2 border-b border-border bg-muted/20 p-3">
                {tools.map(({ label, icon: Icon }) => (
                  <button
                    key={label}
                    type="button"
                    onClick={() => setActiveTool(label)}
                    aria-pressed={activeTool === label}
                    className={cn('flex items-center gap-2 rounded-md px-3 py-2 text-xs transition-colors', activeTool === label ? 'bg-secondary text-foreground' : 'text-muted-foreground hover:bg-secondary/60 hover:text-foreground')}
                  >
                    <Icon className="size-4" /> {label}
                  </button>
                ))}
                <button type="button" className="ml-auto rounded-md p-2 text-muted-foreground hover:bg-secondary hover:text-foreground" aria-label="Reset drawing">
                  <RotateCcw className="size-4" />
                </button>
              </div>
              <div className="relative flex flex-1 items-center justify-center overflow-hidden bg-background/50 p-8">
                <div className="absolute inset-0 opacity-30 grid-pattern" />
                <div className="relative h-52 w-80" aria-label="Example molecular drawing of caffeine">
                  <div className="absolute left-10 top-20 h-0.5 w-20 -rotate-30 bg-foreground" />
                  <div className="absolute left-24 top-12 h-0.5 w-20 rotate-30 bg-foreground" />
                  <div className="absolute left-24 top-32 h-0.5 w-20 -rotate-30 bg-foreground" />
                  <div className="absolute left-40 top-20 h-0.5 w-20 rotate-30 bg-foreground" />
                  <div className="absolute left-40 top-20 h-0.5 w-20 -rotate-30 bg-foreground" />
                  <div className="absolute left-24 top-12 h-20 w-0.5 bg-foreground" />
                  <div className="absolute left-56 top-12 h-20 w-0.5 bg-foreground" />
                  <Atom className="left-4 top-20" label="O" accent />
                  <Atom className="left-20 top-3" label="N" />
                  <Atom className="left-20 top-32" label="N" />
                  <Atom className="left-44 top-4" label="O" accent />
                  <Atom className="left-52 top-32" label="N" />
                  <Atom className="left-44 top-20" label="N" />
                </div>
                <div className="absolute bottom-4 left-4 flex items-center gap-2 text-xs text-muted-foreground">
                  <span className="size-2 rounded-full bg-primary" /> Example canvas · click a tool to begin
                </div>
              </div>
            </div>
          ) : (
            <button
              type="button"
              onClick={() => inputRef.current?.click()}
              className="flex min-h-96 w-full flex-col items-center justify-center gap-4 bg-background/50 p-8 text-center transition-colors hover:bg-muted/20"
            >
              <span className="flex size-16 items-center justify-center rounded-full border border-dashed border-primary/50 bg-primary/10 text-primary">
                {imageName ? <Check className="size-7" /> : <ImagePlus className="size-7" />}
              </span>
              <span className="text-lg font-medium text-foreground">{imageName ?? 'Upload a clear structure image'}</span>
              <span className="max-w-sm text-sm leading-relaxed text-muted-foreground">PNG, JPG, or a photo of a hand-drawn structure. The agent will extract and ask you to confirm it before quoting.</span>
              <input
                ref={inputRef}
                type="file"
                accept="image/png,image/jpeg,image/webp"
                className="sr-only"
                onChange={(event) => setImageName(event.target.files?.[0]?.name ?? null)}
              />
            </button>
          )}
        </div>

        <aside className="flex flex-col gap-6 bg-muted/10 p-5 sm:p-6">
          <div>
            <span className="font-mono text-xs text-muted-foreground">DETECTED STRUCTURE</span>
            <p className="mt-3 text-xl font-semibold text-foreground">Caffeine</p>
            <p className="font-mono text-sm text-muted-foreground">C₈H₁₀N₄O₂ · 24 atoms</p>
          </div>
          <div className="flex flex-col gap-3 text-sm">
            <CheckLine label="Valid closed-shell structure" />
            <CheckLine label="Within 50-atom limit" />
            <CheckLine label="Estimated neutral charge" />
          </div>
          <div className="mt-auto rounded-lg border border-accent/30 bg-accent/5 p-4">
            <div className="flex items-center justify-between gap-4">
              <span className="text-xs text-muted-foreground">EST. QUOTE</span>
              <span className="font-mono text-lg font-semibold text-accent">4.80 USDC</span>
            </div>
            <p className="mt-2 text-xs leading-relaxed text-muted-foreground">Final quote follows structure confirmation. Wallet approval is never requested before review.</p>
          </div>
          <Button className="w-full gap-2" disabled>
            <Sparkles className="size-4" /> Configure calculation
          </Button>
          <p className="text-center text-xs text-muted-foreground">
            Full draw-to-DFT is UI scaffolding. Real cashier path is SMILES → MolADT below.
          </p>
        </aside>
      </div>
      <div className="border-t border-border p-4 sm:p-6">
        <MoladtTry />
      </div>
    </div>
  )
}

function Atom({ className, label, accent = false }: { className: string; label: string; accent?: boolean }) {
  return (
    <span className={cn('absolute z-10 flex size-9 items-center justify-center rounded-full border bg-card font-mono text-sm font-bold shadow-lg', accent ? 'border-accent/50 text-accent' : 'border-primary/50 text-primary', className)}>
      {label}
    </span>
  )
}

function CheckLine({ label }: { label: string }) {
  return (
    <div className="flex items-center gap-2 text-muted-foreground">
      <span className="flex size-5 items-center justify-center rounded-full bg-primary/10 text-primary"><Check className="size-3" /></span>
      {label}
    </div>
  )
}
