import { Globe, Brain, ShieldCheck } from "lucide-react";
import { Section } from "./ui/section";
import { ScrollAnimate } from "./scroll-animate";

const cards = [
  {
    icon: Globe,
    title: "Massive Scale",
    description:
      "Aggregate compute from thousands of machines worldwide into a single, coherent supercomputer. Problems that once took years can be solved in days.",
  },
  {
    icon: Brain,
    title: "AI-Orchestrated",
    description:
      "An AI engine continuously evaluates which problems to attack, how to distribute work, and when to pivot — maximizing discovery yield per compute hour.",
  },
  {
    icon: ShieldCheck,
    title: "Open & Verifiable",
    description:
      "Every result is cryptographically verified through a multi-stage proof pipeline. Open source, auditable, and MIT licensed.",
  },
];

export function VisionCards() {
  return (
    <Section>
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {cards.map((card, i) => (
          <ScrollAnimate key={card.title} delay={i * 100}>
            <div className="card-glow rounded-2xl border border-border bg-card p-8 sm:p-10 group">
              <div className="w-12 h-12 rounded-xl bg-accent-purple/10 text-accent-purple flex items-center justify-center mb-6 transition-transform duration-300 group-hover:scale-110">
                <card.icon size={24} />
              </div>
              <h3 className="text-xl font-semibold text-foreground mb-3">
                {card.title}
              </h3>
              <p className="text-muted-foreground leading-relaxed">
                {card.description}
              </p>
            </div>
          </ScrollAnimate>
        ))}
      </div>
    </Section>
  );
}
