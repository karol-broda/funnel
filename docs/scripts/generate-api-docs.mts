import { generateFiles } from 'fumadocs-openapi'

void generateFiles({
  input: ['./openapi.json'],
  output: './content/docs/reference/server-api',
  per: 'tag',
  groupBy: 'tag',
  addGeneratedComment: true,
})
