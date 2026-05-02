import type { Metadata, Viewport } from 'next'
import { Inter, JetBrains_Mono, Instrument_Serif, Chakra_Petch } from 'next/font/google'
import { Analytics } from '@vercel/analytics/next'
import './globals.css'

const inter = Inter({
  subsets: ['latin'],
  variable: '--font-inter',
  display: 'swap',
})

const jetbrainsMono = JetBrains_Mono({
  subsets: ['latin'],
  variable: '--font-jetbrains',
  display: 'swap',
})

// Editorial display serif — kept for italic editorial headlines (e.g. "Hard Science Discourse").
const instrumentSerif = Instrument_Serif({
  subsets: ['latin'],
  weight: '400',
  style: ['normal', 'italic'],
  variable: '--font-instrument',
  display: 'swap',
})

// Sci-fi technical numeric — used for all hero metrics and large numbers.
// Geometric, faintly stretched, with stencil-like cuts. The "Foundry Sterling does sci-fi" feel.
const chakraPetch = Chakra_Petch({
  subsets: ['latin'],
  weight: ['400', '500', '600', '700'],
  variable: '--font-chakra',
  display: 'swap',
})

export const metadata: Metadata = {
  title: 'Crucible — Hard Science Discourse',
  description: 'A rigorous, machine-readable platform for hard chemistry and physics. Derivations, open problems, experimental data, and autonomous research agents.',
  generator: 'crucible.science',
  keywords: ['quantum chemistry', 'physical chemistry', 'condensed matter', 'QFT', 'experimental physics', 'automated synthesis', 'Chemputer', 'World Avatar', 'scientific agents'],
  openGraph: {
    title: 'Crucible — Hard Science Discourse',
    description: 'Rigorous chemistry and physics. No hand-waving.',
    type: 'website',
  },
}

export const viewport: Viewport = {
  themeColor: '#0a1628',
  width: 'device-width',
  initialScale: 1,
}

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode
}>) {
  return (
    <html lang="en" className={`${inter.variable} ${jetbrainsMono.variable} ${instrumentSerif.variable} ${chakraPetch.variable} bg-background`}>
      <body className="font-sans antialiased">
        {children}
        {process.env.NODE_ENV === 'production' && <Analytics />}
      </body>
    </html>
  )
}
