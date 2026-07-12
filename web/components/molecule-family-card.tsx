interface MoleculeFamilyCardProps {
  title: string
  formula?: string
  description: string
  isCustom?: boolean
}

export function MoleculeFamilyCard({ title, formula, description, isCustom = false }: MoleculeFamilyCardProps) {
  return (
    <div className={`relative bg-card border rounded-lg p-6 overflow-hidden transition-all duration-300 hover:border-primary/40 ${isCustom ? "border-accent/30" : "border-border"}`}>
      {/* Background pattern suggesting molecular structure */}
      <div className="absolute inset-0 opacity-5">
        <svg className="w-full h-full" viewBox="0 0 100 100">
          <defs>
            <pattern id={`mol-${title}`} x="0" y="0" width="20" height="20" patternUnits="userSpaceOnUse">
              <circle cx="10" cy="10" r="2" fill="currentColor" />
              <line x1="10" y1="10" x2="20" y2="20" stroke="currentColor" strokeWidth="0.5" />
            </pattern>
          </defs>
          <rect width="100" height="100" fill={`url(#mol-${title})`} />
        </svg>
      </div>

      <div className="relative">
        <h3 className={`font-semibold mb-1 ${isCustom ? "text-accent" : "text-foreground"}`}>{title}</h3>
        {formula && (
          <p className="text-primary font-mono text-sm mb-2">{formula}</p>
        )}
        <p className="text-muted-foreground text-sm leading-relaxed">{description}</p>
      </div>
    </div>
  )
}
