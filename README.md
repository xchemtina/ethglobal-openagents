# chimiaclaw

**Rust-native signed artifact DAG for autonomous scientific agents**  
*(built live for EthGlobal OpenAgents 2026)*

Every scientific action, agent decision, procurement, and governance event becomes an **immutable signed artifact** in a verifiable DAG.

```mermaid
%%{init: {'theme': 'dark', 'themeVariables': {
  'primaryColor': '#00f5ff',
  'primaryTextColor': '#ffffff',
  'lineColor': '#39ff14',
  'secondaryColor': '#16213e',
  'tertiaryColor': '#ff00aa',
  'mainBkg': '#0a0a12',
  'nodeBorder': '#00f5ff'
}}}%%
flowchart TD
    Papers[Scientific Papers] --> Literature[Literature Agent<br/>science.literature.synthesis]
    Literature -- signed --> Retrosynthesis[Retrosynthesis Agent<br/>chem.retrosynth.route_proposal]
    Retrosynthesis -- signed --> DFT[DFT Agent<br/>chem.dft.result]
    DFT -- signed --> Evidence[HOMO/LUMO + Evidence]
    Evidence -. context .-> Literature
