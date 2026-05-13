'use client'

export default function GlobalError({
  reset,
}: {
  error: Error & { digest?: string }
  reset: () => void
}) {
  return (
    <html lang="en">
      <body
        style={{
          margin: 0,
          minHeight: '100dvh',
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          fontFamily:
            'ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
          background: '#1a1d2e',
          color: '#e0ddd8',
        }}
      >
        <h1
          style={{
            fontSize: 'clamp(6rem, 20vw, 12rem)',
            lineHeight: 1,
            letterSpacing: '-0.02em',
            margin: 0,
            opacity: 0.1,
            fontWeight: 400,
          }}
        >
          500
        </h1>
        <p
          style={{
            marginTop: '0.5rem',
            fontSize: '0.875rem',
            textTransform: 'uppercase',
            letterSpacing: '0.2em',
            opacity: 0.5,
          }}
        >
          Something went wrong
        </p>
        <button
          onClick={reset}
          style={{
            marginTop: '2rem',
            display: 'inline-flex',
            alignItems: 'center',
            gap: '0.5rem',
            borderRadius: '9999px',
            padding: '0.625rem 1.5rem',
            fontSize: '0.875rem',
            fontWeight: 500,
            color: 'rgba(224, 221, 216, 0.7)',
            background: 'rgba(224, 221, 216, 0.06)',
            border: 'none',
            boxShadow: 'inset 0 0 0 1px rgba(224, 221, 216, 0.08)',
            cursor: 'pointer',
          }}
        >
          Try again
        </button>
      </body>
    </html>
  )
}
