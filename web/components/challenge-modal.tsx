'use client'

import { useState } from 'react'
import { useAccount } from 'wagmi'
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
  const { isConnected, address } = useAccount()
  const [open, setOpen] = useState(false)
  const [searchQuery, setSearchQuery] = useState('')
  const [selectedArtifact, setSelectedArtifact] = useState<DFTResult | null>(null)
  const [step, setStep] = useState<'search' | 'confirm' | 'submitted'>('search')

  const filteredArtifacts = SAMPLE_ARTIFACTS.filter(
    (a) =>
      a.id.toLowerCase().includes(searchQuery.toLowerCase()) ||
      a.requestId.toLowerCase().includes(searchQuery.toLowerCase())
  )

  const handleSelectArtifact = (artifact: DFTResult) => {
    setSelectedArtifact(artifact)
    setStep('confirm')
  }

  const handleSubmitChallenge = () => {
    // In production, this would call the smart contract
    setStep('submitted')
  }

  const handleClose = () => {
    setOpen(false)
    // Reset state after animation
    setTimeout(() => {
      setStep('search')
      setSelectedArtifact(null)
      setSearchQuery('')
    }, 200)
  }

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogTrigger asChild>
        {trigger || (
          <Button variant="outline" className={cn('gap-2', className)}>
            <Shield className="size-4" />
            Challenge Calculation
          </Button>
        )}
      </DialogTrigger>
      <DialogContent className="sm:max-w-lg">
        {step === 'search' && (
          <>
            <DialogHeader>
              <DialogTitle className="flex items-center gap-2">
                <Shield className="size-5 text-primary" />
                Challenge a DFT Result
              </DialogTitle>
              <DialogDescription>
                Search for an artifact ID to challenge. You&apos;ll stake {CHALLENGE_BOND_AMOUNT} USDC
                as a bond, which is returned if the challenge reveals a discrepancy.
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
                            Anchored
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
                Confirm Challenge
              </DialogTitle>
              <DialogDescription>
                Review the details before submitting your challenge.
              </DialogDescription>
            </DialogHeader>

            <div className="space-y-4">
              <div className="rounded-lg border border-border/50 bg-card/50 p-4">
                <h4 className="mb-2 text-sm font-medium">Original Result</h4>
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
                  Challenge Terms
                </h4>
                <ul className="space-y-1 text-sm text-amber-200/80">
                  <li>Bond amount: <strong>{CHALLENGE_BOND_AMOUNT} USDC</strong></li>
                  <li>Discrepancy threshold: <strong>{DISCREPANCY_THRESHOLD} Ha</strong></li>
                  <li>Result will be re-computed by a different operator</li>
                </ul>
              </div>

              {!isConnected && (
                <div className="rounded-lg border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive">
                  Please connect your wallet to submit a challenge.
                </div>
              )}
            </div>

            <DialogFooter className="gap-2 sm:gap-0">
              <Button variant="outline" onClick={() => setStep('search')}>
                Back
              </Button>
              <Button
                onClick={handleSubmitChallenge}
                disabled={!isConnected}
                className="gap-2"
              >
                Stake {CHALLENGE_BOND_AMOUNT} USDC
                <ArrowRight className="size-4" />
              </Button>
            </DialogFooter>
          </>
        )}

        {step === 'submitted' && (
          <>
            <DialogHeader>
              <DialogTitle className="flex items-center gap-2">
                <Shield className="size-5 text-emerald-500" />
                Challenge Submitted
              </DialogTitle>
              <DialogDescription>
                Your challenge has been submitted and is being processed.
              </DialogDescription>
            </DialogHeader>

            <div className="space-y-4">
              <div className="rounded-lg border border-emerald-500/30 bg-emerald-500/10 p-4 text-center">
                <p className="text-sm text-emerald-200">
                  A different operator will re-compute the DFT calculation.
                  You&apos;ll be notified when results are ready.
                </p>
                <p className="mt-2 font-mono text-xs text-muted-foreground">
                  Challenge ID: chal-{Date.now().toString(36)}
                </p>
              </div>

              <div className="text-center text-sm text-muted-foreground">
                Connected as: <span className="font-mono text-primary">{address?.slice(0, 10)}...</span>
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
