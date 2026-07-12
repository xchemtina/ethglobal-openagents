import { Lock, ArrowRight } from "lucide-react"

interface PipelineCardProps {
  step: number
  title: string
  description: string
  isLast?: boolean
}

export function PipelineCard({ step, title, description, isLast = false }: PipelineCardProps) {
  return (
    <div className="relative flex-shrink-0 w-72 group">
      <div className="relative bg-card border border-border rounded-lg p-5 h-full transition-all duration-300 hover:border-primary/50 hover:bg-card/80">
        {/* Step number */}
        <div className="absolute -top-3 left-4 bg-primary text-primary-foreground text-xs font-mono px-2 py-0.5 rounded">
          {String(step).padStart(2, "0")}
        </div>

        {/* Lock icon */}
        <div className="absolute top-3 right-3">
          <Lock className="w-3.5 h-3.5 text-primary/60" />
        </div>

        <h3 className="text-foreground font-semibold mt-2 mb-2">{title}</h3>
        <p className="text-muted-foreground text-sm leading-relaxed">{description}</p>
      </div>

      {/* Arrow connector */}
      {!isLast && (
        <div className="hidden lg:flex absolute top-1/2 -right-4 -translate-y-1/2 z-10">
          <ArrowRight className="w-6 h-6 text-primary/40" />
        </div>
      )}
    </div>
  )
}
