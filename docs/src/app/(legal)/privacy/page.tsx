import { createMetadata, siteConfig } from "@/lib/seo";

export const metadata = createMetadata(
  "Privacy Policy",
  "How we collect, use, and protect your data when using funnel",
  undefined,
  "/privacy"
);

export default function PrivacyPage() {
  const lastUpdated = "January 13, 2025";

  return (
    <div className="container max-w-4xl py-16">
      <div className="prose prose-gray dark:prose-invert max-w-none">
        <h1>Privacy Policy</h1>

        <p className="text-muted-foreground">Last updated: {lastUpdated}</p>

        <p>
          This privacy policy explains how funnel ("we", "us", or "our")
          collects, uses, and protects your information when you use our
          documentation website at {siteConfig.url}.
        </p>

        <h2>Information We Collect</h2>

        <h3>Analytics Data</h3>
        <p>
          We use PostHog to collect analytics data to improve our documentation
          and understand how users interact with our site. This includes:
        </p>
        <ul>
          <li>
            <strong>Page views:</strong> Which pages you visit and when
          </li>
          <li>
            <strong>User interactions:</strong> Button clicks, link clicks, and
            form interactions
          </li>
          <li>
            <strong>Device information:</strong> Browser type, device type,
            screen resolution
          </li>
          <li>
            <strong>Performance metrics:</strong> Page load times and web vitals
          </li>
          <li>
            <strong>Usage patterns:</strong> How you navigate through our
            documentation
          </li>
        </ul>

        <h3>Technical Information</h3>
        <p>
          When you visit our site, we automatically collect certain technical
          information:
        </p>
        <ul>
          <li>IP address (anonymized)</li>
          <li>Browser type and version</li>
          <li>Operating system</li>
          <li>Referrer URL</li>
          <li>Session duration</li>
        </ul>

        <h2>How We Use Your Information</h2>

        <p>We use the collected information to:</p>
        <ul>
          <li>Improve our documentation and user experience</li>
          <li>Understand which content is most helpful</li>
          <li>Identify and fix technical issues</li>
          <li>Monitor site performance</li>
          <li>Generate anonymized usage statistics</li>
        </ul>

        <h2>Data Storage and Processing</h2>

        <p>
          Analytics data is processed by PostHog and stored in their EU servers.
          We proxy analytics requests through our own infrastructure to respect
          user privacy and avoid ad blockers.
        </p>

        <p>
          We do not store personal information beyond what's necessary for
          analytics. All data is anonymized and aggregated.
        </p>

        <h2>Third-Party Services</h2>

        <h3>PostHog</h3>
        <p>
          We use PostHog for analytics. PostHog's privacy policy can be found at{" "}
          <a
            href="https://posthog.com/privacy"
            target="_blank"
            rel="noopener noreferrer"
          >
            https://posthog.com/privacy
          </a>
        </p>

        <h3>Hosting</h3>
        <p>
          Our website is hosted on cloud infrastructure. Server logs may contain
          technical information about requests.
        </p>

        <h2>Your Rights</h2>

        <p>You have the right to:</p>
        <ul>
          <li>
            Opt out of analytics tracking by enabling "Do Not Track" in your
            browser
          </li>
          <li>Request information about data we've collected about you</li>
          <li>Request deletion of your data</li>
          <li>Object to data processing</li>
        </ul>

        <h2>Cookies and Local Storage</h2>

        <p>We use minimal cookies and local storage for:</p>
        <ul>
          <li>Theme preferences (dark/light mode)</li>
          <li>Analytics session tracking</li>
          <li>Performance monitoring</li>
        </ul>

        <p>
          You can disable cookies in your browser settings, though this may
          affect site functionality.
        </p>

        <h2>Data Retention</h2>

        <p>
          Analytics data is retained for a maximum of 2 years. Theme preferences
          are stored locally on your device until you clear your browser data.
        </p>

        <h2>International Data Transfers</h2>

        <p>
          Your data may be processed in countries outside your residence. We
          ensure appropriate safeguards are in place for international
          transfers.
        </p>

        <h2>Updates to This Policy</h2>

        <p>
          We may update this privacy policy from time to time. We will notify
          users of significant changes by updating the "Last updated" date
          above.
        </p>

        <h2>Contact</h2>

        <p>
          If you have questions about this privacy policy or want to exercise
          your rights, please contact us:
        </p>
        <ul>
          <li>
            GitHub:{" "}
            <a
              href={siteConfig.githubUrl}
              target="_blank"
              rel="noopener noreferrer"
            >
              {siteConfig.githubUrl}
            </a>
          </li>
          <li>Email: Create an issue on our GitHub repository</li>
        </ul>

        <h2>Legal Basis</h2>

        <p>
          Our legal basis for processing personal data is legitimate interest in
          improving our product and documentation, and your consent for
          analytics tracking.
        </p>
      </div>
    </div>
  );
}
