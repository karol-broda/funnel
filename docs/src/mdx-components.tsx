import defaultComponents from 'fumadocs-ui/mdx';
import type { MDXComponents } from 'mdx/types';
import Mermaid from './components/mdx/mermaid';
import * as Twoslash from 'fumadocs-twoslash/ui';

export function getMDXComponents(components: MDXComponents = {}): MDXComponents {
  return {
    ...defaultComponents,
    ...Twoslash,
    Mermaid,
    ...components,
  };
}
