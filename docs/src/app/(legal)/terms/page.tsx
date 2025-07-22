import { createMetadata, siteConfig } from "@/lib/seo";
import Link from "next/link";

export const metadata = createMetadata(
  "Terms of Service",
  "Terms and conditions for using funnel and its documentation",
  undefined,
  "/terms"
);

export default function TermsPage() {
  const lastUpdated = "January 13, 2025";

  return (
    <div className="container max-w-4xl py-16">
      <div className="prose prose-gray dark:prose-invert max-w-none">
        <h1>Terms of Service</h1>

        <p className="text-muted-foreground">Last updated: {lastUpdated}</p>

        <p>
          These terms of service govern your use of the funnel documentation
          website and software. By using our service, you agree to these terms.
        </p>

        <h2>About Funnel</h2>

        <p>
          Funnel is an open-source tunneling solution built with Go. It allows
          you to expose local services to the internet through websocket
          connections.
        </p>

        <h2>Software License</h2>

        <p>
          Funnel is released under the MIT License. You can find the full
          license text in the{" "}
          <a
            href={`${siteConfig.githubUrl}/blob/master/LICENSE.md`}
            target="_blank"
            rel="noopener noreferrer"
          >
            GitHub repository
          </a>
          .
        </p>

        <h2>Acceptable Use</h2>

        <h3>Permitted Uses</h3>
        <ul>
          <li>Using funnel for development and testing purposes</li>
          <li>Self-hosting funnel servers</li>
          <li>Modifying and distributing funnel under the MIT license terms</li>
          <li>Using this documentation for learning and reference</li>
        </ul>

        <h3>Prohibited Uses</h3>
        <p>You may not use funnel to:</p>
        <ul>
          <li>Engage in illegal activities or violate applicable laws</li>
          <li>Circumvent security measures or access controls</li>
          <li>Distribute malware, spam, or harmful content</li>
          <li>Infringe on intellectual property rights</li>
          <li>Abuse or attack network infrastructure</li>
        </ul>

        <h2>Disclaimers</h2>

        <h3>Development Software</h3>
        <p>
          Funnel is primarily intended for development and testing purposes.
          While functional, it may not include all security features required
          for production environments.
        </p>

        <h3>No Warranty</h3>
        <p>
          The software is provided "as is" without any warranty. We make no
          guarantees about its reliability, security, or fitness for any
          particular purpose.
        </p>

        <h3>Use at Your Own Risk</h3>
        <p>
          You assume all risks associated with using funnel. We are not liable
          for any damages or losses resulting from its use.
        </p>

        <h2>Limitation of Liability</h2>

        <p>
          In no event shall the funnel developers be liable for any direct,
          indirect, incidental, special, or consequential damages arising from
          your use of the software or documentation.
        </p>

        <h2>Security Considerations</h2>

        <p>
          Funnel currently uses a simple security model where anyone who knows
          your server address can connect and create tunnels. This is suitable
          for development environments but requires additional protection for
          production use.
        </p>

        <h2>Data and Privacy</h2>

        <p>
          Your use of this website is also governed by our{" "}
          <a href="/privacy">Privacy Policy</a>. We collect minimal analytics
          data to improve our documentation.
        </p>

        <h2>Modifications</h2>

        <p>
          We reserve the right to modify these terms at any time. Continued use
          of the service after changes constitutes acceptance of the new terms.
        </p>

        <h2>Open Source</h2>

        <p>
          Funnel is open source software. Contributions, bug reports, and
          feature requests are welcome on our{" "}
          <a
            href={siteConfig.githubUrl}
            target="_blank"
            rel="noopener noreferrer"
          >
            GitHub repository
          </a>
          .
        </p>

        <h2>Contact</h2>

        <p>For questions about these terms or the software, please:</p>
        <ul>
          <li>
            Open an issue on{" "}
            <a
              href={siteConfig.githubUrl}
              target="_blank"
              rel="noopener noreferrer"
            >
              GitHub
            </a>
          </li>
          <li>
            Check the <Link href="/docs">documentation</Link> for technical
            questions
          </li>
        </ul>

        <h2>Governing Law</h2>

        <p>
          These terms are governed by the laws of the jurisdiction where the
          software is primarily developed and maintained.
        </p>
      </div>
    </div>
  );
}
