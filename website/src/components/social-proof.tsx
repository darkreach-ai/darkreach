import { Section } from "./ui/section";
import { ScrollAnimate } from "./scroll-animate";

const TECH_STACK = [
  {
    name: "Rust",
    description: "Systems language",
  },
  {
    name: "GMP",
    description: "Arbitrary precision",
  },
  {
    name: "PFGW",
    description: "Primality proving",
  },
  {
    name: "OEIS",
    description: "Sequence database",
  },
];

export function SocialProof() {
  return (
    <Section>
      <ScrollAnimate>
        <div className="text-center">
          <p className="text-sm font-medium text-muted-foreground uppercase tracking-wider mb-2">
            Standing on the shoulders of giants
          </p>
          <p className="text-xs text-muted-foreground/60 mb-10">
            Built with proven tools from the computational number theory community
          </p>
          <div className="flex flex-wrap items-center justify-center gap-4 sm:gap-6">
            {TECH_STACK.map((tech) => (
              <div
                key={tech.name}
                className="inline-flex items-center gap-2.5 px-5 py-2.5 rounded-full border border-border/60 bg-card/40 hover:border-border hover:bg-card/60 transition-colors"
              >
                <span className="text-sm font-semibold text-foreground">
                  {tech.name}
                </span>
                <span className="text-xs text-muted-foreground">
                  {tech.description}
                </span>
              </div>
            ))}
          </div>
        </div>
      </ScrollAnimate>
    </Section>
  );
}
