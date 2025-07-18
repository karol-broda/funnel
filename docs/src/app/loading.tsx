import { SpinnerIcon } from "@phosphor-icons/react/ssr";

export default function Loading() {
  return (
    <div className="flex min-h-screen flex-col items-center justify-center px-4">
      <div className="max-w-md mx-auto text-center">
        <div className="mb-8">
          <div className="mx-auto mb-4 flex h-20 w-20 items-center justify-center rounded-full bg-primary/10">
            <SpinnerIcon className="h-10 w-10 text-primary animate-spin" />
          </div>
        </div>

        <h2 className="text-xl font-semibold mb-2">loading...</h2>

        <p className="text-sm text-muted-foreground">
          {"hold on, the hamsters are spinning the wheels"}
        </p>

        <div className="mt-6 flex justify-center items-center gap-1">
          <div
            className="h-2 w-2 bg-primary rounded-full animate-bounce"
            style={{ animationDelay: "0ms" }}
          />
          <div
            className="h-2 w-2 bg-primary rounded-full animate-bounce"
            style={{ animationDelay: "150ms" }}
          />
          <div
            className="h-2 w-2 bg-primary rounded-full animate-bounce"
            style={{ animationDelay: "300ms" }}
          />
        </div>
      </div>
    </div>
  );
}
