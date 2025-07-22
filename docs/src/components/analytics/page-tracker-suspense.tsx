"use client";

import { Suspense } from "react";
import { PageTracker } from "./page-tracker";

function PageTrackerFallback() {
  return null;
}

export function PageTrackerWithSuspense() {
  return (
    <Suspense fallback={<PageTrackerFallback />}>
      <PageTracker />
    </Suspense>
  );
}
