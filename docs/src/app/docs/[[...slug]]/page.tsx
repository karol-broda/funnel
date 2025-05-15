import { source, type DocsPage } from '@/lib/source'
import {
  DocsPage as DocsPageLayout,
  DocsBody,
  DocsTitle,
  DocsDescription,
} from 'fumadocs-ui/page'
import { notFound } from 'next/navigation'
import { getMDXComponents } from '../../../../mdx-components'
import type { Metadata } from 'next'
import {
  buildPageMetadata,
  buildBreadcrumbJsonLd,
  buildTechArticleJsonLd,
} from '@/lib/seo'

function pageInfo(page: DocsPage) {
  return {
    title: page.data.title ?? '',
    description: page.data.description,
    url: page.url,
    slugs: page.slugs,
    seo: page.data.seo,
  }
}

export default async function Page(props: {
  params: Promise<{ slug?: string[] }>
}) {
  const params = await props.params
  const page = source.getPage(params.slug) as DocsPage | undefined
  if (!page) {
    notFound()
  }

  const MDX = page.data.body

  return (
    <DocsPageLayout
      toc={page.data.toc}
      tableOfContent={{ style: 'clerk' }}
      full={false}
    >
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{
          __html: JSON.stringify([
            buildBreadcrumbJsonLd(page.slugs, page.data.title ?? ''),
            buildTechArticleJsonLd(pageInfo(page)),
          ]),
        }}
      />
      <DocsTitle>{page.data.title}</DocsTitle>
      <DocsDescription>{page.data.description}</DocsDescription>
      <DocsBody>
        <MDX components={getMDXComponents()} />
      </DocsBody>
    </DocsPageLayout>
  )
}

export function generateStaticParams() {
  return source.generateParams()
}

export async function generateMetadata(props: {
  params: Promise<{ slug?: string[] }>
}): Promise<Metadata> {
  const params = await props.params
  const page = source.getPage(params.slug) as DocsPage | undefined
  if (!page) {
    notFound()
  }

  return buildPageMetadata(pageInfo(page))
}
