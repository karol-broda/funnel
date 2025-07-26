import { HeartIcon } from "@phosphor-icons/react/dist/ssr";
import Link from "next/link";

export function Footer() {
  return (
    <footer className="border-t bg-card/50 py-12">
      <div className="container flex flex-col items-center justify-center gap-4 text-center">
        <div className="flex flex-col sm:flex-row items-center justify-center gap-2 text-sm text-muted-foreground">
          <div className="flex items-center gap-4">
            <Link
              href="/privacy"
              className="hover:text-foreground transition-colors"
            >
              privacy policy
            </Link>
            <Link
              href="/terms"
              className="hover:text-foreground transition-colors"
            >
              terms of service
            </Link>
          </div>
          <span className="hidden sm:inline">•</span>
          <span>
            built with <HeartIcon className="inline-block" weight="fill" /> for
            developers.
          </span>
        </div>
        <div className="text-xs text-muted-foreground">
          © 2025 funnel. Open source tunneling solution.
        </div>
      </div>
    </footer>
  );
}
