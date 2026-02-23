import { PageHero } from "@/components/page-hero";
import { Section } from "@/components/ui/section";
import { Card } from "@/components/ui/card";
import { Pipeline } from "@/components/pipeline";
import { WaitlistCTA } from "@/components/waitlist-cta";
import { ScrollAnimate } from "@/components/scroll-animate";
import {
  Users,
  Brain,
  Network,
  Cpu,
  ShieldCheck,
  RotateCcw,
  BarChart3,
  Zap,
} from "lucide-react";
import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Platform",
  description:
    "The 5-layer architecture behind darkreach: from researchers to verified proofs.",
};

const layers = [
  {
    icon: Users,
    number: "01",
    title: "Researchers",
    subtitle: "Define problems",
    description:
      "Define research initiatives, configure search parameters, and set verification requirements.",
    accent: "from-indigo-500 to-violet-500",
  },
  {
    icon: Brain,
    number: "02",
    title: "AI Engine",
    subtitle: "Orchestrate",
    description:
      "OODA decision loop observes network state, scores opportunities across 10 dimensions, and allocates compute.",
    accent: "from-violet-500 to-purple-500",
  },
  {
    icon: Network,
    number: "03",
    title: "Coordinator",
    subtitle: "Distribute",
    description:
      "Generates work blocks, distributes to nodes via PostgreSQL row-level locking, tracks progress.",
    accent: "from-purple-500 to-pink-500",
  },
  {
    icon: Cpu,
    number: "04",
    title: "Nodes",
    subtitle: "Compute",
    description:
      "Operator nodes claim work blocks, run sieve and primality tests in parallel using GMP and PFGW.",
    accent: "from-emerald-500 to-teal-500",
  },
  {
    icon: ShieldCheck,
    number: "05",
    title: "Verification",
    subtitle: "Prove",
    description:
      "3-tier verification: deterministic proofs (Pocklington/Morrison/BLS), BPSW+MR, and PFGW confirmation.",
    accent: "from-amber-500 to-orange-500",
  },
];

const aiFeatures = [
  {
    icon: RotateCcw,
    title: "OODA Decision Loop",
    description:
      "Observe network state, Orient with scoring model, Decide on resource allocation, Act by dispatching work, Learn from outcomes.",
    accent: "from-indigo-500 to-violet-500",
  },
  {
    icon: BarChart3,
    title: "10-Component Scoring",
    description:
      "Record gap, yield rate, cost efficiency, opportunity density, network fit, momentum, competition, GPU fit, storage fit, network locality.",
    accent: "from-violet-500 to-purple-500",
  },
  {
    icon: Zap,
    title: "Autonomous Learning",
    description:
      "Online gradient descent on scoring weights, drift detection between snapshots, budget gates and stall penalties for safety.",
    accent: "from-emerald-500 to-teal-500",
  },
];

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

export default function PlatformPage() {
  return (
    <>
      <PageHero
        eyebrow="Platform"
        title="One platform. Unlimited compute."
        description="A 5-layer architecture that connects researchers, AI orchestration, distributed compute, and mathematical verification into a single platform."
      />

      {/* Architecture Overview */}
      <Section secondary>
        <div className="text-center mb-16">
          <p className="text-sm font-medium text-accent-purple uppercase tracking-wider mb-3">
            Architecture
          </p>
          <h2 className="text-3xl sm:text-4xl font-bold text-foreground mb-4">
            How it all fits together
          </h2>
          <p className="text-lg text-muted-foreground max-w-2xl mx-auto">
            Five layers, each with a clear responsibility. Problems flow down,
            results flow up.
          </p>
        </div>

        <div className="max-w-2xl mx-auto space-y-0">
          {layers.map((layer, i) => (
            <ScrollAnimate key={layer.title} delay={i * 80}>
              <div className="flex gap-5">
                {/* Vertical connector */}
                <div className="flex flex-col items-center">
                  <div
                    className={`w-10 h-10 rounded-xl bg-gradient-to-br ${layer.accent} flex items-center justify-center text-white shrink-0`}
                  >
                    <layer.icon size={20} />
                  </div>
                  {i < layers.length - 1 && (
                    <div className="w-px flex-1 bg-border min-h-[24px]" />
                  )}
                </div>

                {/* Content */}
                <div className="pb-8">
                  <div className="flex items-baseline gap-2">
                    <span className="text-xs font-mono text-muted-foreground/50">
                      {layer.number}
                    </span>
                    <h3 className="text-lg font-semibold text-foreground">
                      {layer.title}
                    </h3>
                    <span className="text-xs text-muted-foreground">
                      {layer.subtitle}
                    </span>
                  </div>
                  <p className="text-sm text-muted-foreground mt-1 leading-relaxed">
                    {layer.description}
                  </p>
                </div>
              </div>
            </ScrollAnimate>
          ))}
        </div>
      </Section>

      {/* AI Engine */}
      <Section>
        <div className="text-center mb-16">
          <p className="text-sm font-medium text-accent-purple uppercase tracking-wider mb-3">
            AI-orchestrated intelligence
          </p>
          <h2 className="text-3xl sm:text-4xl font-bold text-foreground mb-4">
            The AI Engine
          </h2>
          <p className="text-lg text-muted-foreground max-w-2xl mx-auto">
            A unified decision loop that replaces manual strategy with
            autonomous, data-driven orchestration.
          </p>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          {aiFeatures.map((feature, i) => (
            <ScrollAnimate key={feature.title} delay={i * 100}>
              <Card hover className="h-full">
                <div
                  className={`w-10 h-10 rounded-lg bg-gradient-to-br ${feature.accent} flex items-center justify-center text-white mb-4`}
                >
                  <feature.icon size={20} />
                </div>
                <h3 className="text-foreground font-semibold mb-2">
                  {feature.title}
                </h3>
                <p className="text-sm text-muted-foreground leading-relaxed">
                  {feature.description}
                </p>
              </Card>
            </ScrollAnimate>
          ))}
        </div>
      </Section>

      {/* Compute Pipeline */}
      <Pipeline />

      {/* Technology Stack */}
      <Section>
        <div className="text-center mb-16">
          <p className="text-sm font-medium text-accent-purple uppercase tracking-wider mb-3">
            Built with
          </p>
          <h2 className="text-3xl sm:text-4xl font-bold text-foreground mb-4">
            Technology Stack
          </h2>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
          {techStack.map((tech, i) => (
            <ScrollAnimate key={tech.name} delay={i * 60}>
              <Card hover className="h-full">
                <h3 className="text-foreground font-semibold mb-2">
                  {tech.name}
                </h3>
                <p className="text-sm text-muted-foreground">
                  {tech.description}
                </p>
              </Card>
            </ScrollAnimate>
          ))}
        </div>
      </Section>

      <WaitlistCTA />
    </>
  );
}
