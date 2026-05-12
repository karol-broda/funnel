'use client'

import { motion } from 'motion/react'
import { fadeUp } from '@/lib/animation'

const features = [
  ['QUIC streams', 'Each HTTP request gets its own QUIC stream. No head-of-line blocking, no framing overhead.'],
  ['Automatic TLS', "Wildcard certificates from Let's Encrypt via DNS-01. Cloudflare, Route53, or your own provider."],
  ['Teams & OAuth', 'GitHub and generic OIDC login. API keys with scoped permissions. Team-scoped tunnels.'],
  ['Self-hosted', 'Single binary, embedded database option. Your infrastructure, your data.'],
  ['NixOS native', 'NixOS module with systemd hardening. Home Manager with sops-nix. OCI containers.'],
  ['Observable', 'Prometheus metrics for tunnels, bandwidth, and latency. Per-tunnel request stats.'],
] as const

export function FeatureGrid() {
  return (
    <div className="mt-10 space-y-10">
      {features.map(([title, detail], i) => (
        <motion.div
          key={title}
          {...fadeUp(i * 0.04)}
          className="grid gap-1 sm:grid-cols-[180px_1fr] sm:gap-8"
        >
          <h3 className="text-sm font-semibold">{title}</h3>
          <p className="text-sm leading-relaxed text-fd-muted-foreground">{detail}</p>
        </motion.div>
      ))}
    </div>
  )
}
