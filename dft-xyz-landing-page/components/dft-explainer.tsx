'use client'

import { useState } from 'react'
import { cn } from '@/lib/utils'
import { ChevronDown, Atom, Info } from 'lucide-react'

interface DFTExplainerProps {
  className?: string
}

export function DFTExplainer({ className }: DFTExplainerProps) {
  const [isOpen, setIsOpen] = useState(false)

  return (
    <div className={cn('rounded-lg border border-border/50 bg-card/30', className)}>
      <button
        onClick={() => setIsOpen(!isOpen)}
        className="flex items-center justify-between w-full p-4 text-left hover:bg-muted/50 transition-colors rounded-lg"
      >
        <div className="flex items-center gap-3">
          <div className="size-8 rounded-full bg-primary/10 flex items-center justify-center">
            <Atom className="size-4 text-primary" />
          </div>
          <div>
            <h4 className="font-medium text-foreground">What is DFT?</h4>
            <p className="text-xs text-muted-foreground">
              Density Functional Theory explained
            </p>
          </div>
        </div>
        <ChevronDown
          className={cn(
            'size-5 text-muted-foreground transition-transform',
            isOpen && 'rotate-180'
          )}
        />
      </button>

      {isOpen && (
        <div className="px-4 pb-4 space-y-4 border-t border-border/50 pt-4">
          <div>
            <h5 className="text-sm font-medium text-foreground mb-2">
              Quantum Chemistry for Molecular Properties
            </h5>
            <p className="text-sm text-muted-foreground leading-relaxed">
              Density Functional Theory (DFT) is a quantum mechanical method for computing
              electronic structure of molecules. It predicts properties like energy, geometry,
              and reactivity by solving the Schr&ouml;dinger equation using electron density
              rather than wavefunctions.
            </p>
          </div>

          <div className="grid sm:grid-cols-2 gap-3">
            <div className="rounded-lg border border-border/50 bg-muted/30 p-3">
              <h6 className="text-xs font-medium text-primary mb-1">What DFT Computes</h6>
              <ul className="text-xs text-muted-foreground space-y-1">
                <li>Total energy (Hartrees)</li>
                <li>HOMO-LUMO gap (eV)</li>
                <li>Dipole moment</li>
                <li>Optimized geometry</li>
                <li>Vibrational frequencies</li>
              </ul>
            </div>
            <div className="rounded-lg border border-border/50 bg-muted/30 p-3">
              <h6 className="text-xs font-medium text-primary mb-1">Methods We Use</h6>
              <ul className="text-xs text-muted-foreground space-y-1">
                <li>B3LYP — workhorse hybrid</li>
                <li>PBE0 — accurate thermochemistry</li>
                <li>M06-2X — main-group chemistry</li>
                <li>Skala 1.1 — neural XC functional</li>
              </ul>
            </div>
          </div>

          <div className="flex items-start gap-2 rounded-lg bg-primary/5 border border-primary/20 p-3">
            <Info className="size-4 text-primary mt-0.5 shrink-0" />
            <p className="text-xs text-muted-foreground">
              <strong className="text-foreground">Why it matters for agents:</strong> DFT results
              are computationally expensive (minutes to hours per molecule). Signed, verifiable
              results prevent re-computation and enable trustless commerce.
            </p>
          </div>
        </div>
      )}
    </div>
  )
}
