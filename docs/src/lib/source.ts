import { docs } from 'collections/server'
import { loader, type InferPageType } from 'fumadocs-core/source'

export const source = loader({
  baseUrl: '/docs',
  source: docs.toFumadocsSource(),
})

export type DocsPage = InferPageType<typeof source>
