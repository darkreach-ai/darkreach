import Link from "next/link";
import { Section } from "./ui/section";
import { ScrollAnimate } from "./scroll-animate";

const installCode = `# Install darkreach
curl -fsSL https://darkreach.ai/install.sh | sh

# Register as an operator
darkreach register --name "my-node"

# Start contributing compute
darkreach work --threads 4`;

export function OperatorRecruit() {
  return (
    <Section secondary>
      <ScrollAnimate>
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-12 items-center">
          <div>
            <p className="text-sm font-medium text-accent-purple uppercase tracking-wider mb-3">
              For operators
            </p>
            <h2 className="text-3xl sm:text-4xl font-bold text-foreground mb-4">
              Contribute your compute
            </h2>
            <p className="text-muted-foreground leading-relaxed mb-6">
              Join the network in under a minute. Run a node on any Linux or macOS
              machine — from a spare laptop to a rack of servers. The AI engine
              automatically assigns optimal workloads based on your hardware.
            </p>
            <Link
              href="/operators"
              className="text-sm font-medium text-accent-purple hover:underline"
            >
              Become an operator →
            </Link>
          </div>

          <div className="terminal-chrome">
            <div className="terminal-header">
              <span className="terminal-dot" style={{ background: "#f85149" }} />
              <span className="terminal-dot" style={{ background: "#f0883e" }} />
              <span className="terminal-dot" style={{ background: "#34d399" }} />
              <span className="terminal-title">Terminal</span>
            </div>
            <pre className="code-block">
              <code>{installCode}</code>
            </pre>
          </div>
        </div>
      </ScrollAnimate>
    </Section>
  );
}
