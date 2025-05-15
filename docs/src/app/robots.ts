import type { MetadataRoute } from 'next'

const baseUrl =
  process.env.NEXT_PUBLIC_URL ?? 'https://funnel.karolbroda.com'

export default function robots(): MetadataRoute.Robots {
  return {
    rules: {
      userAgent: '*',
      allow: '/',
    },
    sitemap: `${baseUrl}/sitemap.xml`,
  }
}
