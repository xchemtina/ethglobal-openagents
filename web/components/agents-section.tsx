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
        description="Primary interface. Discover SKUs at GET /v1/catalog, pay with HTTP 402 / PAYMENT-SIGNATURE, receive signed chem.molecule.adt or chem.dft.result JSON. No GUI required."
      />
      <AgentCard
        icon={<Box className="w-5 h-5" />}
        title="SMILES & labels"
        description="Inputs are machine strings: SMILES for MolADT, cache labels for gallery DFT. Geometry workers and live SCF stay behind the cashier — agents never draw."
      />
      <AgentCard
        icon={<Globe className="w-5 h-5" />}
        title="ENS + settlement"
        description="Service identity under chimiaclaw.eth / market.chimiaclaw.eth. Stub mode for integration; live USDC on Base when CHIMIA_X402_PAY_TO and facilitator are set."
      />
    </div>
  )
}
