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
              Privacy Policy
            </Link>
            <Link
              href="/terms"
              className="hover:text-foreground transition-colors"
            >
              Terms of Service
            </Link>
          </div>
          <span className="hidden sm:inline">•</span>
          <span>Built with ❤️ for developers</span>
        </div>
        <div className="text-xs text-muted-foreground">
          © 2025 funnel. Open source tunneling solution.
        </div>
      </div>
    </footer>
  );
}
