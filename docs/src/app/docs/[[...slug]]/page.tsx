import { source, type DocsPageData } from '@/lib/source'
import {
  DocsPage,
  DocsBody,
  DocsTitle,
  DocsDescription,
} from 'fumadocs-ui/page'
import { notFound } from 'next/navigation'
import { getMDXComponents } from '../../../../mdx-components'
import type { Metadata } from 'next'

const baseUrl =
  process.env.NEXT_PUBLIC_URL ?? 'https://funnel.karolbroda.com'

function buildBreadcrumbJsonLd(slugs: string[], title: string) {
  const items = [
    { '@type': 'ListItem', position: 1, name: 'Home', item: baseUrl },
    { '@type': 'ListItem', position: 2, name: 'Docs', item: `${baseUrl}/docs` },
  ]

  if (slugs.length > 0) {
    items.push({
      '@type': 'ListItem',
      position: 3,
      name: title,
      item: `${baseUrl}/docs/${slugs.join('/')}`,
    })
  }

  return {
    '@context': 'https://schema.org',
    '@type': 'BreadcrumbList',
    itemListElement: items,
  }
}

export default async function Page(props: {
  params: Promise<{ slug?: string[] }>
}) {
  const params = await props.params
  const page = source.getPage(params.slug)
  if (!page) {
    notFound()
  }

  const data = page.data as DocsPageData
  const MDX = data.body

  return (
    <DocsPage
      toc={data.toc}
      tableOfContent={{ style: 'clerk' }}
      full={false}
    >
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{
          __html: JSON.stringify(
            buildBreadcrumbJsonLd(page.slugs, page.data.title ?? ''),
          ),
        }}
      />
      <DocsTitle>{page.data.title}</DocsTitle>
      <DocsDescription>{page.data.description}</DocsDescription>
      <DocsBody>
        <MDX components={getMDXComponents()} />
      </DocsBody>
    </DocsPage>
  )
}

export function generateStaticParams() {
  return source.generateParams()
}

export async function generateMetadata(props: {
  params: Promise<{ slug?: string[] }>
}): Promise<Metadata> {
  const params = await props.params
  const page = source.getPage(params.slug)
  if (!page) {
    notFound()
  }

  const ogPath = `/docs-og/${page.slugs.join('/')}`

  return {
    title: page.data.title,
    description: page.data.description,
    alternates: {
      canonical: page.url,
    },
    openGraph: {
      title: page.data.title,
      description: page.data.description,
      images: [{ url: ogPath }],
    },
    twitter: {
      card: 'summary_large_image',
      title: page.data.title,
      description: page.data.description,
      images: [ogPath],
    },
  }
}
