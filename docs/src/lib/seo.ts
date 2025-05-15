import type { Metadata } from 'next'
import type { DocsPage } from './source'

const baseUrl =
  process.env.NEXT_PUBLIC_URL ?? 'https://funnel.karolbroda.com'

type PageInfo = {
  title: string
  description?: string
  url: string
  slugs: string[]
  seo?: DocsPage['data']['seo']
}

export function buildPageMetadata(page: PageInfo): Metadata {
  const { seo } = page

  const metaTitle = seo?.title ?? page.title
  const metaDescription = seo?.description ?? page.description
  const ogImagePath = seo?.image ?? `/docs-og/${page.slugs.join('/')}`
  const canonical = seo?.canonical ?? page.url

  const metadata: Metadata = {
    title: metaTitle,
    description: metaDescription,
    alternates: {
      canonical,
    },
    openGraph: {
      title: metaTitle,
      description: metaDescription,
      images: [{ url: ogImagePath }],
    },
    twitter: {
      card: 'summary_large_image',
      title: metaTitle,
      description: metaDescription,
      images: [ogImagePath],
    },
  }

  if (seo?.keywords && seo.keywords.length > 0) {
    metadata.keywords = seo.keywords
  }

  if (seo?.noIndex) {
    metadata.robots = { index: false, follow: false }
  }

  return metadata
}

export function buildBreadcrumbJsonLd(slugs: string[], title: string) {
  const items = [
    { '@type': 'ListItem', position: 1, name: 'Home', item: baseUrl },
    {
      '@type': 'ListItem',
      position: 2,
      name: 'Docs',
      item: `${baseUrl}/docs`,
    },
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

export function buildTechArticleJsonLd(page: PageInfo) {
  const { seo } = page

  return {
    '@context': 'https://schema.org',
    '@type': 'TechArticle',
    headline: seo?.title ?? page.title,
    description: seo?.description ?? page.description,
    url: `${baseUrl}/docs/${page.slugs.join('/')}`,
    author: {
      '@type': 'Person',
      name: 'Karol Broda',
      url: 'https://karolbroda.com',
    },
    publisher: {
      '@type': 'Organization',
      name: 'funnel',
      url: baseUrl,
    },
    ...(seo?.keywords && seo.keywords.length > 0
      ? { keywords: seo.keywords.join(', ') }
      : {}),
  }
}
