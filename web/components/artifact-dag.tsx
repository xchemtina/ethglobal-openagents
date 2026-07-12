"use client"

import { useEffect, useRef } from "react"

export function ArtifactDAG() {
  const svgRef = useRef<SVGSVGElement>(null)

  const nodes = [
    { id: "structure", label: "Structure", x: 50, y: 100 },
    { id: "moladt", label: "MolADT", x: 150, y: 100 },
    { id: "dft", label: "DFT Request", x: 250, y: 100 },
    { id: "result", label: "DFT Result", x: 350, y: 100 },
    { id: "quote", label: "Uniswap Quote", x: 450, y: 100 },
    { id: "anchor", label: "0G Anchor", x: 550, y: 100 },
  ]

  const connections = [
    { from: 0, to: 1 },
    { from: 1, to: 2 },
    { from: 2, to: 3 },
    { from: 3, to: 4 },
    { from: 4, to: 5 },
  ]

  return (
    <div className="relative w-full max-w-2xl mx-auto lg:mx-0">
      <svg
        ref={svgRef}
        viewBox="0 0 620 200"
        className="w-full h-auto"
        style={{ minHeight: "200px" }}
      >
        <defs>
          <linearGradient id="lineGradient" x1="0%" y1="0%" x2="100%" y2="0%">
            <stop offset="0%" stopColor="#3b82f6" stopOpacity="0.3" />
            <stop offset="50%" stopColor="#3b82f6" stopOpacity="0.8" />
            <stop offset="100%" stopColor="#3b82f6" stopOpacity="0.3" />
          </linearGradient>
          <filter id="glow">
            <feGaussianBlur stdDeviation="2" result="coloredBlur" />
            <feMerge>
              <feMergeNode in="coloredBlur" />
              <feMergeNode in="SourceGraphic" />
            </feMerge>
          </filter>
        </defs>

        {/* Connection lines */}
        {connections.map((conn, i) => (
          <line
            key={i}
            x1={nodes[conn.from].x + 35}
            y1={nodes[conn.from].y}
            x2={nodes[conn.to].x - 5}
            y2={nodes[conn.to].y}
            stroke="url(#lineGradient)"
            strokeWidth="2"
            className="flow-line"
            style={{ animationDelay: `${i * 0.2}s` }}
          />
        ))}

        {/* Nodes */}
        {nodes.map((node, i) => (
          <g key={node.id} style={{ animationDelay: `${i * 0.1}s` }}>
            {/* Node circle */}
            <circle
              cx={node.x}
              cy={node.y}
              r="18"
              fill="#12121a"
              stroke="#3b82f6"
              strokeWidth="2"
              filter="url(#glow)"
            />
            {/* Lock icon */}
            <g transform={`translate(${node.x - 6}, ${node.y - 6})`}>
              <rect
                x="2"
                y="5"
                width="8"
                height="6"
                rx="1"
                fill="none"
                stroke="#3b82f6"
                strokeWidth="1"
              />
              <path
                d="M3 5 V3 C3 1 5 0 6 0 C7 0 9 1 9 3 V5"
                fill="none"
                stroke="#3b82f6"
                strokeWidth="1"
              />
            </g>
            {/* Label */}
            <text
              x={node.x}
              y={node.y + 40}
              textAnchor="middle"
              fill="#9ca3af"
              fontSize="10"
              fontFamily="monospace"
            >
              {node.label}
            </text>
          </g>
        ))}
      </svg>

      {/* Subtle pulse effect */}
      <div className="absolute inset-0 pointer-events-none">
        <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-32 h-32 bg-primary/5 rounded-full blur-3xl" />
      </div>
    </div>
  )
}
