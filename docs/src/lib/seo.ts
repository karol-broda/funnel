import { Metadata } from "next";

export const siteConfig = {
  name: "funnel",
  description:
    "A tunneling solution built with Go. Expose local services to the internet through websocket connections. Perfect for development, testing, and demonstration purposes.",
  url: "https://funnel.karolbroda.com",
  ogImage:
    "/og?title=funnel&description=A%20tunneling%20solution%20built%20with%20Go",
  keywords: [
    "tunneling",
    "ngrok alternative",
    "localhost tunnel",
    "websocket tunnel",
    "development tools",
    "local development",
    "expose localhost",
    "http tunnel",
    "secure tunnel",
    "go tunneling",
    "reverse proxy",
    "port forwarding",
    "developer tools",
    "local server",
    "webhook testing",
    "local https",
    "tunnel service",
    "self-hosted tunnel",
  ],
  creator: "Karol Broda",
  githubUrl: "https://github.com/karol-broda/funnel",
  twitterHandle: "@karolbroda",
  license: "MIT",
  version: "1.0.0",
  category: "Developer Tools",
};

export function createMetadata(
  title: string,
  description?: string,
  image?: string,
  path?: string
): Metadata {
  const metaTitle =
    title === siteConfig.name ? title : `${title} | ${siteConfig.name}`;
  const metaDescription = description || siteConfig.description;
  const metaImage =
    image ||
    `/og?title=${encodeURIComponent(
      metaTitle
    )}&description=${encodeURIComponent(metaDescription)}`;
  const canonicalUrl = path ? `${siteConfig.url}${path}` : siteConfig.url;

  return {
    title: metaTitle,
    description: metaDescription,
    keywords: siteConfig.keywords,
    authors: [{ name: siteConfig.creator }],
    creator: siteConfig.creator,
    publisher: siteConfig.creator,
    metadataBase: new URL(siteConfig.url),
    alternates: {
      canonical: canonicalUrl,
    },
    openGraph: {
      type: "website",
      locale: "en_US",
      url: canonicalUrl,
      title: metaTitle,
      description: metaDescription,
      siteName: siteConfig.name,
      images: [
        {
          url: metaImage,
          width: 1200,
          height: 630,
          alt: metaTitle,
        },
      ],
    },
    twitter: {
      card: "summary_large_image",
      title: metaTitle,
      description: metaDescription,
      images: [metaImage],
      creator: "@karolbroda",
    },
    robots: {
      index: true,
      follow: true,
      googleBot: {
        index: true,
        follow: true,
        "max-video-preview": -1,
        "max-image-preview": "large",
        "max-snippet": -1,
      },
    },
    verification: {
      google: "vKc0JFsriogcH7LEL0Ke9B7xd6j4TdAtIrool1A_Sck",
    },
  };
}

export function generateStructuredData(
  type: "website" | "article" | "software",
  data: {
    title: string;
    description: string;
    url: string;
    image?: string;
    datePublished?: string;
    dateModified?: string;
    author?: string;
  }
) {
  const baseData = {
    "@context": "https://schema.org",
    "@type":
      type === "website"
        ? "WebSite"
        : type === "article"
        ? "Article"
        : "SoftwareApplication",
    name: data.title,
    description: data.description,
    url: data.url,
    image: data.image || siteConfig.ogImage,
  };

  if (type === "website") {
    return {
      ...baseData,
      "@type": "WebSite",
      publisher: {
        "@type": "Person",
        name: siteConfig.creator,
      },
      potentialAction: {
        "@type": "SearchAction",
        target: {
          "@type": "EntryPoint",
          urlTemplate: `${siteConfig.url}/docs?q={search_term_string}`,
        },
        "query-input": "required name=search_term_string",
      },
    };
  }

  if (type === "article") {
    return {
      ...baseData,
      "@type": "Article",
      author: {
        "@type": "Person",
        name: data.author || siteConfig.creator,
      },
      publisher: {
        "@type": "Person",
        name: siteConfig.creator,
      },
      datePublished: data.datePublished,
      dateModified: data.dateModified,
      mainEntityOfPage: {
        "@type": "WebPage",
        "@id": data.url,
      },
    };
  }

  if (type === "software") {
    return {
      ...baseData,
      "@type": "SoftwareApplication",
      applicationCategory: "DeveloperApplication",
      operatingSystem: ["Linux", "macOS", "Windows"],
      programmingLanguage: "Go",
      downloadUrl: `${siteConfig.githubUrl}/releases`,
      codeRepository: siteConfig.githubUrl,
      license: `${siteConfig.githubUrl}/blob/master/LICENSE.md`,
      author: {
        "@type": "Person",
        name: siteConfig.creator,
      },
      offers: {
        "@type": "Offer",
        price: "0",
        priceCurrency: "USD",
      },
    };
  }

  return baseData;
}
