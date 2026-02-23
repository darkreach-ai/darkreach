import { PageHero } from "@/components/page-hero";
import { Section } from "@/components/ui/section";
import { Card } from "@/components/ui/card";
import { PrimeForms } from "@/components/prime-forms";
import { Pipeline } from "@/components/pipeline";
import { LiveFeed } from "@/components/live-feed";
import { Comparison } from "@/components/comparison";
import { ScrollAnimate } from "@/components/scroll-animate";
import { Lock, HelpCircle, Gem, ShieldCheck } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "Prime Number Discovery",
  description:
    "Searching 12 special forms of prime numbers with deterministic proofs across a distributed compute network.",
};

const whyPrimes = [
  {
    icon: Lock,
    title: "Cryptography",
    description:
      "Large primes underpin RSA, Diffie-Hellman, and elliptic curve cryptography. Discovering new primes pushes the boundary of what's computationally feasible.",
    accent: "from-indigo-500 to-violet-500",
  },
  {
    icon: HelpCircle,
    title: "Unsolved Conjectures",
    description:
      "Twin prime conjecture, Goldbach's conjecture, and the distribution of primes remain open. Computational evidence drives mathematical progress.",
    accent: "from-violet-500 to-purple-500",
  },
  {
    icon: Gem,
    title: "Mathematical Beauty",
    description:
      "Special-form primes — factorial, palindromic, Sophie Germain — reveal deep structure in number theory that pure theory can't yet explain.",
    accent: "from-emerald-500 to-teal-500",
  },
  {
    icon: ShieldCheck,
    title: "Verification Challenge",
    description:
      "Proving a number prime requires deterministic certificates. Our 3-tier pipeline guarantees every discovery is independently verifiable.",
    accent: "from-amber-500 to-orange-500",
  },
];

export default function PrimesResearchPage() {
  return (
    <>
      <PageHero
        eyebrow="Research Initiative"
        title="Prime Number Discovery"
        description="Searching 12 special forms of prime numbers with deterministic proofs. Every candidate sieved, tested, and proven across a distributed compute network."
      />

      {/* Why Primes Matter */}
      <Section secondary>
        <div className="text-center mb-16">
          <h2 className="text-3xl sm:text-4xl font-bold text-foreground mb-4">
            Why primes matter
          </h2>
          <p className="text-lg text-muted-foreground max-w-2xl mx-auto">
            Prime numbers are the atoms of mathematics. Their study drives
            breakthroughs in security, computation, and pure mathematics.
          </p>
        </div>

        <div className="grid grid-cols-1 sm:grid-cols-2 gap-6">
          {whyPrimes.map((item, i) => (
            <ScrollAnimate key={item.title} delay={i * 80}>
              <Card hover className="h-full">
                <div
                  className={`w-10 h-10 rounded-lg bg-gradient-to-br ${item.accent} flex items-center justify-center text-white mb-4`}
                >
                  <item.icon size={20} />
                </div>
                <h3 className="text-foreground font-semibold mb-2">
                  {item.title}
                </h3>
                <p className="text-sm text-muted-foreground leading-relaxed">
                  {item.description}
                </p>
              </Card>
            </ScrollAnimate>
          ))}
        </div>
      </Section>

      {/* 12 Search Forms */}
      <PrimeForms />

      {/* Discovery Pipeline */}
      <Pipeline />

      {/* Live Discoveries */}
      <LiveFeed />

      {/* Comparison */}
      <Comparison />

      {/* CTA */}
      <Section>
        <ScrollAnimate>
          <div className="text-center">
            <h2 className="text-2xl font-bold text-foreground mb-4">
              Ready to contribute?
            </h2>
            <p className="text-muted-foreground mb-8 max-w-lg mx-auto">
              Join the network as an operator and help discover the next prime,
              or explore the complete documentation for all 12 search forms.
            </p>
            <div className="flex flex-col sm:flex-row items-center justify-center gap-3">
              <Button variant="primary" href="/operators">
                Become an operator
              </Button>
              <Button variant="outline" href="/docs/prime-forms">
                View all 12 forms
              </Button>
            </div>
          </div>
        </ScrollAnimate>
      </Section>
    </>
  );
}
