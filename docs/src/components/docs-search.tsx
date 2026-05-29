'use client'

import {
  SearchDialog,
  SearchDialogClose,
  SearchDialogContent,
  SearchDialogHeader,
  SearchDialogInput,
  SearchDialogList,
  SearchDialogListItem,
  SearchDialogOverlay,
  type SearchItemType,
  type SharedProps,
} from 'fumadocs-ui/components/dialog/search'
import { useDocsSearch } from 'fumadocs-core/search/client'
import { useI18n } from 'fumadocs-ui/contexts/i18n'
import type { SearchLink } from 'fumadocs-ui/contexts/search'
import type { Icon } from '@phosphor-icons/react/dist/lib/types'
import { BookOpenIcon } from '@phosphor-icons/react/dist/csr/BookOpen'
import { CodeIcon } from '@phosphor-icons/react/dist/csr/Code'
import { CompassIcon } from '@phosphor-icons/react/dist/csr/Compass'
import { FileCodeIcon } from '@phosphor-icons/react/dist/csr/FileCode'
import { HashIcon } from '@phosphor-icons/react/dist/csr/Hash'
import { HardDrivesIcon } from '@phosphor-icons/react/dist/csr/HardDrives'
import { LifebuoyIcon } from '@phosphor-icons/react/dist/csr/Lifebuoy'
import { MagnifyingGlassIcon } from '@phosphor-icons/react/dist/csr/MagnifyingGlass'
import { PathIcon } from '@phosphor-icons/react/dist/csr/Path'
import { PlugsConnectedIcon } from '@phosphor-icons/react/dist/csr/PlugsConnected'
import { TerminalIcon } from '@phosphor-icons/react/dist/csr/Terminal'
import { TextAlignLeftIcon } from '@phosphor-icons/react/dist/csr/TextAlignLeft'
import type { ComponentType, ReactNode } from 'react'
import { isValidElement } from 'react'
import { useEffect, useRef } from 'react'
import { useMemo } from 'react'

export type QuickLink = {
  title: string
  href: string
  label: string
  icon: string
}

type DocsSearchDialogProps = SharedProps & {
  api?: string
  delayMs?: number
  links?: SearchLink[]
  quickLinks?: QuickLink[]
}

const icons: Record<string, ComponentType<IconProps>> = {
  BookOpenIcon,
  CompassIcon,
  TerminalIcon,
  HardDrivesIcon,
  PathIcon,
  FileCodeIcon,
  LifebuoyIcon,
  PlugsConnectedIcon,
}

type IconProps = Parameters<Icon>[0]

export function DocsSearchDialog({
  api,
  delayMs,
  links = [],
  quickLinks = [],
  ...props
}: DocsSearchDialogProps) {
  const { locale } = useI18n()
  const inputRef = useRef<HTMLInputElement>(null)
  const { search, setSearch, query } = useDocsSearch({
    type: 'fetch',
    api,
    locale,
    delayMs,
  })
  const defaultItems = useMemo(
    () => [
      ...quickLinks.map(
        (link): SearchItemType => ({
          id: link.href,
          type: 'page',
          url: link.href,
          content: <QuickLinkContent link={link} />,
        }),
      ),
      ...links.map(
        ([name, href]): SearchItemType => ({
          id: href,
          type: 'page',
          url: href,
          content: name,
        }),
      ),
    ],
    [links, quickLinks],
  )
  const items = query.data === 'empty' ? defaultItems : query.data
  const hasSearch = search.trim().length > 0

  useEffect(() => {
    if (!props.open) {
      return
    }

    const frame = requestAnimationFrame(() => {
      inputRef.current?.select()
    })

    return () => cancelAnimationFrame(frame)
  }, [props.open])

  return (
    <SearchDialog
      search={search}
      onSearchChange={setSearch}
      isLoading={query.isLoading}
      {...props}
    >
      <SearchDialogOverlay className="bg-fd-background/70 backdrop-blur-md" />
      <SearchDialogContent className="top-3 max-w-2xl rounded-lg border-fd-border bg-fd-popover shadow-[0_24px_80px_rgba(0,0,0,0.22)] md:top-[calc(50%-300px)]">
        <SearchDialogHeader className="border-b border-fd-border px-4 py-3">
          <MagnifyingGlassIcon
            className={query.isLoading ? 'size-4 animate-pulse' : 'size-4'}
            weight="duotone"
          />
          <SearchDialogInput
            ref={inputRef}
            autoFocus
            placeholder="Search docs..."
            className="text-base"
            onFocus={(event) => event.currentTarget.select()}
          />
          <SearchDialogClose className="h-7 px-2 text-[11px]" />
        </SearchDialogHeader>
        <SearchDialogList
          items={items}
          Empty={() => <EmptyState query={search} />}
          Item={({ item, onClick }) => (
            <SearchResult item={item} onClick={onClick} />
          )}
          className="docs-search-list"
        />
        <div className="flex items-center justify-between gap-3 border-t border-fd-border bg-fd-secondary/35 px-4 py-2 text-xs text-fd-muted-foreground">
          <span>{hasSearch ? 'Search results' : 'Useful starting points'}</span>
          <span className="hidden sm:inline">Enter to open</span>
        </div>
      </SearchDialogContent>
    </SearchDialog>
  )
}

