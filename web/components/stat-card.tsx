interface StatCardProps {
  value: string
  label: string
  sublabel?: string
  isPrice?: boolean
}

export function StatCard({ value, label, sublabel, isPrice = false }: StatCardProps) {
  return (
    <div className="bg-card border border-border rounded-lg p-6 text-center transition-all duration-300 hover:border-primary/30">
      <div className={`text-4xl font-bold mb-2 font-mono ${isPrice ? "text-accent" : "text-primary"}`}>
        {value}
      </div>
      <div className="text-foreground font-medium">{label}</div>
      {sublabel && <div className="text-muted-foreground text-sm mt-1">{sublabel}</div>}
    </div>
  )
}
