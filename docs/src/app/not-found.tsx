import Link from 'next/link'

export default function NotFound() {
  return (
    <main className="flex min-h-dvh flex-col items-center justify-center gap-4">
      <h1 className="text-4xl font-semibold tracking-tight">404</h1>
      <p className="text-fd-muted-foreground">Page not found.</p>
      <Link
        href="/"
        className="text-sm text-fd-primary underline underline-offset-4"
      >
        Back to home
      </Link>
    </main>
  )
}
