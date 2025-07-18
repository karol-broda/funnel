import { buttonVariants } from "fumadocs-ui/components/ui/button";
import {
  MagnifyingGlassIcon,
  HouseIcon,
  ArrowLeftIcon,
} from "@phosphor-icons/react/ssr";
import Link from "next/link";
import { createMetadata } from "@/lib/seo";

export const metadata = createMetadata(
  "page not found",
  "this page went on vacation and forgot to leave a forwarding address",
  undefined,
  "/404"
);

export default function NotFound() {
  return (
    <div className="flex min-h-screen flex-col items-center justify-center px-4 text-center">
      <div className="max-w-md mx-auto">
        <div className="mb-8">
          <div className="text-9xl font-bold text-primary/20 mb-4">404</div>
          <div className="flex items-center justify-center gap-2 mb-4">
            <MagnifyingGlassIcon className="h-6 w-6 text-muted-foreground" />
            <span className="text-muted-foreground">searching...</span>
          </div>
        </div>

        <h1 className="text-3xl font-bold mb-4">well, this is awkward 😅</h1>

        <p className="text-lg text-muted-foreground mb-2">
          the page youre looking for decided to tunnel somewhere else
        </p>

        <p className="text-sm text-muted-foreground mb-8">
          {
            "maybe it's hiding behind a websocket connection? or perhaps it got lost in the void of my spaghetti code..."
          }
        </p>

        <div className="flex flex-col sm:flex-row gap-4 justify-center">
          <Link
            href="/"
            className={buttonVariants({
              color: "primary",
              className: "px-6 py-3 text-base font-semibold",
            })}
          >
            <HouseIcon className="h-4 w-4 mr-2" />
            back to safety
          </Link>

          <Link
            href="/docs"
            className={buttonVariants({
              color: "secondary",
              className: "px-6 py-3 text-base font-semibold",
            })}
          >
            <ArrowLeftIcon className="h-4 w-4 mr-2" />
            check the docs
          </Link>
        </div>

        <div className="mt-8 text-xs text-muted-foreground">
          <p>
            {"if you think this page should exist, it's probably my fault."}
          </p>
          <p>
            {"feel free to judge my routing skills on "}
            <a
              href="https://github.com/karol-broda/funnel"
              className="text-primary hover:underline"
              target="_blank"
              rel="noopener noreferrer"
            >
              github
            </a>
          </p>
        </div>
      </div>
    </div>
  );
}
