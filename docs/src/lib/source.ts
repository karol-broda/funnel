import { docs } from 'collections/server'
import { loader, type Page, type PageData } from 'fumadocs-core/source'
import type { DocData, DocMethods } from 'fumadocs-mdx/runtime/types'

export const source = loader({
  baseUrl: '/docs',
  source: docs.toFumadocsSource(),
})

export type DocsPageData = PageData & DocData & DocMethods
export type DocsPage = Page<undefined, DocsPageData>
