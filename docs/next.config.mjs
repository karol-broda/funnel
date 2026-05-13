import { createMDX } from 'fumadocs-mdx/next'

/** @type {import('next').NextConfig} */
const config = {
  reactStrictMode: true,
  serverExternalPackages: [
    '@takumi-rs/core',
    '@takumi-rs/image-response',
  ],
  typescript: {
    ignoreBuildErrors: true,
  },
}

const withMDX = createMDX()

export default withMDX(config)
