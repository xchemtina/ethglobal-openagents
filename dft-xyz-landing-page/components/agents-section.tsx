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
        title="MCP Server"
        description="Any MCP-aware agent discovers DFT.xyz tools automatically. Submit a validated molecular structure and receive a signed artifact with energy, gap, dipole, and a settlement quote."
      />
      <AgentCard
        icon={<Box className="w-5 h-5" />}
        title="Molecule.xyz Integration"
        description="Deploy as an executor module on Onchain Labs. The lab's treasury funds compute, results write back as versioned records with content identifiers."
      />
      <AgentCard
        icon={<Globe className="w-5 h-5" />}
        title="ENS Discovery"
        description="Each agent (dft.service, retro.service, literature.service) publishes capabilities to chimiaclaw.eth on Sepolia. Settlement endpoint: uniswap-trade-api:CLASSIC:V2+V3+V4."
      />
    </div>
  )
}
