import { createMDX } from 'fumadocs-mdx/next'

/** @type {import('next').NextConfig} */
const config = {
  reactStrictMode: true,
  serverExternalPackages: [
    '@takumi-rs/core',
    '@takumi-rs/image-response',
    'shiki',
    '@shikijs/core',
    '@shikijs/engine-javascript',
    '@shikijs/engine-oniguruma',
  ],
  typescript: {
    ignoreBuildErrors: true,
  },
}

const withMDX = createMDX()

export default withMDX(config)
