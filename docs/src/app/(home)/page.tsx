import { buttonVariants } from "fumadocs-ui/components/ui/button";
import {
  SparkleIcon,
  GlobeIcon,
  ShieldCheckIcon,
  TerminalIcon,
  HardDrivesIcon,
  LightningIcon,
  ArrowRightIcon,
} from "@phosphor-icons/react/ssr";
import Link from "next/link";
import Mermaid from "@/components/mdx/mermaid";
import Aurora from "@/components/ui/aurora";
import FeatureBento from "@/components/ui/feature-bento";
import InstallScript from "@/components/home/install-script";
import { createMetadata } from "@/lib/seo";
import { TrackedLink } from "@/components/analytics/tracked-link";

export const metadata = createMetadata(
  "funnel",
  "expose local services to the internet through websocket connections. perfect for development, testing, and demonstration purposes.",
  undefined,
  "/"
);

const architecture = `
graph LR
    A["your app<br/>chillin at :3000"] --> B["funnel client<br/>(the messenger)"]
    B -.websocket magic.-> C["funnel server<br/>vibing at :8080"]
    D["someone on<br/>the internet"] --> E["tunnel-id.localhost:8080<br/>(the secret door)"]
    E --> C
    C -.-> B
    B --> A
    
    style A fill:#b7bdf8,color:#24273a
    style B fill:#f5bde6,color:#24273a
    style C fill:#8bd5ca,color:#24273a
    style D fill:#a6da95,color:#24273a
    style E fill:#91d7e3,color:#24273a
`;

const funnelFeatures = [
  {
    icon: <TerminalIcon className="h-6 w-6" weight="bold" />,
    title: "dead simple cli",
    description:
      "if you can type 'funnel http 3000', you're basically a pro already. no phd required.",
    label: "Easy",
  },
  {
    icon: <ShieldCheckIcon className="h-6 w-6" weight="bold" />,
    title: "probably secure",
    description:
      "i slapped tls on it with let's encrypt. your data is safer than my git commit history.",
    label: "Safe",
  },
  {
    icon: <SparkleIcon className="h-6 w-6" weight="bold" />,
    title: "stupidly fast",
    description:
      "written in go because i heard it was fast. uses less ram than your spotify tab.",
    label: "Speed",
  },
  {
    icon: <GlobeIcon className="h-6 w-6" weight="bold" />,
    title: "works everywhere™",
    description:
      "linux? mac? windows? raspberry pi? if it runs go, it probably runs funnel. no promises though.",
    label: "Universal",
  },
  {
    icon: <HardDrivesIcon className="h-6 w-6" weight="bold" />,
    title: "host it yourself",
    description:
      "paranoid? control freak? same. run your own server and blame yourself when it breaks.",
    label: "Self-hosted",
  },
  {
    icon: <LightningIcon className="h-6 w-6" weight="bold" />,
    title: "never gives you up",
    description:
      "connection dropped? funnel reconnects like that ex who won't take a hint. but useful.",
    label: "Reliable",
  },
];

