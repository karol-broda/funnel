'use client'

import { motion } from 'motion/react'
import { fadeUp } from '@/lib/animation'

const steps = [
  {
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="h-5 w-5">
        <rect x="2" y="3" width="20" height="14" rx="2" />
        <path d="M8 21h8M12 17v4" />
      </svg>
    ),
    title: 'Your service',
    detail: 'localhost:3000',
    accent: false,
  },
  {
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="h-5 w-5">
        <path d="M5 12h14M12 5l7 7-7 7" />
      </svg>
    ),
    title: 'QUIC tunnel',
    detail: 'Per-request streams',
    accent: true,
  },
  {
    icon: (
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" className="h-5 w-5">
        <circle cx="12" cy="12" r="10" />
        <path d="M2 12h20M12 2a15.3 15.3 0 014 10 15.3 15.3 0 01-4 10 15.3 15.3 0 01-4-10 15.3 15.3 0 014-10z" />
      </svg>
    ),
    title: 'Public URL',
    detail: '*.tunnel.example.com',
    accent: false,
  },
] as const

function StepNode({
  icon,
  title,
  detail,
  accent,
  index,
}: {
  icon: React.ReactNode
  title: string
  detail: string
  accent: boolean
  index: number
}) {
  return (
    <motion.div {...fadeUp(index * 0.08)} className="flex flex-col items-center gap-3">
      <div
        className={`flex h-14 w-14 items-center justify-center rounded-full border backdrop-blur-sm text-lg ${
          accent
            ? 'border-fd-primary/30 bg-fd-primary/10 text-fd-primary'
            : 'border-fd-border bg-fd-card/60 text-fd-muted-foreground'
        }`}
      >
        {icon}
      </div>
      <div className="text-center">
        <div className="text-sm font-medium">{title}</div>
        <div className="mt-0.5 text-xs text-fd-muted-foreground">{detail}</div>
      </div>
    </motion.div>
  )
}

export function FlowDiagram() {
  return (
    <div className="relative">
      <div className="pointer-events-none absolute inset-x-0 top-7 flex justify-center">
        <div className="flex w-2/3 items-center">
          <div className="h-px flex-1 bg-gradient-to-r from-fd-border to-fd-primary/30" />
          <div className="h-px flex-1 bg-gradient-to-r from-fd-primary/30 to-fd-border" />
        </div>
      </div>
      <div className="relative grid grid-cols-3 gap-4 text-center">
        {steps.map((step, i) => (
          <StepNode key={step.title} {...step} index={i} />
        ))}
      </div>
    </div>
  )
}
