"use client";

import { useState, useEffect } from "react";
import { XIcon } from "@phosphor-icons/react";
import { buttonVariants } from "fumadocs-ui/components/ui/button";
import Link from "next/link";

export function CookieBanner() {
  const [isVisible, setIsVisible] = useState(false);
  const CONSENT_KEY = "funnel-analytics-consent";

  useEffect(() => {
    const hasConsent = localStorage.getItem(CONSENT_KEY);

    if (hasConsent !== "accepted") {
      setIsVisible(true);
    }
  }, []);

  const handleAccept = () => {
    localStorage.setItem(CONSENT_KEY, "accepted");
    setIsVisible(false);

    if (
      typeof window !== "undefined" &&
      (window as unknown as { posthog?: any }).posthog
    ) {
      (window as unknown as { posthog?: any }).posthog.opt_in_capturing();
    }
  };

  const handleDecline = () => {
    setIsVisible(false);

    if (
      typeof window !== "undefined" &&
      (window as unknown as { posthog?: any }).posthog
    ) {
      (window as unknown as { posthog?: any }).posthog.opt_out_capturing();
    }
  };

  if (!isVisible) return null;

  return (
    <div className="fixed bottom-0 left-0 right-0 z-50 bg-background/95 backdrop-blur border-t border-border shadow-lg">
      <div className="container py-4">
        <div className="flex flex-col sm:flex-row items-start sm:items-center gap-4 text-sm">
          <div className="flex-1">
            <p className="font-medium mb-1">
              We use analytics to improve our documentation
            </p>
            <p className="text-muted-foreground">
              We collect anonymous usage data to understand how you interact
              with our docs and make them better. See our{" "}
              <Link href="/privacy" className="underline hover:no-underline">
                privacy policy
              </Link>{" "}
              for details. If you decline, we won't store anything at all.
            </p>
          </div>

          <div className="flex items-center gap-2">
            <button
              onClick={handleDecline}
              className={buttonVariants({
                color: "secondary",
                size: "sm",
              })}
            >
              Decline
            </button>
            <button
              onClick={handleAccept}
              className={buttonVariants({
                color: "primary",
                size: "sm",
              })}
            >
              Accept
            </button>
            <button
              onClick={handleDecline}
              className="p-1 hover:bg-muted rounded"
              aria-label="Close banner"
            >
              <XIcon className="h-4 w-4" />
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
