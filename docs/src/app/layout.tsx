import "katex/dist/katex.min.css";
import "./global.css";
import { RootProvider } from "fumadocs-ui/provider";
import { Inter } from "next/font/google";
import type { ReactNode } from "react";
import { createMetadata, generateStructuredData, siteConfig } from "@/lib/seo";

const inter = Inter({
  subsets: ["latin"],
});

export const metadata = createMetadata(
  siteConfig.name,
  siteConfig.description,
  siteConfig.ogImage,
  "/"
);

export default function Layout({ children }: { children: ReactNode }) {
  const structuredData = generateStructuredData("website", {
    title: siteConfig.name,
    description: siteConfig.description,
    url: siteConfig.url,
    image: siteConfig.ogImage,
  });

  return (
    <html lang="en" className={inter.className} suppressHydrationWarning>
      <head>
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{
            __html: JSON.stringify(structuredData),
          }}
        />
        <link rel="icon" href="/favicon.svg" type="image/svg+xml" />
        <meta name="msapplication-TileColor" content="#000000" />
        <link rel="manifest" href="/site.webmanifest" />
        <meta name="theme-color" content="#000000" />
        <meta name="color-scheme" content="dark light" />
        <meta name="robots" content="index,follow" />
        <meta name="googlebot" content="index,follow" />
        <link rel="author" href="/humans.txt" />
        <link rel="alternate" type="application/rss+xml" title="funnel RSS Feed" href="/feed.xml" />
      </head>
      <body>
        <RootProvider
          theme={{
            defaultTheme: "dark",
            storageKey: "theme",
          }}
        >
          {children}
        </RootProvider>
      </body>
    </html>
  );
}