function QuickLinkContent({ link }: { link: QuickLink }) {
  const Icon = icons[link.icon] ?? BookOpenIcon

  return (
    <div className="flex min-w-0 items-center gap-3">
      <span className="flex size-8 shrink-0 items-center justify-center rounded-md border border-fd-border bg-fd-card text-fd-muted-foreground">
        <Icon className="size-4" weight="duotone" />
      </span>
      <span className="min-w-0">
        <span className="block truncate font-medium text-fd-popover-foreground">
          {link.title}
        </span>
        <span className="block truncate text-xs text-fd-muted-foreground">
          {link.label}
        </span>
      </span>
    </div>
  )
}

function SearchResult({
  item,
  onClick,
}: {
  item: SearchItemType
  onClick: () => void
}) {
  if (item.type === 'action') {
    return (
      <SearchDialogListItem
        item={item}
        onClick={onClick}
        className="mx-1 my-0.5 rounded-md px-3 py-2"
      />
    )
  }

  if (isValidElement(item.content)) {
    return (
      <SearchDialogListItem
        item={item}
        onClick={onClick}
        className="mx-1 my-0.5 rounded-md px-3 py-2.5 aria-selected:bg-fd-accent/80"
      >
        {item.content}
      </SearchDialogListItem>
    )
  }

  const icon = resultIcon(item.type)
  const label = resultLabel(item.type)
  const content = String(item.content)

  if (item.type === 'text') {
    return (
      <MatchResult
        item={item}
        content={content}
        onClick={onClick}
      />
    )
  }

  return (
    <SearchDialogListItem
      item={item}
      onClick={onClick}
      className="mx-1 my-0.5 rounded-md px-3 py-2.5 aria-selected:bg-fd-accent/75"
    >
      <div className="flex min-w-0 gap-3 overflow-hidden">
        <span className="mt-0.5 flex size-7 shrink-0 items-center justify-center rounded-md border border-fd-border bg-fd-card text-fd-muted-foreground">
          {icon}
        </span>
        <span className="min-w-0 flex-1 overflow-hidden">
          <span className="mb-1 flex min-w-0 items-center gap-2">
            <ResultContent type={item.type} content={content} />
            <ResultBadge>{label}</ResultBadge>
          </span>
          <Breadcrumbs values={item.breadcrumbs} />
        </span>
      </div>
    </SearchDialogListItem>
  )
}

