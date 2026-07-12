'use client'

import { cn } from '@/lib/utils'
import { User, Bot, Wallet, ArrowRight, Shield, Key } from 'lucide-react'

interface AgentIdentityVisualProps {
  className?: string
}

export function AgentIdentityVisual({ className }: AgentIdentityVisualProps) {
  return (
    <div className={cn('relative', className)}>
      {/* Flow diagram: ENS → Agent → Wallet */}
      <div className="flex flex-col lg:flex-row items-center justify-center gap-4 lg:gap-8">
        {/* ENS Identity */}
        <div className="flex flex-col items-center p-6 rounded-xl border border-primary/30 bg-primary/5 min-w-[180px]">
          <div className="size-12 rounded-full bg-primary/20 flex items-center justify-center mb-3">
            <User className="size-6 text-primary" />
          </div>
          <h4 className="font-semibold text-foreground mb-1">ENS Identity</h4>
          <p className="text-xs text-muted-foreground text-center">Human-readable name</p>
          <code className="mt-2 text-xs font-mono text-primary bg-primary/10 px-2 py-1 rounded">
            chimiaclaw.eth
          </code>
        </div>

        {/* Arrow */}
        <div className="flex items-center justify-center">
          <div className="hidden lg:flex items-center gap-2">
            <div className="h-px w-8 bg-gradient-to-r from-primary/50 to-primary" />
            <ArrowRight className="size-5 text-primary" />
          </div>
          <div className="lg:hidden flex flex-col items-center gap-2">
            <ArrowRight className="size-5 text-primary rotate-90" />
          </div>
        </div>

        {/* Agent */}
        <div className="flex flex-col items-center p-6 rounded-xl border border-emerald-500/30 bg-emerald-500/5 min-w-[180px]">
          <div className="size-12 rounded-full bg-emerald-500/20 flex items-center justify-center mb-3">
            <Bot className="size-6 text-emerald-400" />
          </div>
          <h4 className="font-semibold text-foreground mb-1">Autonomous Agent</h4>
          <p className="text-xs text-muted-foreground text-center">Signed computation</p>
          <div className="mt-2 flex items-center gap-1 text-xs">
            <Shield className="size-3 text-emerald-400" />
            <span className="font-mono text-emerald-400">ed25519 pubkey</span>
          </div>
        </div>

        {/* Arrow */}
        <div className="flex items-center justify-center">
          <div className="hidden lg:flex items-center gap-2">
            <div className="h-px w-8 bg-gradient-to-r from-emerald-500/50 to-amber-500" />
            <ArrowRight className="size-5 text-amber-500" />
          </div>
          <div className="lg:hidden flex flex-col items-center gap-2">
            <ArrowRight className="size-5 text-amber-500 rotate-90" />
          </div>
        </div>

        {/* Wallet */}
        <div className="flex flex-col items-center p-6 rounded-xl border border-amber-500/30 bg-amber-500/5 min-w-[180px]">
          <div className="size-12 rounded-full bg-amber-500/20 flex items-center justify-center mb-3">
            <Wallet className="size-6 text-amber-400" />
          </div>
          <h4 className="font-semibold text-foreground mb-1">Settlement Wallet</h4>
          <p className="text-xs text-muted-foreground text-center">USDC payments</p>
          <div className="mt-2 flex items-center gap-1 text-xs">
            <Key className="size-3 text-amber-400" />
            <span className="font-mono text-amber-400">0x7a23...8f91</span>
          </div>
        </div>
      </div>

      {/* Connection lines (decorative) */}
      <div className="absolute inset-0 -z-10 overflow-hidden pointer-events-none">
        <div className="absolute top-1/2 left-0 right-0 h-px bg-gradient-to-r from-transparent via-border/50 to-transparent" />
      </div>
    </div>
  )
}
