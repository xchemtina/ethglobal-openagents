'use client'

import { useState } from 'react'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Badge } from '@/components/ui/badge'
import { cn } from '@/lib/utils'
import type { DFTResult } from '@/lib/types'
import { SAMPLE_ARTIFACTS, CHALLENGE_BOND_AMOUNT, DISCREPANCY_THRESHOLD } from '@/lib/challenge-data'
import { Search, AlertTriangle, Shield, Zap, ArrowRight } from 'lucide-react'

interface ChallengeModalProps {
  trigger?: React.ReactNode
  className?: string
}

export function ChallengeModal({ trigger, className }: ChallengeModalProps) {
  const [open, setOpen] = useState(false)
  const [searchQuery, setSearchQuery] = useState('')
  const [selectedArtifact, setSelectedArtifact] = useState<DFTResult | null>(null)
  const [step, setStep] = useState<'search' | 'confirm' | 'preview'>('search')

  const filteredArtifacts = SAMPLE_ARTIFACTS.filter(
    (a) =>
      a.id.toLowerCase().includes(searchQuery.toLowerCase()) ||
      a.requestId.toLowerCase().includes(searchQuery.toLowerCase())
  )

  const handleSelectArtifact = (artifact: DFTResult) => {
    setSelectedArtifact(artifact)
    setStep('confirm')
  }

  const handlePreviewChallenge = () => {
    setStep('preview')
  }

  const handleClose = () => {
    setOpen(false)
    setStep('search')
    setSelectedArtifact(null)
    setSearchQuery('')
  }

  return (
    <Dialog open={open} onOpenChange={(nextOpen) => nextOpen ? setOpen(true) : handleClose()}>
      <DialogTrigger asChild>
        {trigger || (
          <Button variant="outline" className={cn('gap-2', className)}>
            <Shield className="size-4" />
            Preview Challenge
          </Button>
        )}
      </DialogTrigger>
      <DialogContent className="sm:max-w-lg">
        {step === 'search' && (
          <>
            <DialogHeader>
              <DialogTitle className="flex items-center gap-2">
                <Shield className="size-5 text-primary" />
                Challenge Workflow Preview
              </DialogTitle>
              <DialogDescription>
                Explore a proposed review workflow using sample data. This local
                preview does not submit a challenge, transfer funds, or run a calculation.
              </DialogDescription>
            </DialogHeader>

            <div className="space-y-4">
              <div className="relative">
                <Search className="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
                <Input
                  placeholder="Search by artifact ID or request ID..."
                  value={searchQuery}
                  onChange={(e) => setSearchQuery(e.target.value)}
                  className="pl-9 font-mono text-sm"
                />
              </div>

              <div className="max-h-64 space-y-2 overflow-y-auto">
                {filteredArtifacts.length === 0 ? (
                  <p className="py-8 text-center text-sm text-muted-foreground">
                    No artifacts found matching your search.
                  </p>
                ) : (
                  filteredArtifacts.map((artifact) => (
                    <button
                      key={artifact.id}
                      onClick={() => handleSelectArtifact(artifact)}
                      className="w-full rounded-lg border border-border/50 bg-card/50 p-3 text-left transition-colors hover:border-primary/50 hover:bg-primary/5"
                    >
                      <div className="flex items-center justify-between">
                        <span className="font-mono text-sm text-primary">
                          {artifact.id}
                        </span>
                        {artifact.anchorTxHash && (
                          <Badge variant="outline" className="text-xs">
                            Sample anchor
                          </Badge>
                        )}
                      </div>
                      <div className="mt-1 flex items-center gap-4 text-xs text-muted-foreground">
                        <span>Energy: {artifact.energy.toFixed(6)} Ha</span>
                        <span>HOMO: {artifact.homo} eV</span>
                      </div>
                    </button>
                  ))
                )}
              </div>
            </div>

            <DialogFooter>
              <Button variant="outline" onClick={handleClose}>
                Cancel
              </Button>
            </DialogFooter>
          </>
        )}

        {step === 'confirm' && selectedArtifact && (
          <>
            <DialogHeader>
              <DialogTitle className="flex items-center gap-2">
                <AlertTriangle className="size-5 text-amber-500" />
                Review Sample Challenge
              </DialogTitle>
              <DialogDescription>
                These illustrative terms are for this preview only. No wallet is needed.
              </DialogDescription>
            </DialogHeader>

            <div className="space-y-4">
              <div className="rounded-lg border border-border/50 bg-card/50 p-4">
                <h4 className="mb-2 text-sm font-medium">Sample Result</h4>
                <dl className="space-y-1 text-sm">
                  <div className="flex justify-between">
                    <dt className="text-muted-foreground">Artifact ID</dt>
                    <dd className="font-mono text-primary">{selectedArtifact.id}</dd>
                  </div>
                  <div className="flex justify-between">
                    <dt className="text-muted-foreground">Energy</dt>
                    <dd className="font-mono">{selectedArtifact.energy.toFixed(6)} Ha</dd>
                  </div>
                  <div className="flex justify-between">
                    <dt className="text-muted-foreground">Operator</dt>
                    <dd className="font-mono text-xs">{selectedArtifact.operatorAddress}</dd>
                  </div>
                </dl>
              </div>

              <div className="rounded-lg border border-amber-500/30 bg-amber-500/10 p-4">
                <h4 className="mb-2 flex items-center gap-2 text-sm font-medium text-amber-400">
                  <Zap className="size-4" />
                  Illustrative Terms
                </h4>
                <ul className="space-y-1 text-sm text-amber-200/80">
                  <li>Example bond: <strong>{CHALLENGE_BOND_AMOUNT} USDC</strong> (not collected)</li>
                  <li>Example discrepancy threshold: <strong>{DISCREPANCY_THRESHOLD} Ha</strong></li>
                  <li>Re-computation is not connected to this preview.</li>
                </ul>
              </div>

            </div>

            <DialogFooter className="gap-2 sm:gap-0">
              <Button variant="outline" onClick={() => setStep('search')}>
                Back
              </Button>
              <Button
                onClick={handlePreviewChallenge}
                className="gap-2"
              >
                View Local Preview
                <ArrowRight className="size-4" />
              </Button>
            </DialogFooter>
          </>
        )}

        {step === 'preview' && selectedArtifact && (
          <>
            <DialogHeader>
              <DialogTitle className="flex items-center gap-2">
                <Shield className="size-5 text-emerald-500" />
                Local Preview Ready
              </DialogTitle>
              <DialogDescription>
                Nothing has been submitted. This example exists only in this browser view.
              </DialogDescription>
            </DialogHeader>

            <div className="space-y-4">
              <div className="rounded-lg border border-emerald-500/30 bg-emerald-500/10 p-4 text-center">
                <p className="text-sm text-emerald-200">
                  No payment, calculation, or notification has been scheduled.
                  Close this dialog to discard the preview.
                </p>
                <p className="mt-2 font-mono text-xs text-muted-foreground">
                  Preview reference: preview-{selectedArtifact.id}
                </p>
              </div>

            </div>

            <DialogFooter>
              <Button onClick={handleClose}>Done</Button>
            </DialogFooter>
          </>
        )}
      </DialogContent>
    </Dialog>
  )
}
