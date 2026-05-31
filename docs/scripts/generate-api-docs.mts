import { generateFiles } from 'fumadocs-openapi'

function tagSlug(value: string) {
  return value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '')
}

void generateFiles({
  input: ['./openapi.json'],
  output: './content/docs/reference/server-api',
  per: 'tag',
  groupBy: 'tag',
  addGeneratedComment: true,
  frontmatter: (_title, _description, context) => {
    const seo =
      context.type === 'tag'
        ? (context.tag as Record<string, unknown>)?.['x-seo']
        : undefined

    const sectionTag =
      context.type === 'tag'
        ? tagSlug(String((context.tag as Record<string, unknown>)?.name ?? title))
        : 'server-api'

    return {
      full: true,
      ...(seo ? { seo } : {}),
      search: {
        tags: ['reference', 'server-api', sectionTag],
      },
    }
  },
})
