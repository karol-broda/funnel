import { source, type DocsPage } from '@/lib/source'

const baseUrl = process.env.NEXT_PUBLIC_URL ?? 'https://funnel.karolbroda.com'

type SitemapEntry = {
  url: string
  lastModified: Date
  changeFrequency: string
  priority: number
}

function buildEntries(): SitemapEntry[] {
  const pages = (source.getPages() as DocsPage[]).filter(
    (page) => !page.data.seo?.noIndex,
  )

  return [
    {
      url: baseUrl,
      lastModified: new Date(),
      changeFrequency: 'monthly',
      priority: 1,
    },
    ...pages.map((page) => ({
      url: `${baseUrl}${page.url}`,
      lastModified: new Date(),
      changeFrequency: 'weekly',
      priority: page.data.seo?.priority ?? 0.8,
    })),
  ]
}

function toXml(entries: SitemapEntry[]): string {
  const urls = entries
    .map(
      (entry) => `  <url>
    <loc>${entry.url}</loc>
    <lastmod>${entry.lastModified.toISOString()}</lastmod>
    <changefreq>${entry.changeFrequency}</changefreq>
    <priority>${entry.priority}</priority>
  </url>`,
    )
    .join('\n')

  return `<?xml version="1.0" encoding="UTF-8"?>
<?xml-stylesheet type="text/xsl" href="/sitemap.xsl"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
${urls}
</urlset>`
}

export function GET() {
  return new Response(toXml(buildEntries()), {
    headers: { 'Content-Type': 'application/xml' },
  })
}
