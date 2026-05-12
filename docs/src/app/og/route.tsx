import { palette } from '@/lib/colors'
import { createOGResponse, OGFrame } from '@/lib/og'

export const revalidate = 3600

export function GET() {
  return createOGResponse(
    <OGFrame>
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          width: '100%',
          height: '100%',
          position: 'relative',
        }}
      >
        <div
          style={{
            display: 'flex',
            fontFamily: 'DM Serif Display',
            fontSize: 120,
            letterSpacing: '-0.03em',
            color: palette.text,
          }}
        >
          funnel
        </div>
        <div
          style={{
            display: 'flex',
            fontSize: 24,
            color: palette.muted,
            marginTop: 12,
            letterSpacing: '0.15em',
            textTransform: 'uppercase' as const,
          }}
        >
          Self-hosted tunnels over QUIC
        </div>
      </div>
    </OGFrame>,
  )
}
