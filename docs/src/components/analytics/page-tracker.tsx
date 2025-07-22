"use client";

import { usePageTracking } from "@/hooks/use-analytics";

export function PageTracker() {
  usePageTracking();
  return null;
}
