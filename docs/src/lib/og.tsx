import { ImageResponse } from '@takumi-rs/image-response'
import type { Font } from '@takumi-rs/core'
import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import { palette } from './colors'
import type { CSSProperties, ReactNode } from 'react'

const abs: CSSProperties = { display: 'flex', position: 'absolute' }
const serif: CSSProperties = { fontFamily: 'DM Serif Display' }

let fontData: Buffer | null = null

function loadFont(): Buffer {
  if (!fontData) {
    fontData = readFileSync(
      join(process.cwd(), 'public/dm-serif-display-latin-400-normal.woff2'),
    )
  }
  return fontData
}

export function getOGFont(): Font {
  return {
    name: 'DM Serif Display',
    data: loadFont(),
    weight: 400,
    style: 'normal',
  }
}

export interface OGResponseOptions {
  width?: number
  height?: number
  format?: 'png' | 'webp' | 'jpeg'
}

export function createOGResponse(
  element: React.ReactElement,
  options?: OGResponseOptions,
) {
  const { width = 1200, height = 630, format = 'png' } = options ?? {}

  return new ImageResponse(element, {
    width,
    height,
    format,
    fonts: [getOGFont()],
  })
}

export function OGFrame({ children }: { children: ReactNode }) {
  return (
    <div
      style={{
        display: 'flex',
        width: '100%',
        height: '100%',
        position: 'relative',
        overflow: 'hidden',
        background: `linear-gradient(145deg, ${palette.bg} 0%, #101d2a 50%, ${palette.bgCard} 100%)`,
      }}
    >
      {/* Warm glow */}
      <div
        style={{
          ...abs,
          top: '-150px',
          right: '-100px',
          width: '500px',
          height: '500px',
          borderRadius: '50%',
          background: `radial-gradient(circle, ${palette.accent}15 0%, transparent 70%)`,
        }}
      />
      <div
        style={{
          ...abs,
          bottom: '-200px',
          left: '-100px',
          width: '400px',
          height: '400px',
          borderRadius: '50%',
          background: `radial-gradient(circle, ${palette.accent}0a 0%, transparent 70%)`,
        }}
      />
      {children}
      {/* Gold accent line */}
      <div
        style={{
          ...abs,
          bottom: '0',
          left: '0',
          right: '0',
          height: '2px',
          background: `linear-gradient(90deg, ${palette.accent}, ${palette.accentDark} 60%, transparent)`,
        }}
      />
    </div>
  )
}

export function OGLogo() {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: '12px' }}>
      <div
        style={{
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          width: '36px',
          height: '36px',
          borderRadius: '8px',
          background: `${palette.accent}12`,
          border: `1px solid ${palette.accent}20`,
          ...serif,
          fontSize: '20px',
          color: palette.accent,
        }}
      >
        f
      </div>
      <div
        style={{
          display: 'flex',
          ...serif,
          fontSize: '20px',
          color: palette.muted,
        }}
      >
        funnel
      </div>
    </div>
  )
}

export function OGTitle({
  children,
  size = 64,
}: {
  children: ReactNode
  size?: number
}) {
  return (
    <div
      style={{
        display: 'flex',
        ...serif,
        fontSize: size,
        lineHeight: 1.15,
        letterSpacing: '-0.02em',
        color: palette.text,
        maxWidth: '800px',
      }}
    >
      {children}
    </div>
  )
}

export function OGDescription({ children }: { children: ReactNode }) {
  return (
    <div
      style={{
        display: 'flex',
        fontSize: 22,
        color: palette.subtle,
        marginTop: 20,
        lineHeight: 1.5,
        maxWidth: '700px',
      }}
    >
      {children}
    </div>
  )
}
