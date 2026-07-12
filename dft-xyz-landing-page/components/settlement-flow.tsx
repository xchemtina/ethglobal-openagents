'use client'

import { cn } from '@/lib/utils'
import { ArrowRight, Lock, Coins, Check, Clock } from 'lucide-react'

interface SettlementFlowProps {
  className?: string
}

export function SettlementFlow({ className }: SettlementFlowProps) {
  const steps = [
    {
      icon: Coins,
      label: 'Quote',
      description: 'Live Uniswap V4 price',
      color: 'text-primary',
      bgColor: 'bg-primary/10',
      borderColor: 'border-primary/30',
    },
    {
      icon: Lock,
      label: 'Escrow',
      description: 'Funds held in contract',
      color: 'text-amber-400',
      bgColor: 'bg-amber-500/10',
      borderColor: 'border-amber-500/30',
    },
    {
      icon: Clock,
      label: 'Compute',
      description: 'DFT calculation runs',
      color: 'text-blue-400',
      bgColor: 'bg-blue-500/10',
      borderColor: 'border-blue-500/30',
    },
    {
      icon: Check,
      label: 'Release',
      description: 'Operator receives payment',
      color: 'text-emerald-400',
      bgColor: 'bg-emerald-500/10',
      borderColor: 'border-emerald-500/30',
    },
  ]

  return (
    <div className={cn('', className)}>
      {/* Desktop flow */}
      <div className="hidden md:flex items-center justify-center gap-2">
        {steps.map((step, index) => (
          <div key={step.label} className="flex items-center">
            <div
              className={cn(
                'flex flex-col items-center p-4 rounded-lg border',
                step.bgColor,
                step.borderColor
              )}
            >
              <step.icon className={cn('size-6 mb-2', step.color)} />
              <span className="font-medium text-sm text-foreground">{step.label}</span>
              <span className="text-xs text-muted-foreground text-center mt-1">
                {step.description}
              </span>
            </div>
            {index < steps.length - 1 && (
              <ArrowRight className="size-5 text-muted-foreground mx-2 shrink-0" />
            )}
          </div>
        ))}
      </div>

      {/* Mobile flow */}
      <div className="md:hidden space-y-3">
        {steps.map((step, index) => (
          <div key={step.label} className="flex items-center gap-3">
            <div
              className={cn(
                'flex items-center justify-center size-10 rounded-lg border shrink-0',
                step.bgColor,
                step.borderColor
              )}
            >
              <step.icon className={cn('size-5', step.color)} />
            </div>
            <div className="flex-1 min-w-0">
              <span className="font-medium text-sm text-foreground">{step.label}</span>
              <p className="text-xs text-muted-foreground">{step.description}</p>
            </div>
            {index < steps.length - 1 && (
              <ArrowRight className="size-4 text-muted-foreground shrink-0" />
            )}
          </div>
        ))}
      </div>
    </div>
  )
}
