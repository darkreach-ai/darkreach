import { PageHero } from "@/components/page-hero";
import { Section } from "@/components/ui/section";
import { InitiativeCard } from "@/components/initiative-card";
import { WaitlistCTA } from "@/components/waitlist-cta";
import { ScrollAnimate } from "@/components/scroll-animate";
import { Sparkles, Dna, Cloud, Lock } from "lucide-react";
import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Research",
  description:
    "Research initiatives powered by darkreach: prime discovery, protein folding, climate models, and cryptographic research.",
};

export default function ResearchPage() {
  return (
    <>
      <PageHero
        eyebrow="Research"
        title="Solving the world's biggest problems"
        description="We build distributed compute infrastructure for computational challenges that push the boundaries of what's possible. One initiative at a time."
      />

      {/* Active Initiatives */}
      <Section secondary>
        <div className="mb-8">
          <p className="text-sm font-medium text-accent-purple uppercase tracking-wider mb-3">
            Active
          </p>
          <h2 className="text-3xl sm:text-4xl font-bold text-foreground">
            Live initiatives
          </h2>
        </div>

        <ScrollAnimate>
          <InitiativeCard
            icon={Sparkles}
            title="Prime Number Discovery"
            description="Searching 12 special forms of prime numbers with deterministic proofs. The largest distributed prime search platform with AI-driven orchestration."
            status="active"
            stats={{
              Discoveries: "392K+",
              Forms: "12",
              Verified: "100%",
            }}
            href="/research/primes"
          />
        </ScrollAnimate>
      </Section>

      {/* Coming Soon */}
      <Section>
        <div className="mb-8">
          <p className="text-sm font-medium text-muted-foreground uppercase tracking-wider mb-3">
            Coming Soon
          </p>
          <h2 className="text-3xl sm:text-4xl font-bold text-foreground">
            Future initiatives
          </h2>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
          {[
            {
              icon: Dna,
              title: "Protein Folding Verification",
              description:
                "Distributed verification of protein structure predictions. Cross-check AlphaFold results across thousands of nodes.",
            },
            {
              icon: Cloud,
              title: "Climate Model Simulation",
              description:
                "Run high-resolution climate simulations across operator nodes. More compute, better predictions.",
            },
            {
              icon: Lock,
              title: "Cryptographic Research",
              description:
                "Stress-test post-quantum cryptographic algorithms with distributed brute-force analysis and lattice reduction.",
            },
          ].map((initiative, i) => (
            <ScrollAnimate key={initiative.title} delay={i * 100}>
              <InitiativeCard {...initiative} status="coming-soon" />
            </ScrollAnimate>
          ))}
        </div>
      </Section>

      {/* For Universities */}
      <Section secondary id="universities">
        <ScrollAnimate>
          <div className="max-w-2xl">
            <p className="text-sm font-medium text-accent-purple uppercase tracking-wider mb-3">
              For Universities
            </p>
            <h2 className="text-3xl sm:text-4xl font-bold text-foreground mb-4">
              Partner with us
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-4">
              We work with university research groups to provide distributed
              compute for computational experiments. If your department needs
              large-scale parallel computation, we can help.
            </p>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Current partners get priority access to the compute network,
              dedicated support, and co-authorship on resulting publications.
            </p>
            <a
              href="mailto:research@darkreach.ai"
              className="inline-flex items-center gap-2 text-accent-purple font-medium hover:underline"
            >
              Contact us &rarr;
            </a>
          </div>
        </ScrollAnimate>
      </Section>

      <WaitlistCTA />
    </>
  );
}
