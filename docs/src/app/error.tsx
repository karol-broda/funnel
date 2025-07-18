"use client";

import { useEffect } from "react";
import { buttonVariants } from "fumadocs-ui/components/ui/button";
import {
  WarningIcon,
  ArrowClockwiseIcon,
  HouseIcon,
} from "@phosphor-icons/react/ssr";
import Link from "fumadocs-core/link";

export default function Error({
  error,
  reset,
}: {
  error: Error & { digest?: string };
  reset: () => void;
}) {
  useEffect(() => {
    console.error("page error:", error);
  }, [error]);

  return (
    <div className="flex min-h-screen flex-col items-center justify-center px-4 text-center">
      <div className="max-w-md mx-auto">
        <div className="mb-8">
          <div className="mx-auto mb-4 flex h-20 w-20 items-center justify-center rounded-full bg-destructive/10">
            <WarningIcon className="h-10 w-10 text-destructive" />
          </div>
        </div>

        <h1 className="text-3xl font-bold mb-4">oops, something broke 💥</h1>

        <p className="text-lg text-muted-foreground mb-2">
          {"looks like my code decided to throw a tantrum"}
        </p>

        <p className="text-sm text-muted-foreground mb-8">
          {"don't worry, it's probably not your fault. probably."}
        </p>

        {process.env.NODE_ENV === "development" && (
          <div className="mb-6 rounded-lg bg-destructive/10 p-4 text-left">
            <p className="text-sm font-semibold mb-2">debug info:</p>
            <code className="text-xs text-muted-foreground break-all">
              {error.message}
            </code>
            {error.digest && (
              <p className="text-xs text-muted-foreground mt-2">
                digest: {error.digest}
              </p>
            )}
          </div>
        )}

        <div className="flex flex-col sm:flex-row gap-4 justify-center">
          <button
            onClick={reset}
            className={buttonVariants({
              color: "primary",
              className: "px-6 py-3 text-base font-semibold",
            })}
          >
            <ArrowClockwiseIcon className="h-4 w-4 mr-2" />
            try again
          </button>

          <Link
            href="/"
            className={buttonVariants({
              color: "secondary",
              className: "px-6 py-3 text-base font-semibold",
            })}
          >
            <HouseIcon className="h-4 w-4 mr-2" />
            go home
          </Link>
        </div>

        <div className="mt-8 text-xs text-muted-foreground">
          <p>
            {"if this keeps happening, please roast me on "}
            <Link
              href="https://github.com/karol-broda/funnel/issues"
              className="text-primary hover:underline"
              target="_blank"
              rel="noopener noreferrer"
            >
              github issues
            </Link>
          </p>
        </div>
      </div>
    </div>
  );
}
