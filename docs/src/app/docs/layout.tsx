import { DocsLayout, type DocsLayoutProps } from "fumadocs-ui/layouts/docs";
import type { ReactNode } from "react";
import { baseOptions } from "@/app/layout.config";
import { source } from "@/lib/source";

const layoutOptions: DocsLayoutProps = {
  ...baseOptions,
  tree: source.pageTree,
  nav: {
    title: "funnel",
  },
  sidebar: {
    title: "funnel docs",
    banner: (
      <div className="flex flex-col gap-2 rounded-lg bg-card p-2 text-sm">
        <p className="font-semibold">⚠️ under development</p>
        <p className="text-muted-foreground">
          this project is a work in progress. features may change.
        </p>
      </div>
    ),
  },
};

export default function Layout({ children }: { children: ReactNode }) {
  return <DocsLayout {...layoutOptions}>{children}</DocsLayout>;
}
