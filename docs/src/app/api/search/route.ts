import { source } from '@/lib/source'
import type { StructuredData } from 'fumadocs-core/mdx-plugins'
import { createFromSource } from 'fumadocs-core/search/server'

export const { GET } = createFromSource(source, {
  language: 'english',
  async buildIndex(page) {
    const raw = await page.data.getText('raw')
    const structuredData = enrichStructuredData(
      page.data.structuredData ?? fallbackStructuredData(raw),
      {
        raw,
        keywords: page.data.seo?.keywords ?? [],
      },
    )

    return {
      title: page.data.title,
      description: page.data.description,
      url: page.url,
      id: page.url,
      structuredData,
      tag: page.data.search.tags,
    }
  },
})

function fallbackStructuredData(raw: string): StructuredData {
  const body = stripFrontmatter(raw)
  const headingMatches = Array.from(body.matchAll(/^(#{1,4})\s+(.+)$/gm))
  const headings = headingMatches.map((match) => ({
    id: slugify(match[2]),
    content: cleanInlineMdx(match[2]),
  }))

  return {
    headings,
    contents: buildSections(body, headingMatches),
  }
}

function stripFrontmatter(raw: string) {
  return raw.replace(/^---\n[\s\S]*?\n---\n?/, '')
}

function cleanSearchText(value: string) {
  return value
    .replace(/```[\w-]*\n([\s\S]*?)```/g, ' $1 ')
    .replace(/<([A-Z][\w.]*)\b([^>]*)>/g, ' $1 $2 ')
    .replace(/<\/?[^>]+>/g, ' ')
    .replace(/[{}()[\]`*_#>|]/g, ' ')
    .replace(/\s+/g, ' ')
    .trim()
}

function cleanInlineMdx(value: string) {
  return value
    .replace(/[`*_#]/g, '')
    .replace(/\s+/g, ' ')
    .trim()
}

function slugify(value: string) {
  return cleanInlineMdx(value)
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '')
}

function buildSections(
  body: string,
  headingMatches: RegExpMatchArray[],
): StructuredData['contents'] {
  if (headingMatches.length === 0) {
    return [{ heading: undefined, content: cleanSearchText(body) }]
  }

  const sections: StructuredData['contents'] = []
  const intro = cleanSearchText(body.slice(0, headingMatches[0].index))

  if (intro) {
    sections.push({ heading: undefined, content: intro })
  }

  headingMatches.forEach((match, index) => {
    const start = (match.index ?? 0) + match[0].length
    const end = headingMatches[index + 1]?.index ?? body.length
    const content = cleanSearchText(body.slice(start, end))

    if (content) {
      sections.push({
        heading: slugify(match[2]),
        content,
      })
    }
  })

  return sections
}

function enrichStructuredData(
  structuredData: StructuredData,
  metadata: {
    raw: string
    keywords: string[]
  },
): StructuredData {
  const supplementalTerms = [
    ...metadata.keywords.flatMap(expandIdentifier),
    ...extractSearchTerms(metadata.raw),
  ]

  return {
    headings: structuredData.headings,
    contents: [
      ...structuredData.contents,
      {
        heading: undefined,
        content: uniqueTerms(supplementalTerms).join(' '),
      },
    ].filter((section) => section.content.trim().length > 0),
  }
}

function extractSearchTerms(raw: string) {
  const body = stripFrontmatter(raw)
  const terms: string[] = []
  const patterns = [
    /`([^`\n]+)`/g,
    /```[\w-]*\n([\s\S]*?)```/g,
    /\b(?:GET|POST|PUT|PATCH|DELETE)\s+\/[a-z0-9_./{}:-]+/gi,
    /\/[a-z0-9_./{}:-]+/gi,
    /--[a-z0-9][a-z0-9-]*/gi,
    /\b[A-Z][A-Z0-9_]{2,}\b/g,
    /\b[\w.-]+\.(?:toml|json|mdx?|rs|nix|sh|tsx?|jsx?|css)\b/g,
    /\b[a-z][a-z0-9]+(?:_[a-z0-9]+)+\b/gi,
  ]

  for (const pattern of patterns) {
    for (const match of body.matchAll(pattern)) {
      const value = cleanSearchText(match[1] ?? match[0])

      if (value) {
        terms.push(...expandIdentifier(value))
      }
    }
  }

  return terms
}

function expandIdentifier(value: string) {
  const normalized = value.trim()

  if (!normalized) {
    return []
  }

  const words = normalized
    .replace(/([a-z0-9])([A-Z])/g, '$1 $2')
    .replace(/[-_/.:{}]+/g, ' ')
    .replace(/\s+/g, ' ')
    .trim()

  return words && words !== normalized ? [normalized, words] : [normalized]
}

function uniqueTerms(values: string[]) {
  return Array.from(
    new Set(
      values
        .map((value) => value.trim())
        .filter((value) => value.length > 0),
    ),
  )
}
