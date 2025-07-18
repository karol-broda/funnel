import { SpinnerIcon } from "@phosphor-icons/react/ssr";

export default function DocsLoading() {
  return (
    <div className="flex items-center justify-center min-h-[50vh] py-12">
      <div className="text-center">
        <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-full bg-primary/10">
          <SpinnerIcon className="h-6 w-6 text-primary animate-spin" />
        </div>

        <p className="text-sm text-muted-foreground">loading docs...</p>

        <div className="mt-4 flex justify-center items-center gap-1">
          <div
            className="h-1 w-1 bg-primary rounded-full animate-bounce"
            style={{ animationDelay: "0ms" }}
          />
          <div
            className="h-1 w-1 bg-primary rounded-full animate-bounce"
            style={{ animationDelay: "150ms" }}
          />
          <div
            className="h-1 w-1 bg-primary rounded-full animate-bounce"
            style={{ animationDelay: "300ms" }}
          />
        </div>
      </div>
    </div>
  );
}
