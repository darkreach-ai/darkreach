import { Section } from "@/components/ui/section";
import { Card } from "@/components/ui/card";
import { Timeline } from "@/components/timeline";
import { Github } from "lucide-react";
import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "About",
  description: "The mission, timeline, and technology behind darkreach.",
};

const techStack = [
  {
    name: "Rust",
    description:
      "Zero-cost abstractions, memory safety, and fearless concurrency for the engine and server.",
  },
  {
    name: "GMP (rug)",
    description:
      "GNU Multiple Precision Arithmetic — the gold standard for arbitrary-precision integer math.",
  },
  {
    name: "PFGW / GWNUM",
    description:
      "Specialized number theory software for 50-100x acceleration on large primality tests.",
  },
  {
    name: "PostgreSQL",
    description:
      "Relational database for primes, workers, jobs, and work distribution with row-level locking.",
  },
  {
    name: "Axum",
    description:
      "Async Rust web framework for the coordinator REST API and WebSocket server.",
  },
  {
    name: "Next.js",
    description:
      "React framework for the dashboard (app.darkreach.ai) and website (darkreach.ai).",
  },
];

export default function AboutPage() {
  return (
    <>
      <Section>
        <h1 className="text-4xl font-bold text-foreground mb-6">About darkreach</h1>
        <div className="max-w-3xl space-y-4 text-muted-foreground">
          <p>
            darkreach is an AI-driven distributed computing platform for
            scientific discovery. It combines autonomous AI agents with
            high-performance algorithms to research, optimize, and execute
            computational campaigns across a fleet of servers.
          </p>
          <p>
            Our current focus is prime number discovery — searching for 12
            special forms of prime numbers with deterministic proofs. But the
            architecture is general: the same agent-driven orchestration can
            tackle any embarrassingly parallel scientific computation.
          </p>
          <p>
            The project is fully open source under the MIT license. We believe
            mathematical discoveries should be independently verifiable, the
            tools should be available to everyone, and the code should serve as a
            teaching resource for computational number theory.
          </p>
        </div>
      </Section>

      <Section secondary>
        <h2 className="text-2xl font-bold text-foreground mb-8">Project Timeline</h2>
        <div className="max-w-2xl">
          <Timeline />
        </div>
      </Section>

      <Section>
        <h2 className="text-2xl font-bold text-foreground mb-8">Tech Stack</h2>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {techStack.map((tech) => (
            <Card key={tech.name}>
              <h3 className="text-foreground font-semibold mb-2">{tech.name}</h3>
              <p className="text-sm text-muted-foreground">{tech.description}</p>
            </Card>
          ))}
        </div>
      </Section>

      <Section secondary>
        <div className="text-center max-w-2xl mx-auto">
          <h2 className="text-2xl font-bold text-foreground mb-4">Open Source</h2>
          <p className="text-muted-foreground mb-6">
            darkreach is licensed under MIT. Every line of engine code, every
            proof algorithm, and every orchestration strategy is open for
            inspection, modification, and contribution.
          </p>
          <div className="flex flex-col sm:flex-row items-center justify-center gap-3">
            <a
              href="https://github.com/darkreach-ai/darkreach"
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-2 px-6 py-3 rounded-md border border-border text-foreground hover:border-text-muted transition-colors"
            >
              <Github size={20} />
              View on GitHub
            </a>
            <a
              href="https://discord.gg/2Khf4t8M33"
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-2 px-6 py-3 rounded-md border border-border text-foreground hover:border-text-muted transition-colors"
            >
              <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor"><path d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028c.462-.63.874-1.295 1.226-1.994a.076.076 0 0 0-.041-.106 13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.892.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.03zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z"/></svg>
              Join Discord
            </a>
          </div>
        </div>
      </Section>
    </>
  );
}