export default function HomePage() {
  return (
    <>
      <style
        dangerouslySetInnerHTML={{
          __html: `
          @keyframes shimmer {
            0% { transform: translateX(-100%); }
            100% { transform: translateX(100%); }
          }
          
          .animate-shimmer {
            animation: shimmer 1.5s ease-out;
          }
        `,
        }}
      />
      <main className="relative py-32 md:py-48">
        <div className="absolute inset-0 z-[-1] overflow-hidden opacity-60 dark:opacity-80">
          <Aurora colorStops={["#60A5FA", "#A78BFA", "#F472B6"]} />
        </div>
        <div className="container relative flex flex-col items-center text-center">
          <div className="mb-4 inline-flex items-center gap-2 rounded-full bg-white/10 backdrop-blur-sm border border-white/20 px-4 py-2 text-sm text-white">
            <SparkleIcon className="h-4 w-4" />
            <span>yet another tunneling tool nobody asked for</span>
          </div>

          <h1 className="mb-6 text-6xl font-bold tracking-tight text-white drop-shadow-2xl">
            localhost wants to see other people
          </h1>

          <div className="mb-8 max-w-2xl bg-black/10 backdrop-blur-sm rounded-2xl p-6">
            <p className="text-xl text-white font-medium mb-3">
              tired of {'"works on my machine"'}? yeah, me too.
              <span className="text-white font-bold">
                so i built this thing.
              </span>
            </p>

            <p className="text-base text-white/90">
              {
                "it's like those expensive tunneling tools but free and jankier."
              }{" "}
              <br />
              self-hosted, questionably secure, definitely overengineered.
            </p>
          </div>

          <InstallScript />

          <div className="mt-8 flex gap-4">
            <Link
              href="/docs"
              className={buttonVariants({
                color: "primary",
                className: "px-6 py-3 text-base font-semibold",
              })}
            >
              try it anyway <ArrowRightIcon weight="bold" />
            </Link>
            <TrackedLink
              href="https://github.com/karol-broda/funnel"
              target="_blank"
              rel="noreferrer noopener"
              context="home_page_github_button"
              className={buttonVariants({
                color: "secondary",
                className: "px-6 py-3 text-base font-semibold",
              })}
            >
              ⭐ judge my code
            </TrackedLink>
          </div>

          <div className="mt-12 flex flex-wrap justify-center items-center gap-4">
            <div className="flex items-center gap-2 bg-black/20 backdrop-blur-sm rounded-full px-3 py-1">
              <div className="h-2 w-2 rounded-full bg-green-400 animate-pulse" />
              <span className="text-sm text-white font-medium">
                works 90% of the time
              </span>
            </div>
            <div className="flex items-center gap-2 bg-black/20 backdrop-blur-sm rounded-full px-3 py-1">
              <div className="h-2 w-2 rounded-full bg-yellow-400 animate-pulse" />
              <span className="text-sm text-white font-medium">
                minimal bugs™
              </span>
            </div>
            <div className="flex items-center gap-2 bg-black/20 backdrop-blur-sm rounded-full px-3 py-1">
              <div className="h-2 w-2 rounded-full bg-purple-400 animate-pulse" />
              <span className="text-sm text-white font-medium">
                procrastination-powered
              </span>
            </div>
          </div>
        </div>
      </main>

      <section className="py-24">
        <div className="container">
          <div className="text-center">
            <h2 className="mb-4 text-3xl font-bold tracking-tight">
              why funnel is kinda awesome 😎
            </h2>
            <p className="mx-auto max-w-2xl text-lg text-muted-foreground">
              {
                "i built this because paying for tunneling services hurts my soul."
              }
              turns out, it works pretty well!
            </p>
          </div>

          <div className="mt-12 flex justify-center">
            <FeatureBento
              cards={funnelFeatures}
              enableStars={true}
              enableSpotlight={true}
              enableBorderGlow={true}
              enableTilt={false}
              enableMagnetism={false}
              clickEffect={true}
              glowColor="96, 165, 250"
              particleCount={6}
              spotlightRadius={300}
            />
          </div>
        </div>
      </section>

      <section className="bg-muted/30 py-24 text-center">
        <div className="container">
          <h2 className="mb-4 text-3xl font-bold tracking-tight">
            literally just 3 steps (i counted twice)
          </h2>
          <p className="mb-8 text-lg text-muted-foreground">
            easier than making instant noodles, with 73% less sodium
          </p>
          <div className="mt-8 flex flex-col items-stretch justify-center gap-8 md:flex-row">
            <StepCard
              step="1"
              title="run your janky app"
              code="python3 -m http.server 3000"
              subtitle={"or whatever mess you're building"}
            />
            <StepCard
              step="2"
              title="fire up the server"
              code="./bin/funnel-server"
              subtitle={"pray it doesn't crash"}
            />
            <StepCard
              step="3"
              title="connect & share"
              code="funnel http 3000 --id demo"
              subtitle={"watch people judge your css"}
            />
          </div>
        </div>
      </section>

      <section className="py-24 text-center">
        <div className="container">
          <h2 className="mb-4 text-3xl font-bold tracking-tight">
            the nerdy details (with pictures!)
          </h2>
          <p className="mx-auto mb-12 max-w-2xl text-lg text-muted-foreground">
            websockets go brrr... no seriously, thats basically it. i made this
            diagram to look smart:
          </p>
          <div className="mx-auto w-full max-w-2xl">
            <Mermaid chart={architecture} />
          </div>
          <p className="mt-8 text-sm text-muted-foreground">
            {
              "if this looks complicated, don't worry. i don't fully understand it either."
            }
          </p>
        </div>
      </section>

      <section className="py-32 text-center">
        <div className="container">
          <h2 className="text-4xl font-bold tracking-tight">
            {"still here? wow, you're persistent 🏆"}
          </h2>
          <p className="mx-auto mt-4 max-w-2xl text-lg text-muted-foreground">
            look, i spent way too many weekends on this thing. might as well
            give it a try. worst case, you waste 5 minutes.
          </p>
          <div className="mt-8 flex justify-center gap-4">
            <Link
              href="/docs"
              className={buttonVariants({
                color: "primary",
                className: "px-8 py-4 text-lg font-semibold",
              })}
            >
              {"fine, i'll try your tunnel thing"}
            </Link>
          </div>
          <p className="mt-6 text-sm text-muted-foreground">
            {"or don't. i'm not your boss. 🤷"}
          </p>
        </div>
      </section>
    </>
  );
}

function StepCard({
  step,
  title,
  code,
  subtitle,
}: {
  step: string;
  title: string;
  code: string;
  subtitle?: string;
}) {
  return (
    <div className="group relative overflow-hidden flex-1 rounded-lg border bg-card p-6 text-left transition-all duration-300 hover:scale-105 hover:shadow-xl hover:shadow-primary/20 hover:border-primary/50 hover:-translate-y-2">
      {/* shimmer effect */}
      <div className="absolute inset-0 -translate-x-full group-hover:animate-shimmer bg-gradient-to-r from-transparent via-white/5 to-transparent pointer-events-none" />
      <div className="flex items-center gap-4">
        <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full bg-primary font-bold text-primary-foreground transition-all duration-300 group-hover:scale-110 group-hover:rotate-12 group-hover:shadow-lg">
          {step}
        </div>
        <div>
          <h3 className="text-lg font-semibold transition-colors duration-300 group-hover:text-primary">
            {title}
          </h3>
          {subtitle && (
            <p className="text-sm text-muted-foreground transition-colors duration-300 group-hover:text-foreground">
              {subtitle}
            </p>
          )}
        </div>
      </div>
      <pre className="mt-4 w-full rounded-md bg-background p-4 transition-all duration-300 group-hover:bg-secondary/20 group-hover:border group-hover:border-primary/20">
        <code className="text-sm transition-colors duration-300 group-hover:text-primary">
          {code}
        </code>
      </pre>
    </div>
  );
}
