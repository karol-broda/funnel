import { docs } from '@/.source';
import { IconProps } from '@phosphor-icons/react';
import * as Icons from '@phosphor-icons/react/ssr';
import { loader } from 'fumadocs-core/source';
import { createOpenAPI, attachFile } from 'fumadocs-openapi/server';
import { createElement, FC } from 'react';

// See https://fumadocs.vercel.app/docs/headless/source-api for more info
export const source = loader({
  // it assigns a URL to your pages
  baseUrl: '/docs',
  source: docs.toFumadocsSource(),
  pageTree: {
    // adds a badge to each page item in page tree
    attachFile,
  },
  icon: icon => {
    if (icon == undefined) {
      return;
    }
    const iconName = !icon.endsWith('Icon') ? `${icon}Icon` : icon;

    const IconComponent = Icons[iconName as keyof typeof Icons];

    if (IconComponent !== undefined) {
      const Component = IconComponent as FC<IconProps>;

      return createElement(Component, {
        weight: 'bold',
        className: 'w-4 h-4',
      });
    }

    console.warn(
      `[Fumadocs] Icon "${icon}" not found in @phosphor-icons/react/ssr. No icon will be rendered for this page.`,
    );

    return;
  },
});

export const openapi = createOpenAPI({
  // Optional: configure proxy for CORS handling
  proxyUrl: '/api/proxy',
});