function MatchResult({
  item,
  content,
  onClick,
}: {
  item: SearchItemType
  content: string
  onClick: () => void
}) {
  const codeHeavy = isCodeHeavy(content)

  return (
    <SearchDialogListItem
      item={item}
      onClick={onClick}
      className="mx-1 my-0.5 rounded-md px-3 py-2.5 aria-selected:bg-fd-accent/75"
    >
      <div className="flex min-w-0 gap-3 overflow-hidden">
        <span className="mt-0.5 flex size-7 shrink-0 items-center justify-center rounded-md border border-fd-border bg-fd-card text-fd-muted-foreground">
          {codeHeavy ? (
            <CodeIcon className="size-3.5" weight="duotone" />
          ) : (
            <TextAlignLeftIcon className="size-3.5" weight="duotone" />
          )}
        </span>
        <span className="min-w-0 flex-1 overflow-hidden">
          <span
            className={
              codeHeavy
                ? 'docs-search-match-inline-code'
                : 'docs-search-match-inline'
            }
            dangerouslySetInnerHTML={{ __html: sanitizeResultHtml(content) }}
          />
          <Breadcrumbs values={'breadcrumbs' in item ? item.breadcrumbs : undefined} />
        </span>
      </div>
    </SearchDialogListItem>
  )
}

function ResultContent({
  type,
  content,
}: {
  type: 'page' | 'heading' | 'text'
  content: string
}) {
  if (type === 'text') {
    return (
      <span
        className={
          isCodeHeavy(content)
            ? 'docs-search-code-result'
            : 'docs-search-text-result'
        }
        dangerouslySetInnerHTML={{ __html: sanitizeResultHtml(content) }}
      />
    )
  }

  return (
    <span
      className={
        type === 'heading'
          ? 'docs-search-heading-result'
          : 'docs-search-page-result'
      }
      dangerouslySetInnerHTML={{ __html: sanitizeResultHtml(content) }}
    />
  )
}

function ResultBadge({ children }: { children: ReactNode }) {
  return (
    <span className="shrink-0 rounded border border-fd-border px-1.5 py-0.5 text-[10px] uppercase text-fd-muted-foreground">
      {children}
    </span>
  )
}

function Breadcrumbs({ values }: { values?: ReactNode[] }) {
  if (!values || values.length === 0) {
    return null
  }

  return (
    <span className="mt-1 flex min-w-0 items-center gap-1 overflow-hidden text-xs text-fd-muted-foreground">
      {values.map((value, index) => (
        <span key={index} className="contents">
          {index > 0 ? <span className="text-fd-border">/</span> : null}
          <span className="truncate">{value}</span>
        </span>
      ))}
    </span>
  )
}

function sanitizeResultHtml(value: string) {
  return value
    .replace(/<(?!\/?mark\b)[^>]*>/g, ' ')
    .replace(/<mark\b[^>]*>/g, '<mark>')
    .replace(/\s+/g, ' ')
    .trim()
}

function isCodeHeavy(value: string) {
  const text = value.replace(/<[^>]+>/g, '')

  return (
    /`|--[a-z0-9-]+|\/[a-z0-9_./{}:-]+|\b[A-Z][A-Z0-9_]{2,}\b|[{}()[\]=]/i.test(
      text,
    ) || text.length > 96
  )
}


function EmptyState({ query }: { query: string }) {
  return (
    <div className="px-4 py-10 text-center">
      <BookOpenIcon
        className="mx-auto mb-3 size-5 text-fd-muted-foreground"
        weight="duotone"
      />
      <p className="text-sm font-medium text-fd-popover-foreground">
        No results for &quot;{query}&quot;
      </p>
      <p className="mx-auto mt-1 max-w-sm text-sm text-fd-muted-foreground">
        Try a command, config key, endpoint, or page title.
      </p>
    </div>
  )
}

function resultIcon(type: SearchItemType['type']) {
  if (type === 'heading') {
    return <HashIcon className="size-3.5" weight="duotone" />
  }

  if (type === 'text') {
    return <TextAlignLeftIcon className="size-3.5" weight="duotone" />
  }

  return <BookOpenIcon className="size-3.5" weight="duotone" />
}

function resultLabel(type: SearchItemType['type']) {
  if (type === 'heading') {
    return 'Section'
  }

  if (type === 'text') {
    return 'Match'
  }

  return 'Page'
}
