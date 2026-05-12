"use client";

import Link from "next/link";
import { motion } from "motion/react";
import { FlowField } from "./_components/flow-field";
import { Terminal } from "./_components/terminal";
import { FlowDiagram } from "./_components/flow";
import { FeatureGrid } from "./_components/feature-grid";
import { GitHubIcon } from "./_components/github-icon";
import { heroButton } from "./_components/hero-button";
import { ease, fadeUp } from "@/lib/animation";

function ArrowRight({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
    >
      <path d="M5 12h14M12 5l7 7-7 7" />
    </svg>
  );
}

function Hero() {
  return (
    <section className="relative flex min-h-dvh items-center justify-center overflow-hidden bg-fd-background">
      <FlowField />

      <div className="relative z-10 mx-auto max-w-xl px-6 text-center">
        <motion.h1
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 1.2, ease }}
          className="font-display text-[clamp(4.5rem,14vw,9rem)] leading-none tracking-tight"
        >
          funnel
        </motion.h1>

        <motion.p
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 1, delay: 0.4 }}
          className="mt-4 text-sm uppercase tracking-[0.2em] text-fd-muted-foreground"
        >
          Self-hosted tunnels over QUIC
        </motion.p>

        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.8, delay: 0.7 }}
          className="mt-10 flex justify-center gap-3"
        >
          <Link
            href="/docs"
            className={heroButton({ variant: "primary", className: "group" })}
          >
            Docs
            <ArrowRight className="size-3 opacity-40 transition-transform group-hover:translate-x-0.5 group-hover:opacity-70" />
          </Link>
          <a
            href="https://github.com/karol-broda/funnel"
            className={heroButton({ variant: "secondary" })}
          >
            <GitHubIcon className="size-3.5" />
            GitHub
          </a>
        </motion.div>
      </div>

      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ delay: 1.5, duration: 1 }}
        className="absolute bottom-8 left-1/2 -translate-x-1/2"
      >
        <motion.div
          animate={{ y: [0, 6, 0] }}
          transition={{ duration: 2, repeat: Infinity, ease: "easeInOut" }}
          className="h-8 w-[1px] bg-gradient-to-b from-transparent to-fd-foreground/20"
        />
      </motion.div>
    </section>
  );
}

function HowItWorks() {
  return (
    <section className="bg-fd-background py-24 md:py-32">
      <div className="mx-auto max-w-2xl px-6">
        <motion.p
          {...fadeUp()}
          className="font-display text-lg text-fd-muted-foreground"
        >
          How it works
        </motion.p>
        <motion.h2
          {...fadeUp(0.05)}
          className="mt-2 font-display text-3xl tracking-tight md:text-4xl"
        >
          One connection, many streams
        </motion.h2>
        <motion.p
          {...fadeUp(0.1)}
          className="mt-3 max-w-xl text-sm leading-relaxed text-fd-muted-foreground"
        >
          The client maintains a single QUIC connection to your server. When a
          request hits your public URL, the server opens a new stream. No
          reconnection, no overhead, no blocking.
        </motion.p>
        <div className="mt-12">
          <FlowDiagram />
        </div>
      </div>
    </section>
  );
}

function TerminalSection() {
  return (
    <section className="border-y border-fd-border bg-fd-muted/30 py-24 md:py-32">
      <div className="mx-auto max-w-2xl px-6">
        <Terminal />
      </div>
    </section>
  );
}

function Features() {
  return (
    <section className="bg-fd-background py-24 md:py-32">
      <div className="mx-auto max-w-2xl px-6">
        <motion.p
          {...fadeUp()}
          className="font-display text-lg text-fd-muted-foreground"
        >
          Features
        </motion.p>
        <FeatureGrid />
      </div>
    </section>
  );
}

function CTA() {
  return (
    <section className="border-t border-fd-border bg-fd-background py-24 md:py-32">
      <div className="mx-auto max-w-md px-6 text-center">
        <motion.div {...fadeUp()}>
          <h2 className="font-display text-2xl tracking-tight">
            Get started
          </h2>
          <p className="mt-3 text-sm text-fd-muted-foreground">
            One command. No sign-up. No third-party.
          </p>
          <div className="mt-10 flex justify-center gap-3">
            <Link
              href="/docs"
              className={heroButton({ variant: "primary", className: "group" })}
            >
              Read the docs
              <ArrowRight className="size-3 transition-transform group-hover:translate-x-0.5" />
            </Link>
            <Link
              href="/docs/getting-started/quickstart"
              className={heroButton({ variant: "secondary" })}
            >
              Quick start
            </Link>
          </div>
        </motion.div>
      </div>
    </section>
  );
}

function Footer() {
  return (
    <footer className="border-t border-fd-border bg-fd-background py-6">
      <div className="mx-auto flex max-w-2xl items-center justify-between px-6 text-xs text-fd-muted-foreground">
        <span>MIT</span>
        <a
          href="https://github.com/karol-broda/funnel"
          className="transition-colors hover:text-fd-foreground"
          aria-label="GitHub"
        >
          <GitHubIcon className="size-3.5" />
        </a>
      </div>
    </footer>
  );
}

export default function HomePage() {
  return (
    <main>
      <Hero />
      <HowItWorks />
      <TerminalSection />
      <Features />
      <CTA />
      <Footer />
    </main>
  );
}
