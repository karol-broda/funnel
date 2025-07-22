"use client";

import { useEffect, useState } from "react";
import { useAnalytics } from "@/hooks/use-analytics";

export function PrivacyAwareAnalytics() {
  const { posthog } = useAnalytics();
  const CONSENT_KEY = "funnel-analytics-consent";

  useEffect(() => {
    const checkConsent = () => {
      const consent = localStorage.getItem(CONSENT_KEY);
      const consentGiven = consent === "accepted";

      if (posthog) {
        if (consentGiven) {
          posthog.opt_in_capturing();
        } else {
          posthog.opt_out_capturing();
        }
      }
    };

    checkConsent();

    const handleStorageChange = (e: StorageEvent) => {
      if (e.key === CONSENT_KEY) {
        checkConsent();
      }
    };

    window.addEventListener("storage", handleStorageChange);
    return () => window.removeEventListener("storage", handleStorageChange);
  }, [posthog]);

  return null;
}
