'use client'

import Link from 'next/link'

export default function Error({
  reset,
}: {
  error: Error & { digest?: string }
  reset: () => void
}) {
  return (
    <main className="flex min-h-dvh flex-col items-center justify-center bg-fd-background">
      <h1 className="font-[family-name:var(--font-display)] text-[clamp(6rem,20vw,12rem)] leading-none tracking-tight text-fd-foreground/10">
        500
      </h1>
      <p className="mt-2 text-sm uppercase tracking-[0.2em] text-fd-muted-foreground">
        Something went wrong
      </p>
      <div className="mt-8 flex gap-3">
        <button
          onClick={reset}
          className="inline-flex items-center gap-2 rounded-full bg-fd-primary/90 px-6 py-2.5 text-sm font-medium text-fd-primary-foreground ring-1 ring-inset ring-fd-primary/20 transition-all hover:bg-fd-primary hover:ring-fd-primary/40"
        >
          Try again
        </button>
        <Link
          href="/"
          className="inline-flex items-center gap-2 rounded-full bg-fd-foreground/[0.06] px-6 py-2.5 text-sm font-medium text-fd-foreground/70 ring-1 ring-inset ring-fd-foreground/[0.08] transition-all hover:bg-fd-foreground/[0.12] hover:text-fd-foreground/90"
        >
          Back to home
        </Link>
      </div>
    </main>
  )
}
