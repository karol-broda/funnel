import { buttonVariants } from 'fumadocs-ui/components/ui/button';
import {
  SparkleIcon,
  GlobeIcon,
  ShieldCheckIcon,
  TerminalIcon,
  HardDrivesIcon,
  LightningIcon,
} from '@phosphor-icons/react/ssr';
import Link from 'next/link';
import Mermaid from '@/components/mdx/mermaid';
import Aurora from '@/components/ui/aurora';
import InstallScript from '@/components/home/install-script';

const architecture = `
graph LR
    A["local app<br/>:3000"] --> B["funnel client"]
    B -.websocket.-> C["funnel server<br/>:8080"]
    D["browser/curl"] --> E["tunnel-id.localhost:8080"]
    E --> C
    C -.-> B
    B --> A
    
    style A fill:#b7bdf8,color:#24273a
    style B fill:#f5bde6,color:#24273a
    style C fill:#8bd5ca,color:#24273a
    style D fill:#a6da95,color:#24273a
    style E fill:#91d7e3,color:#24273a
`;

export default function HomePage() {
  return (
    <>
      <main className="relative py-32 md:py-48">
        <div className="absolute inset-0 z-[-1] overflow-hidden">
          <Aurora colorStops={['#1D4ED8', '#7C3AED', '#DB2777']} />
        </div>
        <div className="container relative flex flex-col items-center text-center">
          <h1 className="mb-4 text-5xl font-bold tracking-tight">
            expose local services to the world
          </h1>

          <p className="mb-8 max-w-2xl text-lg text-muted-foreground">
            funnel is a lightweight, self-hostable tunneling solution that
            exposes local services to the internet through secure websocket
            connections. perfect for development, testing, and sharing your
            work.
          </p>

          <InstallScript />

          <div className="mt-8 flex gap-4">
            <Link
              href="/docs"
              className={buttonVariants({
                color: 'primary',
                className: 'px-6 py-3 text-base font-semibold',
              })}
            >
              get started
            </Link>
            <a
              href="https://github.com/karol-broda/funnel"
              target="_blank"
              rel="noreferrer noopener"
              className={buttonVariants({
                color: 'secondary',
                className: 'px-6 py-3 text-base font-semibold',
              })}
            >
              view on github
            </a>
          </div>
        </div>
      </main>

      <section className="py-24">
        <div className="container">
          <div className="text-center">
            <h2 className="mb-4 text-3xl font-bold tracking-tight">
              powerful features, minimal setup
            </h2>
            <p className="mx-auto max-w-2xl text-lg text-muted-foreground">
              everything you need to securely expose your local services
              without the bloat.
            </p>
          </div>

          <div className="mt-12 grid grid-cols-1 gap-8 md:grid-cols-2 lg:grid-cols-3">
            <FeatureCard
              icon={<TerminalIcon className="h-6 w-6" weight="bold" />}
              title="simple cli"
              description="get started in minutes with a straightforward and easy-to-use command-line interface."
            />
            <FeatureCard
              icon={<ShieldCheckIcon className="h-6 w-6" weight="bold" />}
              title="secure by design"
              description="built-in tls support with let's encrypt for secure, end-to-end encrypted tunnels."
            />
            <FeatureCard
              icon={<SparkleIcon className="h-6 w-6" weight="bold" />}
              title="fast & lightweight"
              description="a single, small binary with minimal resource consumption, written in pure go."
            />
            <FeatureCard
              icon={<GlobeIcon className="h-6 w-6" weight="bold" />}
              title="cross-platform"
              description="builds for linux, macos, and windows, supporting both amd64 and arm64 architectures."
            />
            <FeatureCard
              icon={<HardDrivesIcon className="h-6 w-6" weight="bold" />}
              title="self-hostable"
              description="full control over your infrastructure. run the funnel server on your own hardware or cloud provider."
            />
            <FeatureCard
              icon={<LightningIcon className="h-6 w-6" weight="bold" />}
              title="auto-reconnection"
              description="client automatically re-establishes dropped connections with exponential backoff."
            />
          </div>
        </div>
      </section>

      <section className="bg-muted/30 py-24 text-center">
        <div className="container">
          <h2 className="mb-4 text-3xl font-bold tracking-tight">
            get started in 3 steps
          </h2>
          <div className="mt-8 flex flex-col items-stretch justify-center gap-8 md:flex-row">
            <StepCard
              step="1"
              title="run your local app"
              code="python3 -m http.server 3000"
            />
            <StepCard
              step="2"
              title="start the funnel server"
              code="./bin/funnel-server"
            />
            <StepCard
              step="3"
              title="connect your client"
              code="funnel http 3000 --id demo"
            />
          </div>
        </div>
      </section>

      <section className="py-24 text-center">
        <div className="container">
          <h2 className="mb-4 text-3xl font-bold tracking-tight">
            how it works
          </h2>
          <p className="mx-auto mb-12 max-w-2xl text-lg text-muted-foreground">
            funnel uses a client-server model to create a secure tunnel between
            your local machine and a public server. all communication is
            handled over a persistent websocket connection.
          </p>
          <div className="mx-auto w-full max-w-2xl">
            <Mermaid chart={architecture} />
          </div>
        </div>
      </section>

      <section className="py-32 text-center">
        <div className="container">
          <h2 className="text-4xl font-bold tracking-tight">
            ready to start tunneling?
          </h2>
          <p className="mx-auto mt-4 max-w-2xl text-lg text-muted-foreground">
            get up and running in minutes. it's free and open-source.
          </p>
          <div className="mt-8 flex justify-center gap-4">
            <Link
              href="/docs"
              className={buttonVariants({
                color: 'primary',
                className: 'px-8 py-4 text-lg font-semibold',
              })}
            >
              quickstart guide
            </Link>
          </div>
        </div>
      </section>
    </>
  );
}

function FeatureCard({
  icon,
  title,
  description,
}: {
  icon: React.ReactNode;
  title: string;
  description: string;
}) {
  return (
    <div className="flex flex-col gap-4 rounded-lg border bg-card p-6 text-left transition-transform duration-200 hover:scale-105 hover:shadow-lg">
      <div className="self-start rounded-md bg-secondary p-3 text-secondary-foreground">
        {icon}
      </div>
      <h3 className="text-lg font-semibold">{title}</h3>
      <p className="text-muted-foreground">{description}</p>
    </div>
  );
}

function StepCard({
  step,
  title,
  code,
}: {
  step: string;
  title: string;
  code: string;
}) {
  return (
    <div className="flex-1 rounded-lg border bg-card p-6 text-left transition-transform duration-200 hover:scale-105 hover:shadow-lg">
      <div className="flex items-center gap-4">
        <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-primary font-bold text-primary-foreground">
          {step}
        </div>
        <h3 className="text-lg font-semibold">{title}</h3>
      </div>
      <pre className="mt-4 w-full rounded-md bg-background p-4">
        <code className="text-sm">{code}</code>
      </pre>
    </div>
  );
}
