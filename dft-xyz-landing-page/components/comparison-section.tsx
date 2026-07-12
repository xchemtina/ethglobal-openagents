import { Check, X } from "lucide-react"

interface ComparisonItemProps {
  text: string
  included: boolean
}

function ComparisonItem({ text, included }: ComparisonItemProps) {
  return (
    <li className="flex items-start gap-3">
      {included ? (
        <Check className="w-5 h-5 text-primary shrink-0 mt-0.5" />
      ) : (
        <X className="w-5 h-5 text-muted-foreground/50 shrink-0 mt-0.5" />
      )}
      <span className={included ? "text-foreground" : "text-muted-foreground"}>{text}</span>
    </li>
  )
}

export function ComparisonSection() {
  const othersFeatures = [
    "LLM-driven DFT automation",
    "Log files for tracking",
    "Local results storage",
    "No settlement mechanism",
    "No cryptographic signatures",
    "No decentralized identity",
  ]

  const dftxyzFeatures = [
    "Signed artifact DAG (Ed25519 + Blake3)",
    "Live Uniswap settlement quotes",
    "ENS agent identity",
    "0G decentralized anchoring",
    "MCP server for agent discovery",
    "Skala 1.1 neural functional",
  ]

  return (
    <div className="grid md:grid-cols-2 gap-6">
      {/* Others column */}
      <div className="bg-card border border-border rounded-lg p-6">
        <h3 className="text-lg font-semibold text-muted-foreground mb-4">What others do</h3>
        <ul className="space-y-3">
          {othersFeatures.map((feature, i) => (
            <ComparisonItem key={i} text={feature} included={false} />
          ))}
        </ul>
      </div>

      {/* DFT.xyz column */}
      <div className="bg-card border border-primary/30 rounded-lg p-6 relative overflow-hidden">
        <div className="absolute inset-0 bg-gradient-to-br from-primary/5 to-transparent" />
        <h3 className="text-lg font-semibold text-primary mb-4 relative">What DFT.xyz adds</h3>
        <ul className="space-y-3 relative">
          {dftxyzFeatures.map((feature, i) => (
            <ComparisonItem key={i} text={feature} included={true} />
          ))}
        </ul>
      </div>
    </div>
  )
}

export function DifferentiatorCallout() {
  return (
    <div className="mt-8 bg-muted/50 border border-border rounded-lg p-6">
      <p className="text-muted-foreground leading-relaxed">
        <span className="text-foreground font-semibold">AutoDFT</span> (NTU Singapore) achieves{" "}
        <span className="text-primary font-mono">94.1%</span> task success with 7 LLM agents on VASP.
        We achieve <span className="text-foreground">cryptographic provenance</span>,{" "}
        <span className="text-accent">economic settlement</span>, and{" "}
        <span className="text-foreground">decentralized identity</span> — the layers that turn
        computation into a trustless service any agent can consume and pay for.
      </p>
    </div>
  )
}
