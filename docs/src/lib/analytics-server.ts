import { headers } from "next/headers";
import { userAgent } from "next/server";

export async function getServerAnalyticsContext() {
  const headersList = await headers();
  const ua = userAgent({ headers: headersList });

  return {
    userAgent: ua.ua,
    browser: ua.browser.name,
    device: ua.device.type || "desktop",
    os: ua.os.name,
    referrer: headersList.get("referer"),
    ip: headersList.get("x-forwarded-for") || headersList.get("x-real-ip"),
  };
}

export async function trackServerEvent(
  eventName: string,
  properties: Record<string, string | number | boolean | null>
) {
  "use server";

  if (!process.env.NEXT_PUBLIC_POSTHOG_KEY) return;

  const context = await getServerAnalyticsContext();

  console.log("Server event:", {
    event: eventName,
    properties: { ...properties, ...context },
  });
}
