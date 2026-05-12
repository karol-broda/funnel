/* oxlint-disable new-cap -- Next.js font loaders are factory functions, not constructors */
import { RootProvider } from 'fumadocs-ui/provider/next'
import { DM_Sans, DM_Serif_Display } from 'next/font/google'
import type { ReactNode } from 'react'
import type { Metadata } from 'next'
import './global.css'

const body = DM_Sans({
  subsets: ['latin'],
  variable: '--font-body',
})

const display = DM_Serif_Display({
  subsets: ['latin'],
  weight: '400',
  variable: '--font-display',
})

const siteDescription =
  'Self-hosted tunneling over QUIC. Expose local services with automatic TLS, team management, and first-class NixOS support.'

export const metadata: Metadata = {
  metadataBase: new URL(
    process.env.NEXT_PUBLIC_URL ?? 'https://funnel.karolbroda.com',
  ),
  title: {
    template: '%s | funnel',
    default: 'funnel - Self-hosted tunnels over QUIC',
  },
  description: siteDescription,
  icons: {
    icon: '/icon.svg',
  },
  other: {
    'color-scheme': 'dark light',
  },
  openGraph: {
    title: 'funnel - Self-hosted tunnels over QUIC',
    description: siteDescription,
    siteName: 'funnel',
    locale: 'en_US',
    type: 'website',
    images: [{ url: '/og', width: 1200, height: 630 }],
  },
  twitter: {
    card: 'summary_large_image',
    title: 'funnel - Self-hosted tunnels over QUIC',
    description: siteDescription,
    images: ['/og'],
  },
}

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html
      lang="en"
      data-scroll-behavior="smooth"
      className={`${body.variable} ${display.variable}`}
      suppressHydrationWarning
    >
      <head>
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{
            __html: JSON.stringify([
              {
                '@context': 'https://schema.org',
                '@type': 'WebSite',
                name: 'funnel',
                url: 'https://funnel.karolbroda.com',
                description: siteDescription,
              },
              {
                '@context': 'https://schema.org',
                '@type': 'SoftwareSourceCode',
                name: 'funnel',
                description: siteDescription,
                url: 'https://funnel.karolbroda.com',
                codeRepository: 'https://github.com/karol-broda/funnel',
                programmingLanguage: 'Rust',
                license: 'https://opensource.org/licenses/MIT',
                author: {
                  '@type': 'Person',
                  name: 'Karol Broda',
                  url: 'https://karolbroda.com',
                },
              },
            ]),
          }}
        />
      </head>
      <body className="font-[family-name:var(--font-body)] antialiased">
        <RootProvider>{children}</RootProvider>
      </body>
    </html>
  )
}
