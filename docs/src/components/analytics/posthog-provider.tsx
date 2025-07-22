"use client";

import posthog from "posthog-js";
import { PostHogProvider } from "posthog-js/react";
import { ReactNode } from "react";

type PostHogProviderWrapperProps = {
  children: ReactNode;
};

export function PostHogProviderWrapper({
  children,
}: PostHogProviderWrapperProps) {
  // posthog is already initialized in instrumentation-client.ts
  // this provider just makes it available to React components

  if (!process.env.NEXT_PUBLIC_POSTHOG_KEY) {
    return <>{children}</>;
  }

  return <PostHogProvider client={posthog}>{children}</PostHogProvider>;
}
