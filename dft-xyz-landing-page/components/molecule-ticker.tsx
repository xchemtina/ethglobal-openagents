interface MoleculeTickerItem {
  name: string
  formula?: string
  artifactId: string
  energy?: string
}

const molecules: MoleculeTickerItem[] = [
  { name: "Germane", formula: "GeH₄", artifactId: "art_c186f3052b8156c6", energy: "-2076.42 Ha" },
  { name: "Methylgermane", formula: "CH₃GeH₃", artifactId: "art_a7d2e1f09c4b3a8d", energy: "-2116.89 Ha" },
  { name: "Propylene glycol dihexanoate", artifactId: "art_b9e4c2d18f5a7632", energy: "-772.34 Ha" },
  { name: "Dimethylstannylene", formula: "Me₂Sn:", artifactId: "art_d3f8a6b24c91e057", energy: "-6174.18 Ha" },
  { name: "Dimethylgermylene", formula: "Me₂Ge:", artifactId: "art_e5c7d9a31b64f280", energy: "-2156.73 Ha" },
  { name: "Trimethylgermane", formula: "(CH₃)₃GeH", artifactId: "art_f1a8b3c46d927e54", energy: "-2197.21 Ha" },
  { name: "Germatrane", artifactId: "art_92d4e7f85a3c16b0", energy: "-2432.65 Ha" },
  { name: "Propylene glycol dioctanoate", artifactId: "art_74b1c9d26e8f3a45", energy: "-929.87 Ha" },
]

export function MoleculeTicker() {
  const doubledMolecules = [...molecules, ...molecules]

  return (
    <div className="w-full overflow-hidden bg-muted/30 border-y border-border py-3">
      <div className="ticker-scroll flex gap-8 whitespace-nowrap">
        {doubledMolecules.map((mol, i) => (
          <div key={i} className="flex items-center gap-4 text-sm">
            <span className="text-foreground font-medium">{mol.name}</span>
            {mol.formula && <span className="text-muted-foreground">{mol.formula}</span>}
            <code className="text-primary/70 font-mono text-xs bg-primary/5 px-2 py-0.5 rounded">
              {mol.artifactId}
            </code>
            {mol.energy && <span className="text-accent font-mono text-xs">{mol.energy}</span>}
            <span className="text-border">•</span>
          </div>
        ))}
      </div>
    </div>
  )
}
