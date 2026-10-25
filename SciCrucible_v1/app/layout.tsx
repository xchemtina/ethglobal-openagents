import type { Metadata, Viewport } from 'next'
import { Analytics } from '@vercel/analytics/next'
import './globals.css'

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
    <html lang="en" className="bg-background">
      <body className="font-sans antialiased">
        <aside className="fixed bottom-3 right-3 z-50 max-w-sm rounded-md border border-border bg-background/95 px-3 py-2 text-xs text-muted-foreground shadow-lg" aria-label="Scaffold status">
          Preview scaffold · fixture data. Identity, submissions, and reviews are not connected.
        </aside>
        {children}
        {process.env.NODE_ENV === 'production' && <Analytics />}
      </body>
    </html>
  )
}
