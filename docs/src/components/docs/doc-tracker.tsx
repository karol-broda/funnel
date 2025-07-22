"use client";

import { useEffect } from "react";
import { useDocumentationTracking } from "@/hooks/use-analytics";

interface DocTrackerProps {
  docPath: string;
  docTitle: string;
}

export function DocTracker({ docPath, docTitle }: DocTrackerProps) {
  const { trackDocumentationView } = useDocumentationTracking();

  useEffect(() => {
    trackDocumentationView(docPath, docTitle);
  }, [docPath, docTitle, trackDocumentationView]);

  return null;
}
