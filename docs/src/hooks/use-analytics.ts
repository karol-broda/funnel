"use client";

import { usePostHog } from "posthog-js/react";
import { usePathname, useSearchParams } from "next/navigation";
import { useEffect } from "react";

export function useAnalytics() {
  const posthog = usePostHog();

  const trackEvent = (eventName: string, properties?: Record<string, any>) => {
    if (posthog) {
      posthog.capture(eventName, properties);
    }
  };

  const trackPageView = (path?: string) => {
    if (posthog) {
      posthog.capture("$pageview", {
        $current_url: path || window.location.href,
      });
    }
  };

  const identifyUser = (userId: string, properties?: Record<string, any>) => {
    if (posthog) {
      posthog.identify(userId, properties);
    }
  };

  return {
    trackEvent,
    trackPageView,
    identifyUser,
    posthog,
  };
}

export function usePageTracking() {
  const { trackPageView } = useAnalytics();
  const pathname = usePathname();
  const searchParams = useSearchParams();

  useEffect(() => {
    if (pathname) {
      const url = searchParams ? `${pathname}?${searchParams}` : pathname;
      trackPageView(url);
    }
  }, [pathname, searchParams, trackPageView]);
}

// pre-defined tracking events
export function useDocumentationTracking() {
  const { trackEvent } = useAnalytics();

  const trackDocumentationView = (docPath: string, docTitle: string) => {
    trackEvent("documentation_viewed", {
      doc_path: docPath,
      doc_title: docTitle,
    });
  };

  const trackCodeCopy = (codeBlock: string, location: string) => {
    trackEvent("code_copied", {
      code_preview: codeBlock.slice(0, 100),
      location,
    });
  };

  const trackInstallCommand = (method: string) => {
    trackEvent("install_command_used", {
      installation_method: method,
    });
  };

  const trackExternalLink = (url: string, context: string) => {
    trackEvent("external_link_clicked", {
      destination_url: url,
      click_context: context,
    });
  };

  return {
    trackDocumentationView,
    trackCodeCopy,
    trackInstallCommand,
    trackExternalLink,
  };
}
