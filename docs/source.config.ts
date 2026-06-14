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

const searchSchema = z.object({
  tags: z.array(z.string()).default([]),
  shortcut: z.boolean().default(false),
  shortcutOrder: z.number().default(0),
  shortcutTitle: z.string().default(''),
  shortcutDescription: z.string().default(''),
  shortcutIcon: z.string().default('BookOpenIcon'),
})

export const docs = defineDocs({
  dir: 'content/docs',
  docs: {
    schema: pageSchema.extend({
      seo: seoSchema.optional(),
      search: searchSchema.default({
        tags: [],
        shortcut: false,
        shortcutOrder: 0,
        shortcutTitle: '',
        shortcutDescription: '',
        shortcutIcon: 'BookOpenIcon',
      }),
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
