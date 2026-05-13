import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared'

function LogoMark() {
  return (
    <svg
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 32 32"
      fill="none"
      className="size-5 text-fd-primary"
    >
      <path
        d="M8 8h16l-5.5 12h-5L8 8z M22.625 11L27 8"
        stroke="currentColor"
        strokeWidth="1.75"
        strokeLinejoin="miter"
        fill="none"
      />
      <path
        d="M16 20v6"
        stroke="currentColor"
        strokeWidth="1.75"
      />
    </svg>
  )
}

export function baseOptions(): BaseLayoutProps {
  return {
    githubUrl: 'https://github.com/karol-broda/funnel',
    nav: {
      title: (
        <span className="flex items-center gap-2 font-[family-name:var(--font-display)] text-lg tracking-tight">
          <LogoMark />
          funnel
        </span>
      ),
    },
    links: [
      {
        text: 'Docs',
        url: '/docs',
        active: 'nested-url',
      },
    ],
  }
}
