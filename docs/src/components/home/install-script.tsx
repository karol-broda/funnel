"use client";

import { useState, useCallback } from "react";
import { CopyIcon, CheckIcon } from "@phosphor-icons/react";
import { useDocumentationTracking } from "@/hooks/use-analytics";

export default function InstallScript() {
  const [isCopied, setIsCopied] = useState(false);
  const { trackInstallCommand, trackCodeCopy } = useDocumentationTracking();
  const installScript =
    "curl -LsSf https://raw.githubusercontent.com/karol-broda/funnel/master/scripts/install.sh | sh";

  const handleCopy = useCallback(() => {
    navigator.clipboard.writeText(installScript).then(() => {
      setIsCopied(true);
      setTimeout(() => setIsCopied(false), 2000);

      trackInstallCommand("curl_script");
      trackCodeCopy(installScript, "home_page_install");
    });
  }, [installScript, trackInstallCommand, trackCodeCopy]);

  return (
    <div className="group relative max-w-xl w-full">
      <pre className="w-full overflow-x-auto rounded-lg border-2 border-slate-200 bg-slate-50 p-4 pr-16 text-left text-slate-800 shadow-md dark:border-slate-700 dark:bg-slate-900 dark:text-slate-200">
        <code className="text-sm">
          curl -LsSf
          https://raw.githubusercontent.com/karol-broda/funnel/master/scripts/install.sh
          | sh
        </code>
      </pre>
      <button
        onClick={handleCopy}
        className="absolute right-4 top-1/2 -translate-y-1/2 rounded-lg bg-slate-700 p-2 text-slate-200 opacity-0 transition-all duration-200 group-hover:opacity-100 focus-visible:opacity-100 hover:bg-slate-600 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-slate-400 focus-visible:ring-offset-2 focus-visible:ring-offset-slate-900"
        aria-label="Copy install command"
      >
        {isCopied ? (
          <CheckIcon className="h-6 w-6 text-emerald-400" />
        ) : (
          <CopyIcon className="h-6 w-6" />
        )}
      </button>
    </div>
  );
}
