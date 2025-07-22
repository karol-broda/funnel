"use client";

import { useEffect } from "react";

import { useAnalytics } from "./use-analytics";

export function useRouteChangeTracking() {
  const { trackEvent } = useAnalytics();

  useEffect(() => {
    const handleRouteChange = () => {
      trackEvent("route_change", {
        timestamp: new Date().toISOString(),
        user_agent: navigator.userAgent,
        screen_resolution: `${screen.width}x${screen.height}`,
      });
    };

    window.addEventListener("popstate", handleRouteChange);

    return () => {
      window.removeEventListener("popstate", handleRouteChange);
    };
  }, [trackEvent]);
}

export function useWebVitals() {
  const { trackEvent } = useAnalytics();

  useEffect(() => {
    if ("web-vitals" in window) {
      import("web-vitals").then(({ onCLS, onFID, onFCP, onLCP, onTTFB }) => {
        onCLS((metric) => trackEvent("web_vital_cls", { value: metric.value }));
        onFID((metric) => trackEvent("web_vital_fid", { value: metric.value }));
        onFCP((metric) => trackEvent("web_vital_fcp", { value: metric.value }));
        onLCP((metric) => trackEvent("web_vital_lcp", { value: metric.value }));
        onTTFB((metric) =>
          trackEvent("web_vital_ttfb", { value: metric.value })
        );
      });
    }
  }, [trackEvent]);
}

export function useNextjsFeatureTracking() {
  const { trackEvent } = useAnalytics();

  useEffect(() => {
    if ("serviceWorker" in navigator) {
      navigator.serviceWorker.ready.then(() => {
        trackEvent("feature_service_worker", { available: true });
      });
    }

    const observer = new PerformanceObserver((list) => {
      list.getEntries().forEach((entry) => {
        const resourceEntry = entry as PerformanceResourceTiming;
        if (resourceEntry.initiatorType === "prefetch") {
          trackEvent("nextjs_prefetch", {
            resource: entry.name,
            duration: entry.duration,
          });
        }
      });
    });
    observer.observe({ entryTypes: ["resource"] });

    return () => observer.disconnect();
  }, [trackEvent]);
}
