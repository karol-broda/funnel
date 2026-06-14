import { source } from '@/lib/source'
import type { QuickLink } from '@/components/docs-search'

export function searchDialogOptions() {
  const pages = source.getPages()

  return {
    delayMs: 120,
    quickLinks: pages
      .filter((page) => page.data.search.shortcut)
      .sort(
        (a, b) =>
          a.data.search.shortcutOrder - b.data.search.shortcutOrder,
      )
      .map(
        (page): QuickLink => ({
          title: page.data.search.shortcutTitle || page.data.title,
          href: page.url,
          label:
            page.data.search.shortcutDescription ||
            page.data.description ||
            page.url,
          icon: page.data.search.shortcutIcon,
        }),
      ),
  }
}
