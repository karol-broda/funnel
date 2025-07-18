import { source } from "@/lib/source";
import {
  DocsPage,
  DocsBody,
  DocsDescription,
  DocsTitle,
} from "fumadocs-ui/page";
import { notFound } from "next/navigation";
import { createMetadata, generateStructuredData, siteConfig } from "@/lib/seo";
import {
  getLastModifiedFromUrl,
  getCreatedFromUrl,
  getAuthorFromUrl,
} from "@/lib/git";
import { Metadata } from "next";
import LastModified from "@/components/mdx/last-modified";
import { getMDXComponents } from "@/mdx-components";

export default async function Page(props: {
  params: Promise<{ slug?: string[] }>;
}) {
  const params = await props.params;
  const page = source.getPage(params.slug);
  if (!page) notFound();

  const MDX = page.data.body;

  const lastModified = await getLastModifiedFromUrl(page.url);
  const datePublished = await getCreatedFromUrl(page.url);
  const author = await getAuthorFromUrl(page.url);

  const structuredData = generateStructuredData("article", {
    title: page.data.title,
    description: page.data.description || siteConfig.description,
    url: `${siteConfig.url}${page.url}`,
    datePublished,
    dateModified: lastModified,
    author,
  });

  return (
    <DocsPage toc={page.data.toc} full={page.data.full}>
      <script
        type="application/ld+json"
        dangerouslySetInnerHTML={{
          __html: JSON.stringify(structuredData),
        }}
      />
      <DocsTitle>{page.data.title}</DocsTitle>
      <DocsDescription>{page.data.description}</DocsDescription>
      <DocsBody>
        <MDX components={getMDXComponents()} />
        <LastModified date={lastModified} author={author} />
      </DocsBody>
    </DocsPage>
  );
}

export async function generateStaticParams() {
  return source.generateParams();
}

export async function generateMetadata(props: {
  params: Promise<{ slug?: string[] }>;
}): Promise<Metadata> {
  const params = await props.params;
  const page = source.getPage(params.slug);
  if (!page) notFound();

  const image = ["/docs-og", ...(params.slug || []), "image.png"].join("/");

  return createMetadata(
    page.data.title,
    page.data.description,
    image,
    page.url
  );
}
