"use client";

import dynamic from "next/dynamic";
import { HeroLogo } from "./hero-logo";
import { WaitlistForm } from "./waitlist-form";
import { ArrowDown } from "lucide-react";

const NodeNetwork = dynamic(
  () => import("./node-network").then((m) => m.NodeNetwork),
  { ssr: false }
);

export function Hero() {
  return (
    <section className="relative min-h-[calc(100vh-4rem)] flex flex-col items-center justify-center bg-background overflow-hidden">
      {/* Three.js node network background */}
      <NodeNetwork />

      {/* Ambient glow orb */}
      <div
        className="absolute top-1/3 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[800px] h-[600px] rounded-full pointer-events-none z-[1]"
        style={{
          background:
            "radial-gradient(circle, rgba(99,102,241,0.10) 0%, rgba(99,102,241,0.03) 50%, transparent 70%)",
        }}
      />

      <div className="relative z-10 text-center px-6 max-w-4xl">
        <div className="flex justify-center mb-10">
          <HeroLogo size={80} />
        </div>

        <h1 className="text-5xl sm:text-6xl lg:text-8xl font-bold tracking-[-0.04em] text-foreground mb-6 leading-[1.05]">
          The world&apos;s biggest
          <br />
          <span className="gradient-text">supercomputer.</span>
        </h1>

        <p className="text-lg sm:text-xl text-muted-foreground/80 max-w-2xl mx-auto mb-10">
          A distributed computing platform where researchers define problems,
          operators contribute compute, and AI orchestrates discovery.
        </p>

        <div id="waitlist" className="flex justify-center mb-8">
          <WaitlistForm variant="inline" />
        </div>

        <a
          href="#how-it-works"
          className="inline-flex items-center gap-2 text-sm text-muted-foreground hover:text-foreground transition-colors"
        >
          Learn how it works
          <ArrowDown size={14} />
        </a>
      </div>
    </section>
  );
}
