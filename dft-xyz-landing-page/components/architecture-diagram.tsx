export function ArchitectureDiagram() {
  return (
    <div className="bg-card border border-border rounded-lg p-8 overflow-x-auto">
      <div className="min-w-[600px]">
        {/* Stack layers */}
        <div className="space-y-3">
          {/* Top layer */}
          <div className="bg-primary/10 border border-primary/30 rounded-lg p-4 text-center">
            <code className="text-primary font-mono text-sm">
              Rust-native signed artifact DAG (chimiaclaw-artifact)
            </code>
          </div>

          {/* Middle layers */}
          <div className="grid grid-cols-5 gap-2">
            {["MolADT", "DFT Worker", "Settlement", "Storage", "Identity"].map((layer, i) => (
              <div
                key={layer}
                className="bg-muted border border-border rounded-lg p-3 text-center"
              >
                <span className="text-foreground text-xs font-medium">{layer}</span>
                <div className="text-muted-foreground text-[10px] mt-1 font-mono">
                  {["Molecular ADT", "PySCF/Skala", "Uniswap", "0G", "ENS"][i]}
                </div>
              </div>
            ))}
          </div>

          {/* Bottom layer */}
          <div className="bg-accent/10 border border-accent/30 rounded-lg p-4">
            <div className="text-center">
              <code className="text-accent font-mono text-sm">
                Solidity: ArtifactAnchor.sol + SettlementEscrow.sol + CapabilityRegistry.sol
              </code>
            </div>
          </div>

          {/* CLI sidebar indicator */}
          <div className="flex justify-end mt-4">
            <div className="bg-muted/50 border border-border rounded-lg px-4 py-2 inline-flex items-center gap-2">
              <span className="text-muted-foreground text-xs">CLI:</span>
              <code className="text-foreground font-mono text-xs">
                chimiaclaw-cli --features live-sponsors
              </code>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
