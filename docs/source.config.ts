import { defineDocs, defineConfig } from 'fumadocs-mdx/config'
import { pageSchema } from 'fumadocs-core/source/schema'
import type { ShikiTransformer } from 'shiki'
import { z } from 'zod'

const seoSchema = z.object({
  title: z.string().optional(),
  description: z.string().optional(),
  keywords: z.array(z.string()).optional(),
  noIndex: z.boolean().optional(),
  canonical: z.string().optional(),
  image: z.string().optional(),
  priority: z.number().optional(),
})

export const docs = defineDocs({
  dir: 'content/docs',
  docs: {
    schema: pageSchema.extend({
      seo: seoSchema.optional(),
    }),
  },
})

function seoDefaults() {
  return {
    name: 'seo-defaults',
    doc: {
      frontmatter(data: Record<string, unknown>) {
        const title = data.title as string | undefined
        const description = data.description as string | undefined
        const seo = (data.seo ?? {}) as Record<string, unknown>

        if (title && !seo.title) {
          seo.title = `${title} | funnel docs`
        }

        if (description && !seo.description) {
          seo.description = description
        }

        data.seo = seo
        return data
      },
    },
  }
}

function transformerFullHeight(): ShikiTransformer {
  return {
    name: 'fullHeight',
    pre(node) {
      if (this.options.meta?.__raw?.includes('fullHeight')) {
        node.properties['data-full-height'] = ''
      }
      return node
    },
  }
}

export default defineConfig({
  plugins: [seoDefaults()],
  mdxOptions: {
    rehypeCodeOptions: {
      themes: {
        light: 'github-light-default',
        dark: 'github-dark-default',
      },
      transformers: [transformerFullHeight()],
    },
  },
})
