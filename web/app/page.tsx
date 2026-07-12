import { Button } from "@/components/ui/button"
import { ArtifactDAG } from "@/components/artifact-dag"
import { PipelineCard } from "@/components/pipeline-card"
import { StatCard } from "@/components/stat-card"
import { MoleculeTicker } from "@/components/molecule-ticker"
import { ComparisonSection, DifferentiatorCallout } from "@/components/comparison-section"
import { AgentsSection } from "@/components/agents-section"
import { MoleculeFamilyCard } from "@/components/molecule-family-card"
import { ArchitectureDiagram } from "@/components/architecture-diagram"
import { ConnectWalletButton } from "@/components/connect-wallet-button"
import { ChallengeModal } from "@/components/challenge-modal"
import { ChallengeCard } from "@/components/challenge-card"
import { AgentIdentityVisual } from "@/components/agent-identity-visual"
import { SettlementFlow } from "@/components/settlement-flow"
import { DFTExplainer } from "@/components/dft-explainer"
import { AgentCashier } from "@/components/agent-cashier"
import { ServiceCatalog } from "@/components/service-catalog"
import { GatewayBadge } from "@/components/gateway-badge"
import { AgentApiPanel } from "@/components/agent-api-panel"
import { OrbitalGallery } from "@/components/orbital-gallery"
import { PaymentRails } from "@/components/payment-rails"
import { SAMPLE_CHALLENGES } from "@/lib/challenge-data"
import { Github, FileText, ExternalLink, Shield, Bot, Terminal, Wallet } from "lucide-react"

