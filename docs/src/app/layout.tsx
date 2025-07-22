import "katex/dist/katex.min.css";
import "./global.css";
import { RootProvider } from "fumadocs-ui/provider";
import { Inter } from "next/font/google";
import type { ReactNode } from "react";
import { createMetadata, generateStructuredData, siteConfig } from "@/lib/seo";
import { PostHogProviderWrapper } from "@/components/analytics/posthog-provider";
import { PageTrackerWithSuspense } from "@/components/analytics/page-tracker-suspense";

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
        <link rel="icon" href="/favicon.ico" />
        <link rel="icon" href="/favicon.svg" type="image/svg+xml" />
        {/* <link rel="apple-touch-icon" href="/apple-touch-icon.png" /> */}
        <link rel="manifest" href="/site.webmanifest" />
        <meta name="theme-color" content="#000000" />
        <meta name="color-scheme" content="dark light" />
      </head>
      <body>
        <PostHogProviderWrapper>
          <PageTrackerWithSuspense />
          <RootProvider
            theme={{
              defaultTheme: "dark",
              storageKey: "theme",
            }}
          >
            {children}
          </RootProvider>
        </PostHogProviderWrapper>
      </body>
    </html>
  );
}
