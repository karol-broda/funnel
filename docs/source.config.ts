import { defineDocs, defineConfig } from 'fumadocs-mdx/config'
import type { ShikiTransformer } from 'shiki'

export const docs = defineDocs({
  dir: 'content/docs',
})

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