export default function Home() {
  return (
    <div className="min-h-screen orbital-bg grid-pattern">
      {/* Sticky Header */}
      <header className="sticky top-0 z-50 border-b border-border/50 bg-background/80 backdrop-blur-md">
        <div className="max-w-7xl mx-auto px-4 md:px-8 lg:px-16 h-16 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <h1 className="text-xl font-bold text-foreground">
              DFT<span className="text-primary">.xyz</span>
            </h1>
          </div>
          <div className="flex items-center gap-3">
            <GatewayBadge />
            <Button variant="ghost" size="sm" className="hidden sm:flex gap-2" asChild>
              <a href="#cashier">Cashier</a>
            </Button>
            <Button variant="ghost" size="sm" className="hidden sm:flex gap-2" asChild>
              <a href="#orbitals">Orbitals</a>
            </Button>
            <Button variant="ghost" size="sm" className="hidden sm:flex gap-2" asChild>
              <a href="#catalog">Catalog</a>
            </Button>
            <Button variant="ghost" size="sm" className="hidden md:flex gap-2" asChild>
              <a href="#agent-commerce">Agents</a>
            </Button>
            <ChallengeModal
              trigger={
                <Button variant="ghost" size="sm" className="hidden lg:flex gap-2">
                  <Shield className="size-4" />
                  Challenge
                </Button>
              }
            />
            <ConnectWalletButton />
          </div>
        </div>
      </header>

      {/* Hero Section */}
      <section className="relative py-20 px-4 md:px-8 lg:px-16">
        <div className="max-w-7xl mx-auto">
          <div className="grid lg:grid-cols-2 gap-12 items-center">
            {/* Left content */}
            <div className="fade-in-up">
              {/* Wordmark */}
              <h1 className="text-4xl md:text-5xl lg:text-6xl font-bold text-foreground mb-4">
                DFT<span className="text-primary">.xyz</span>
              </h1>

              <p className="mb-3 font-mono text-xs tracking-[0.2em] text-primary">
                AGENTIC DFT CASHIER · CHIMIADAO
              </p>
              <p className="mb-5 text-balance text-3xl font-medium leading-tight text-foreground md:text-4xl">
                Agents POST SMILES. We return signed DFT.
              </p>

              <p className="mb-8 max-w-xl text-pretty leading-relaxed text-muted-foreground">
                Primary customers are machines: HTTP catalog, x402 payment, content-addressed artifacts.
                No drawing board required. Humans can inspect the same cashier; agents integrate with curl.
              </p>

              <div className="flex flex-wrap gap-4">
                <Button size="lg" className="gap-2" asChild>
                  <a href="#cashier">
                    <Terminal className="size-5" />
                    Open cashier
                  </a>
                </Button>
                <Button size="lg" variant="outline" className="gap-2 border-border text-foreground hover:bg-muted" asChild>
                  <a href="#agent-commerce">
                    <Bot className="size-5" />
                    Agent API
                  </a>
                </Button>
              </div>
              <div className="mt-6 flex flex-wrap gap-x-6 gap-y-2 font-mono text-xs text-muted-foreground">
                <span>SMILES / labels</span>
                <span>HTTP 402 · stub→live USDC</span>
                <span>Signed artifacts</span>
              </div>
            </div>

            {/* Right content - Artifact DAG */}
            <div className="fade-in-up" style={{ animationDelay: "0.2s" }}>
              <ArtifactDAG />
            </div>
          </div>
        </div>
      </section>

      <section id="cashier" className="border-t border-border px-4 py-20 md:px-8 lg:px-16">
        <div className="mx-auto max-w-6xl">
          <div className="mb-10 flex flex-col gap-4 md:flex-row md:items-end md:justify-between">
            <div className="max-w-2xl">
              <p className="mb-3 font-mono text-xs tracking-[0.2em] text-primary">CASHIER</p>
              <h2 className="text-balance text-3xl font-bold text-foreground md:text-4xl">
                Same API for agents and operators
              </h2>
            </div>
            <p className="max-w-sm text-pretty text-sm leading-relaxed text-muted-foreground">
              Stub payments prove the loop today. Live mode flips facilitator + DAO pay_to — not a sketch UI.
            </p>
          </div>
          <AgentCashier />
        </div>
      </section>

      <OrbitalGallery />

      <ServiceCatalog />

      <PaymentRails />

      {/* Agent Identity & Wallet Section - NEW, promoted to top */}
      <section className="py-16 px-4 md:px-8 lg:px-16 border-t border-border bg-muted/10">
        <div className="max-w-5xl mx-auto">
          <div className="text-center mb-10">
            <h2 className="text-2xl md:text-3xl font-bold text-foreground mb-2">
              Agent Identity Meets Wallet Settlement
            </h2>
            <p className="text-muted-foreground max-w-2xl mx-auto">
              Every computation is traceable from ENS identity through signed artifact to wallet payment
            </p>
          </div>

          <AgentIdentityVisual className="mb-12" />

          <div className="grid md:grid-cols-3 gap-6 text-center">
            <div className="p-6 rounded-lg border border-border/50 bg-card/30">
              <h4 className="font-semibold text-foreground mb-2">Discoverable</h4>
              <p className="text-sm text-muted-foreground">
                Find agents via ENS. <code className="text-primary">chimiaclaw.eth</code> resolves
                to capabilities, public keys, and service endpoints.
              </p>
            </div>
            <div className="p-6 rounded-lg border border-border/50 bg-card/30">
              <h4 className="font-semibold text-foreground mb-2">Verifiable</h4>
              <p className="text-sm text-muted-foreground">
                Every result is signed with ed25519. Verify provenance without trusting the operator.
              </p>
            </div>
            <div className="p-6 rounded-lg border border-border/50 bg-card/30">
              <h4 className="font-semibold text-foreground mb-2">Settleable</h4>
              <p className="text-sm text-muted-foreground">
                Pay in USDC via your connected wallet. Escrow holds funds until computation completes.
              </p>
            </div>
          </div>
        </div>
      </section>

      {/* Wallet Settlement Flow - NEW */}
      <section className="py-16 px-4 md:px-8 lg:px-16 border-t border-border">
        <div className="max-w-4xl mx-auto">
          <div className="text-center mb-10">
            <h2 className="text-2xl md:text-3xl font-bold text-foreground mb-2">
              How Settlement Works
            </h2>
            <p className="text-muted-foreground">
              From quote to payment — all through your wallet
            </p>
          </div>

          <SettlementFlow className="mb-8" />

          <div className="grid sm:grid-cols-2 gap-6 mt-10">
            <div className="rounded-lg border border-amber-500/30 bg-amber-500/5 p-6">
              <h4 className="font-semibold text-amber-400 mb-2 flex items-center gap-2">
                <Wallet className="size-5" />
                USDC Settlement
              </h4>
              <p className="text-sm text-muted-foreground">
                Prices quoted in real-time via Uniswap V4 CLASSIC routing. 
                No volatile tokens — stable settlement in USDC.
              </p>
            </div>
            <div className="rounded-lg border border-primary/30 bg-primary/5 p-6">
              <h4 className="font-semibold text-primary mb-2 flex items-center gap-2">
                <Shield className="size-5" />
                Escrow Protection
              </h4>
              <p className="text-sm text-muted-foreground">
                Funds held in smart contract escrow until computation completes.
                Challenge results if you disagree.
              </p>
            </div>
          </div>
        </div>
      </section>

      {/* Challenge Section - NEW */}
      <section className="py-16 px-4 md:px-8 lg:px-16 border-t border-border bg-muted/10">
        <div className="max-w-4xl mx-auto">
          <div className="text-center mb-10">
            <h2 className="text-2xl md:text-3xl font-bold text-foreground mb-2">
              Challenge Any Calculation
            </h2>
            <p className="text-muted-foreground max-w-xl mx-auto">
              Connect your wallet and stake USDC to challenge suspicious results.
              If a discrepancy is found, you win the original operator&apos;s stake.
            </p>
          </div>

          <div className="flex justify-center mb-8">
            <ChallengeModal
              trigger={
                <Button size="lg" className="gap-2">
                  <Shield className="size-5" />
                  Challenge a Calculation
                </Button>
              }
            />
          </div>

          {SAMPLE_CHALLENGES.length > 0 && (
            <div className="space-y-4">
              <h3 className="text-sm font-medium text-muted-foreground">Active Challenges</h3>
              <div className="grid gap-4">
                {SAMPLE_CHALLENGES.map((challenge) => (
                  <ChallengeCard key={challenge.id} challenge={challenge} />
                ))}
              </div>
            </div>
          )}
        </div>
      </section>

      {/* Pipeline Section - Condensed */}
      <section className="py-16 px-4 md:px-8 lg:px-16 border-t border-border">
        <div className="max-w-7xl mx-auto">
          <h2 className="text-2xl md:text-3xl font-bold text-foreground mb-2 text-center">
            The Computation Pipeline
          </h2>
          <p className="text-muted-foreground text-center mb-6 max-w-2xl mx-auto">
            Every step produces a signed artifact — from input to anchored result
          </p>

          {/* DFT Explainer - collapsible */}
          <div className="max-w-2xl mx-auto mb-10">
            <DFTExplainer />
          </div>

          <div className="flex gap-6 overflow-x-auto pb-4 lg:justify-center">
            <PipelineCard
              step={1}
              title="Input"
              description="Drawn or imaged structure → validated geometry"
            />
            <PipelineCard
              step={2}
              title="MolADT"
              description="Typed molecular artifact, signed"
            />
            <PipelineCard
              step={3}
              title="DFT"
              description="PySCF quantum chemistry computation"
            />
            <PipelineCard
              step={4}
              title="Result"
              description="Signed energy, HOMO-LUMO, dipole"
            />
            <PipelineCard
              step={5}
              title="Settlement"
              description="Uniswap quote → wallet payment"
            />
            <PipelineCard
              step={6}
              title="Anchor"
              description="0G Storage + on-chain hash"
              isLast
            />
          </div>
        </div>
      </section>

      {/* Live Evidence Section */}
      <section className="py-16 px-4 md:px-8 lg:px-16 border-t border-border">
        <div className="max-w-7xl mx-auto">
          <h2 className="text-2xl md:text-3xl font-bold text-foreground mb-2 text-center">
            Real Computations. Real Artifacts.
          </h2>
          <p className="text-muted-foreground text-center mb-10">
            Live results from our autonomous DFT pipeline
          </p>

          <div className="grid sm:grid-cols-2 lg:grid-cols-4 gap-6 mb-8">
            <StatCard value="30+" label="DFT Results" sublabel="SCF-converged, signed" />
            <StatCard value="14" label="Molecules" sublabel="Computed overnight" />
            <StatCard value="12+" label="0G Anchors" sublabel="On-chain commitments" />
            <StatCard value="$115.50" label="Batch Quote" sublabel="Live Uniswap pricing" isPrice />
          </div>
        </div>

        {/* Molecule Ticker */}
        <MoleculeTicker />
      </section>

      {/* Why DFT.xyz Section */}
      <section className="py-16 px-4 md:px-8 lg:px-16 border-t border-border">
        <div className="max-w-5xl mx-auto">
          <h2 className="text-2xl md:text-3xl font-bold text-foreground mb-2 text-center">
            Not Just Automation. Trust Infrastructure.
          </h2>
          <p className="text-muted-foreground text-center mb-10">
            The difference between running DFT and building a verifiable computation layer
          </p>

          <ComparisonSection />
          <DifferentiatorCallout />
        </div>
      </section>

      {/* For Agents Section */}
      <section id="agent-commerce" className="py-16 px-4 md:px-8 lg:px-16 border-t border-border">
        <div className="max-w-5xl mx-auto">
          <h2 className="text-2xl md:text-3xl font-bold text-foreground mb-2 text-center">
            Built for Agent-to-Agent Commerce
          </h2>
          <p className="text-muted-foreground text-center mb-10">
            Discover, compute, and settle — all programmatically
          </p>

          <AgentsSection />
          <AgentApiPanel />
        </div>
      </section>

      {/* Molecule Families Section */}
      <section className="py-16 px-4 md:px-8 lg:px-16 border-t border-border">
        <div className="max-w-5xl mx-auto">
          <h2 className="text-2xl md:text-3xl font-bold text-foreground mb-2 text-center">
            What We Compute
          </h2>
          <p className="text-muted-foreground text-center mb-10">
            From industrial plasticizers to exotic main-group chemistry
          </p>

          <div className="grid sm:grid-cols-2 gap-6">
            <MoleculeFamilyCard
              title="Propylene Glycol Diesters"
              formula="C4–C12"
              description="Industrial plasticizers. Conformational energies, thermochemistry."
            />
            <MoleculeFamilyCard
              title="Group-14 Metallylenes"
              formula="Me₂Si:, Me₂Ge:, Me₂Sn:"
              description="HOMO-LUMO gaps across the periodic table column."
            />
            <MoleculeFamilyCard
              title="Organogermanium"
              formula="GeH₄ → germatrane"
              description="Real Olympus PBE/def2-svp scalar batch: germane → germatrane (signed overnight artifacts)."
            />
            <MoleculeFamilyCard
              title="Ge→Sn atranes"
              formula="NC₃Sn–R"
              description="Kavoosi-class cages from Ge XYZ starts. Seven PBE/def2-svp singles already on Olympus; raw worker JSON in demo/ge-sn-batch."
            />
          </div>
        </div>
      </section>

      {/* Architecture Section */}
      <section className="py-16 px-4 md:px-8 lg:px-16 border-t border-border">
        <div className="max-w-4xl mx-auto">
          <h2 className="text-2xl md:text-3xl font-bold text-foreground mb-2 text-center">
            The Stack
          </h2>
          <p className="text-muted-foreground text-center mb-10">
            From Rust artifacts to Solidity anchoring
          </p>

          <ArchitectureDiagram />
        </div>
      </section>

      {/* Footer */}
      <footer className="py-12 px-4 md:px-8 lg:px-16 border-t border-border bg-muted/20">
        <div className="max-w-5xl mx-auto">
          <div className="flex flex-col md:flex-row items-center justify-between gap-6">
            {/* Left */}
            <div className="text-center md:text-left">
              <h3 className="text-xl font-bold text-foreground mb-1">
                DFT<span className="text-primary">.xyz</span>
              </h3>
              <p className="text-muted-foreground text-sm">A ChimiaDAO project</p>
            </div>

            {/* Links */}
            <div className="flex flex-wrap justify-center gap-4">
              <a
                href="https://github.com/ChimiaDAO/OpenAgents"
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center gap-2 text-muted-foreground hover:text-foreground transition-colors text-sm"
              >
                <Github className="w-4 h-4" />
                GitHub
              </a>
              <a
                href="#agent-commerce"
                className="flex items-center gap-2 text-muted-foreground hover:text-foreground transition-colors text-sm"
              >
                <FileText className="w-4 h-4" />
                Agent API
              </a>
              <a
                href="https://www.chimiadao.io"
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center gap-2 text-muted-foreground hover:text-foreground transition-colors text-sm"
              >
                <ExternalLink className="w-4 h-4" />
                chimiadao.io
              </a>
              <a
                href="https://app.ens.domains/chimiaclaw.eth"
                target="_blank"
                rel="noopener noreferrer"
                className="flex items-center gap-2 text-muted-foreground hover:text-foreground transition-colors text-sm"
              >
                <ExternalLink className="w-4 h-4" />
                chimiaclaw.eth
              </a>
            </div>
          </div>

          {/* Bottom */}
          <div className="mt-8 pt-6 border-t border-border text-center">
            <p className="text-primary text-sm font-medium mb-2">
              Every result is a signed artifact. Verify it yourself.
            </p>
            <p className="text-muted-foreground text-xs">
              Built during ETHGlobal OpenAgents. Powered by PySCF, Uniswap, ENS, 0G Storage.
            </p>
          </div>
        </div>
      </footer>
    </div>
  )
}
