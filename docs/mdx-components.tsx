import type { MDXComponents } from 'mdx/types'
import defaultMdxComponents from 'fumadocs-ui/mdx'
import { CodeBlock, Pre } from 'fumadocs-ui/components/codeblock'
import { ImageZoom, type ImageZoomProps } from 'fumadocs-ui/components/image-zoom'
import * as TabsComponents from 'fumadocs-ui/components/tabs'
import { APIPage } from 'fumadocs-openapi/ui'

export function getMDXComponents(components?: MDXComponents): MDXComponents {
  return {
    ...defaultMdxComponents,
    ...TabsComponents,
    APIPage,
    img: (props: React.ComponentProps<'img'>) => (
      <ImageZoom {...(props as ImageZoomProps)} />
    ),
    pre: ({ ref: _ref, ...props }: React.ComponentProps<'pre'>) => (
      <CodeBlock {...props}>
        <Pre>{props.children}</Pre>
      </CodeBlock>
    ),
    ...components,
  }
}

export function useMDXComponents(components: MDXComponents): MDXComponents {
  return getMDXComponents(components)
}
