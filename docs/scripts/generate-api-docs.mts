import { generateFiles } from 'fumadocs-openapi'

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

    return {
      full: true,
      ...(seo ? { seo } : {}),
    }
  },
})
