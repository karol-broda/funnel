import { source } from '@/lib/source'
import { palette } from '@/lib/colors'
import {
  createOGResponse,
  OGFrame,
  OGLogo,
  OGTitle,
  OGDescription,
} from '@/lib/og'
import { notFound } from 'next/navigation'

export const revalidate = 3600

export async function GET(
  _req: Request,
  props: { params: Promise<{ slug?: string[] }> },
) {
  const params = await props.params
  const page = source.getPage(params.slug)
  if (!page) {
    notFound()
  }

  const title = page.data.title ?? ''
  const titleSize = title.length > 40 ? 48 : title.length > 25 ? 56 : 64

  const breadcrumb = params.slug?.slice(0, -1).join(' / ') ?? ''

  return createOGResponse(
    <OGFrame>
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          justifyContent: 'space-between',
          width: '100%',
          height: '100%',
          padding: '56px 72px',
          position: 'relative',
        }}
      >
        <OGLogo />

        <div style={{ display: 'flex', flexDirection: 'column' }}>
          {breadcrumb && (
            <div
              style={{
                display: 'flex',
                fontSize: 14,
                letterSpacing: '0.08em',
                textTransform: 'uppercase' as const,
                color: palette.muted,
                marginBottom: 12,
              }}
            >
              {breadcrumb}
            </div>
          )}
          <OGTitle size={titleSize}>{page.data.title}</OGTitle>
          {page.data.description && (
            <OGDescription>{page.data.description}</OGDescription>
          )}
        </div>
      </div>
    </OGFrame>,
  )
}
