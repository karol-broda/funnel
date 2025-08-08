import defaultComponents from 'fumadocs-ui/mdx';
import type { MDXComponents } from 'mdx/types';
import Mermaid from './components/mdx/mermaid';
import * as Twoslash from 'fumadocs-twoslash/ui';
import { APIPage } from 'fumadocs-openapi/ui';
import { openapi } from '@/lib/source';

export function getMDXComponents(components: MDXComponents = {}): MDXComponents {
  return {
    ...defaultComponents,
    ...Twoslash,
    APIPage: (props) => <APIPage {...openapi.getAPIPageProps(props)} />,
    Mermaid,
    ...components,
  };
}
