import { Section } from "./ui/section";
import Link from "next/link";
import { ArrowRight, Github, UserPlus, Cpu, BarChart3 } from "lucide-react";

const steps = [
  {
    icon: UserPlus,
    title: "Register",
    code: "darkreach register",
  },
  {
    icon: Cpu,
    title: "Connect",
    code: "darkreach run",
  },
  {
    icon: BarChart3,
    title: "Monitor",
    code: "app.darkreach.ai",
  },
];

export function CtaSection() {
  return (
    <Section className="cta-gradient">
      <div className="text-center max-w-3xl mx-auto">
        <p className="text-sm font-medium text-accent-purple uppercase tracking-wider mb-3">
          Get started
        </p>
        <h2 className="text-3xl sm:text-4xl font-bold text-foreground mb-4">
          Become an operator in three steps
        </h2>
        <p className="text-muted-foreground max-w-xl mx-auto mb-12">
          Register as an operator, connect your nodes to the network, and earn compute credits. MIT licensed.
        </p>

        {/* Three-step walkthrough */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4 mb-14">
          {steps.map((step, i) => (
            <div
              key={step.title}
              className="relative rounded-xl border border-border bg-card/60 backdrop-blur-sm p-5 text-left"
            >
              <div className="flex items-center gap-3 mb-3">
                <span className="flex items-center justify-center w-7 h-7 rounded-full bg-accent-purple/10 text-accent-purple text-xs font-bold border border-accent-purple/20">
                  {i + 1}
                </span>
                <span className="text-sm font-semibold text-foreground">
                  {step.title}
                </span>
              </div>
              <code className="block rounded-lg bg-background/80 border border-border px-3 py-2.5 font-mono text-[13px] text-accent-green truncate">
                {step.code}
              </code>
            </div>
          ))}
        </div>

        {/* CTA buttons */}
        <div className="flex flex-col sm:flex-row items-center justify-center gap-3">
          <Link
            href="/download"
            className="group inline-flex items-center gap-2 px-8 py-3.5 rounded-lg bg-accent-purple text-white font-medium hover:bg-accent-purple/90 transition-colors shadow-lg shadow-accent-purple/20 text-lg"
          >
            Start Hunting
            <ArrowRight size={18} className="group-hover:translate-x-0.5 transition-transform" />
          </Link>
          <a
            href="https://github.com/darkreach-ai/darkreach"
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-2 px-8 py-3.5 rounded-lg border border-border text-muted-foreground font-medium hover:text-foreground hover:border-muted-foreground/60 transition-colors text-lg"
          >
            <Github size={18} />
            View on GitHub
          </a>
          <a
            href="https://discord.gg/2Khf4t8M33"
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-2 px-8 py-3.5 rounded-lg border border-border text-muted-foreground font-medium hover:text-foreground hover:border-muted-foreground/60 transition-colors text-lg"
          >
            <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor"><path d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028c.462-.63.874-1.295 1.226-1.994a.076.076 0 0 0-.041-.106 13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.299 12.299 0 0 1-1.873.892.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.839 19.839 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.03zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z"/></svg>
            Join Discord
          </a>
        </div>
      </div>
    </Section>
  );
}
