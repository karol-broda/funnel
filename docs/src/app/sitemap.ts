import { MetadataRoute } from 'next';
import { source } from '@/lib/source';
import { siteConfig } from '@/lib/seo';
import { getLastModifiedFromUrl } from '@/lib/git';

export default async function sitemap(): Promise<MetadataRoute.Sitemap> {
  const pages = source.getPages();
  
  const staticPages = [
    {
      url: siteConfig.url,
      lastModified: new Date(),
      changeFrequency: 'monthly' as const,
      priority: 1,
    },
    {
      url: `${siteConfig.url}/docs`,
      lastModified: new Date(),
      changeFrequency: 'weekly' as const,
      priority: 0.8,
    },
  ];

  const docPages = await Promise.all(
    pages.map(async (page) => ({
      url: `${siteConfig.url}${page.url}`,
      lastModified: new Date(await getLastModifiedFromUrl(page.url)),
      changeFrequency: 'weekly' as const,
      priority: 0.6,
    }))
  );

  return [...staticPages, ...docPages];
} 