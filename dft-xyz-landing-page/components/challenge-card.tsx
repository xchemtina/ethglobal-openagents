import { Badge } from '@/components/ui/badge'
import { cn } from '@/lib/utils'
import type { Challenge } from '@/lib/types'
import { getChallengeStatusLabel, getChallengeStatusColor } from '@/lib/challenge-data'
import { Clock, ArrowRight } from 'lucide-react'

interface ChallengeCardProps {
  challenge: Challenge
  className?: string
}

export function ChallengeCard({ challenge, className }: ChallengeCardProps) {
  const statusLabel = getChallengeStatusLabel(challenge.status)
  const statusColor = getChallengeStatusColor(challenge.status)

  const timeSince = () => {
    const diff = Date.now() - new Date(challenge.createdAt).getTime()
    const hours = Math.floor(diff / 3600000)
    if (hours < 1) return 'Just now'
    if (hours === 1) return '1 hour ago'
    return `${hours} hours ago`
  }

  return (
    <div
      className={cn(
        'rounded-lg border border-border/50 bg-card/50 p-4 transition-colors hover:border-primary/30',
        className
      )}
    >
      <div className="flex items-start justify-between">
        <div>
          <h4 className="font-mono text-sm text-primary">{challenge.id}</h4>
          <p className="mt-0.5 text-xs text-muted-foreground">
            Challenging: {challenge.originalArtifactId}
          </p>
        </div>
        <Badge className={cn('text-xs', statusColor)}>{statusLabel}</Badge>
      </div>

      <div className="mt-3 grid grid-cols-2 gap-3 text-sm">
        <div>
          <span className="text-xs text-muted-foreground">Original Energy</span>
          <p className="font-mono">{challenge.originalResult.energy.toFixed(6)} Ha</p>
        </div>
        {challenge.challengeResult && (
          <div>
            <span className="text-xs text-muted-foreground">Challenge Energy</span>
            <p className="font-mono">{challenge.challengeResult.energy.toFixed(6)} Ha</p>
          </div>
        )}
      </div>

      {challenge.status === 'disputed' && challenge.challengeResult && (
        <div className="mt-3 flex items-center gap-2 rounded border border-red-500/30 bg-red-500/10 px-3 py-2 text-xs">
          <span className="text-red-400">Discrepancy detected:</span>
          <span className="font-mono">
            {Math.abs(challenge.originalResult.energy - challenge.challengeResult.energy).toFixed(6)} Ha
          </span>
          <ArrowRight className="ml-auto size-3 text-red-400" />
        </div>
      )}

      <div className="mt-3 flex items-center justify-between border-t border-border/50 pt-3 text-xs text-muted-foreground">
        <div className="flex items-center gap-1">
          <Clock className="size-3" />
          {timeSince()}
        </div>
        <div>
          Bond: <span className="font-mono text-amber-400">{challenge.bondAmount} USDC</span>
        </div>
      </div>
    </div>
  )
}
