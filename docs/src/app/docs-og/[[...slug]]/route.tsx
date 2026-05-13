import { source } from '@/lib/source'
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
  const titleSize = title.length > 30 ? 52 : 64

  return createOGResponse(
    <OGFrame>
      <div
        style={{
          display: 'flex',
          flexDirection: 'column',
          justifyContent: 'space-between',
          width: '100%',
          height: '100%',
          padding: '56px 64px',
          position: 'relative',
        }}
      >
        <OGLogo />
        <div style={{ display: 'flex', flexDirection: 'column' }}>
          <OGTitle size={titleSize}>{page.data.title}</OGTitle>
          {page.data.description && (
            <OGDescription>{page.data.description}</OGDescription>
          )}
        </div>
      </div>
    </OGFrame>,
  )
}
