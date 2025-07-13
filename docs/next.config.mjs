import { createMDX } from 'fumadocs-mdx/next';
import { transformerTwoslash } from 'fumadocs-twoslash';

const withMDX = createMDX();

/** @type {import('next').NextConfig} */
const config = {
  reactStrictMode: true,
  serverExternalPackages: ['typescript', 'twoslash'],
};

export default withMDX(config);
