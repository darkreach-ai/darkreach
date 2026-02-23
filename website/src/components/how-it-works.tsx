import { Section } from "./ui/section";
import { ScrollAnimate } from "./scroll-animate";

const steps = [
  {
    number: "01",
    title: "Researchers define problems",
    description:
      "Scientists and mathematicians submit computational challenges — from prime searches to protein folding — as structured projects with clear verification criteria.",
  },
  {
    number: "02",
    title: "Operators contribute compute",
    description:
      "Anyone can join the network by running a node. Contribute spare CPU cycles from a laptop, a rack of servers, or cloud instances — all with a single command.",
  },
  {
    number: "03",
    title: "AI orchestrates discovery",
    description:
      "Our AI engine evaluates problems, distributes work blocks, monitors progress, and adapts strategy in real time to maximize the rate of verified discoveries.",
  },
];

export function HowItWorks() {
  return (
    <Section secondary id="how-it-works">
      <div className="text-center mb-16">
        <p className="text-sm font-medium text-accent-purple uppercase tracking-wider mb-3">
          How it works
        </p>
        <h2 className="text-3xl sm:text-4xl font-bold text-foreground">
          Three roles, one network
        </h2>
      </div>

      <div className="relative grid grid-cols-1 md:grid-cols-3 gap-8">
        {/* Gradient connector lines (desktop only) */}
        <div className="hidden md:block step-connector left-[calc(16.666%+1.5rem)] right-[calc(66.666%+1.5rem)]" />
        <div className="hidden md:block step-connector left-[calc(50%+1.5rem)] right-[calc(16.666%+1.5rem)]" style={{ background: "linear-gradient(90deg, rgba(99,102,241,0.3), rgba(99,102,241,0.1))" }} />

        {steps.map((step, i) => (
          <ScrollAnimate key={step.number} delay={i * 120}>
            <div className="text-center group">
              <div className="relative w-12 h-12 mx-auto mb-6">
                {/* Outer glow ring */}
                <div className="absolute inset-0 rounded-full bg-accent-purple/20 scale-100 group-hover:scale-125 transition-transform duration-500" />
                {/* Inner circle */}
                <div className="relative w-12 h-12 rounded-full border-2 border-accent-purple/40 bg-background flex items-center justify-center group-hover:border-accent-purple/70 transition-colors duration-300">
                  <span className="text-sm font-semibold text-accent-purple">
                    {step.number}
                  </span>
                </div>
              </div>
              <h3 className="text-lg font-semibold text-foreground mb-3">
                {step.title}
              </h3>
              <p className="text-muted-foreground leading-relaxed max-w-sm mx-auto">
                {step.description}
              </p>
            </div>
          </ScrollAnimate>
        ))}
      </div>
    </Section>
  );
}
