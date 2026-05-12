"use client";

import { motion } from "motion/react";
import { ease } from "@/lib/animation";

function Dot() {
  return <div className="h-2.5 w-2.5 rounded-full bg-fd-muted-foreground/20" />;
}

function OutputLine({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div>
      {"  "}
      <span className="inline-block w-24 text-fd-muted-foreground/60">
        {label}
      </span>
      {children}
    </div>
  );
}

export function Terminal() {
  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.8, delay: 0.8, ease }}
      className="relative"
    >
      <div className="absolute -inset-2 rounded-2xl bg-linear-to-r from-fd-primary/15 via-fd-primary/5 to-fd-primary/15 blur-xl" />
      <div className="relative overflow-hidden rounded-xl border border-fd-border/60 bg-fd-card/80 shadow-2xl shadow-black/10 backdrop-blur-md">
        <div className="flex items-center gap-2 border-b border-fd-border/50 px-4 py-3">
          <Dot />
          <Dot />
          <Dot />
          <span className="ml-2 text-xs text-fd-muted-foreground">
            terminal
          </span>
        </div>
        <div className="p-5 font-mono text-sm leading-relaxed">
          <div>
            <span className="text-fd-primary">$</span> funnel http 3000 --id
            my-app
          </div>
          <div className="mt-4 text-fd-muted-foreground">
            <div className="text-fd-foreground/50">funnel</div>
            <div className="mt-1">
              <OutputLine label="public url">
                <span className="text-fd-primary">
                  https://my-app.tunnel.example.com
                </span>
              </OutputLine>
              <OutputLine label="forwarding">localhost:3000</OutputLine>
              <OutputLine label="tunnel id">my-app</OutputLine>
            </div>
          </div>
        </div>
      </div>
    </motion.div>
  );
}
