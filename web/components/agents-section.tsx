import { Server, Box, Globe } from "lucide-react"

interface AgentCardProps {
  icon: React.ReactNode
  title: string
  description: string
}

function AgentCard({ icon, title, description }: AgentCardProps) {
  return (
    <div className="bg-card border border-border rounded-lg p-6 transition-all duration-300 hover:border-primary/30">
      <div className="w-10 h-10 bg-primary/10 rounded-lg flex items-center justify-center mb-4 text-primary">
        {icon}
      </div>
      <h3 className="text-foreground font-semibold mb-2">{title}</h3>
      <p className="text-muted-foreground text-sm leading-relaxed">{description}</p>
    </div>
  )
}

export function AgentsSection() {
  return (
    <div className="grid md:grid-cols-3 gap-6">
      <AgentCard
        icon={<Server className="w-5 h-5" />}
        title="HTTP + x402"
        description="Discover SKUs at GET /v1/catalog and request chem.molecule.adt or cached chem.dft.result JSON. The HTTP 402 flow currently supports free or stub demonstrations."
      />
      <AgentCard
        icon={<Box className="w-5 h-5" />}
        title="SMILES & labels"
        description="Use SMILES for MolADT and cache labels for existing DFT results. New SCF jobs require a separate operator-run workflow."
      />
      <AgentCard
        icon={<Globe className="w-5 h-5" />}
        title="Identity + future settlement"
        description="ENS adapters support operator-configured identity. Verified payment and settlement are not yet connected; live mode fails closed."
      />
    </div>
  )
}
