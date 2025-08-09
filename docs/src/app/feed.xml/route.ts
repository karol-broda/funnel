import { siteConfig } from '@/lib/seo';

export async function GET() {
  const feed = `<?xml version="1.0" encoding="UTF-8" ?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
  <channel>
    <title>${siteConfig.name}</title>
    <description>${siteConfig.description}</description>
    <link>${siteConfig.url}</link>
    <atom:link href="${siteConfig.url}/feed.xml" rel="self" type="application/rss+xml" />
    <language>en-us</language>
    <lastBuildDate>${new Date().toUTCString()}</lastBuildDate>
    <item>
      <title>funnel - Open Source Tunneling Tool</title>
      <description>A tunneling solution built with Go for developers</description>
      <link>${siteConfig.url}</link>
      <guid isPermaLink="false">${siteConfig.url}</guid>
      <pubDate>${new Date().toUTCString()}</pubDate>
    </item>
  </channel>
</rss>`;

  return new Response(feed, {
    headers: {
      'Content-Type': 'application/xml',
      'Cache-Control': 'public, max-age=3600',
    },
  });
}
