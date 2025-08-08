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
          handles your information when you use our documentation website at {siteConfig.url}.
        </p>

        <h2>Information We Collect</h2>

        <p>
          We do not collect personal information or use analytics tracking on this website.
        </p>

        <h3>Technical Information</h3>
        <p>
          Our hosting provider may automatically collect basic technical information in server logs:
        </p>
        <ul>
          <li>IP addresses (for security and performance purposes)</li>
          <li>HTTP requests and responses</li>
          <li>Browser user agent strings</li>
          <li>Timestamps of requests</li>
        </ul>

        <h2>How We Use Your Information</h2>

        <p>The minimal technical information collected is used only for:</p>
        <ul>
          <li>Ensuring website security and preventing abuse</li>
          <li>Maintaining site performance and availability</li>
          <li>Basic error monitoring and troubleshooting</li>
        </ul>

        <h2>Data Storage and Processing</h2>

        <p>
          Our website is hosted on cloud infrastructure. Server logs are retained
          temporarily for operational purposes and automatically deleted according
          to our hosting provider's policies.
        </p>

        <h2>Third-Party Services</h2>

        <h3>Hosting</h3>
        <p>
          Our website is hosted on cloud infrastructure. Server logs may contain
          basic technical information about requests as described above.
        </p>

        <h2>Your Rights</h2>

        <p>Since we don't collect personal data through analytics or tracking, there is minimal personal information to manage. However, you have the right to:</p>
        <ul>
          <li>Request information about any data that may be in server logs</li>
          <li>Contact us with privacy concerns</li>
        </ul>

        <h2>Cookies and Local Storage</h2>

        <p>We use minimal local storage only for:</p>
        <ul>
          <li>Theme preferences (dark/light mode)</li>
        </ul>

        <p>
          This data is stored locally on your device and is not transmitted to our servers.
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
          Since we do not collect personal data through analytics or tracking,
          minimal processing is based on legitimate interest for website operation
          and security.
        </p>
      </div>
    </div>
  );
}
