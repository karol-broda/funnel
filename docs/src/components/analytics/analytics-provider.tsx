"use client";

import { createContext, useContext, ReactNode } from "react";
import { usePathname } from "next/navigation";
import { useAnalytics } from "@/hooks/use-analytics";

interface AnalyticsContextType {
  trackNavigation: (from: string, to: string) => void;
  trackFeatureUsage: (feature: string) => void;
  trackPerformance: (metric: string, value: number) => void;
}

const AnalyticsContext = createContext<AnalyticsContextType | null>(null);

export function AnalyticsProvider({ children }: { children: ReactNode }) {
  const pathname = usePathname();
  const { trackEvent } = useAnalytics();

  const trackNavigation = (from: string, to: string) => {
    trackEvent("navigation", { from, to, current_path: pathname });
  };

  const trackFeatureUsage = (feature: string) => {
    trackEvent("feature_used", {
      feature,
      page: pathname,
      timestamp: new Date().toISOString(),
    });
  };

  const trackPerformance = (metric: string, value: number) => {
    trackEvent("performance_metric", {
      metric,
      value,
      page: pathname,
    });
  };

  return (
    <AnalyticsContext.Provider
      value={{
        trackNavigation,
        trackFeatureUsage,
        trackPerformance,
      }}
    >
      {children}
    </AnalyticsContext.Provider>
  );
}

export function useEnhancedAnalytics() {
  const context = useContext(AnalyticsContext);
  if (!context) {
    throw new Error(
      "useEnhancedAnalytics must be used within AnalyticsProvider"
    );
  }
  return context;
}
