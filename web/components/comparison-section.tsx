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
  const plannedFeatures = [
    "Verified USDC payment and settlement",
    "Automatic human-payment fulfillment",
    "Public live DFT dispatch",
    "Wallet escrow and dispute execution",
  ]

  const dftxyzFeatures = [
    "Signed artifact DAG (Ed25519 + Blake3)",
    "SMILES to signed molecular geometry",
    "Cached DFT result discovery and retrieval",
    "Operator-configured identity and storage adapters",
  ]

  return (
    <div className="grid md:grid-cols-2 gap-6">
      {/* Others column */}
      <div className="bg-card border border-border rounded-lg p-6">
        <h3 className="text-lg font-semibold text-muted-foreground mb-4">Under development</h3>
        <ul className="space-y-3">
          {plannedFeatures.map((feature, i) => (
            <ComparisonItem key={i} text={feature} included={false} />
          ))}
        </ul>
      </div>

      {/* DFT.xyz column */}
      <div className="bg-card border border-primary/30 rounded-lg p-6 relative overflow-hidden">
        <div className="absolute inset-0 bg-gradient-to-br from-primary/5 to-transparent" />
        <h3 className="text-lg font-semibold text-primary mb-4 relative">Implemented surfaces</h3>
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
        <span className="text-foreground font-semibold">Artifact integrity is inspectable.</span>{" "}
        Signatures bind payloads and provenance to a signer. They do not establish scientific
        correctness, successful payment, or that a deployed service is available.
      </p>
    </div>
  )
}
