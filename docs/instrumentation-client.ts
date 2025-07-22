import posthog from "posthog-js";

if (typeof window !== "undefined") {
  const apiKey = process.env.NEXT_PUBLIC_POSTHOG_KEY;
  const CONSENT_KEY = "funnel-analytics-consent";

  if (apiKey) {
    const consent = localStorage.getItem(CONSENT_KEY);
    const hasAccepted = consent === "accepted";

    posthog.init(apiKey, {
      api_host: "/ingest",
      ui_host: "https://eu.posthog.com",
      person_profiles: "identified_only",
      capture_pageview: false,
      capture_pageleave: true,
      debug: process.env.NODE_ENV === "development",
      session_recording: {
        recordCrossOriginIframes: true,
      },
      autocapture: {
        css_selector_allowlist: ["[data-track]", "button", "a[href]"],
        dom_event_allowlist: ["click", "submit"],
      },
      opt_out_capturing_by_default: true,
    });

    if (hasAccepted) {
      posthog.opt_in_capturing();
    }
  }
}
