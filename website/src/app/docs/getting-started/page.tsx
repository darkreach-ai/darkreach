"use client";

import { CodeBlock } from "@/components/ui/code-block";

export default function GettingStartedPage() {
  return (
    <div className="prose-docs">
      <h1>Getting Started</h1>
      <p>
        This guide walks you through installing darkreach, running your first
        prime search, joining the network, and viewing results in the dashboard.
      </p>

      <h2>Prerequisites</h2>
      <ul>
        <li>
          <strong>Rust</strong> 1.75 or later &mdash;{" "}
          <a href="https://rustup.rs">rustup.rs</a>
        </li>
        <li>
          <strong>GMP</strong> (GNU Multiple Precision Arithmetic Library)
        </li>
      </ul>

      <h3>macOS</h3>
      <CodeBlock language="bash">{"brew install gmp"}</CodeBlock>

      <h3>Linux (Debian/Ubuntu)</h3>
      <CodeBlock language="bash">
        {"sudo apt install build-essential libgmp-dev m4"}
      </CodeBlock>

      <h2>Build</h2>
      <CodeBlock language="bash">
        {`git clone https://github.com/darkreach-ai/darkreach.git
cd darkreach
cargo build --release`}
      </CodeBlock>
      <p>
        The binary will be at <code>./target/release/darkreach</code>.
      </p>

      <h2>Run Your First Search</h2>
      <p>
        Try a quick factorial prime search to verify everything works:
      </p>
      <CodeBlock language="bash">
        {"./target/release/darkreach factorial --start 1 --end 100"}
      </CodeBlock>
      <p>
        You should see output on stderr as candidates are sieved, filtered, and
        tested, with any primes logged to the console.
      </p>

      <h3>More search examples</h3>
      <CodeBlock language="bash">
        {`# Proth primes k·2^n+1
./target/release/darkreach kbn --k 3 --base 2 --min-n 1 --max-n 1000

# Palindromic primes in base 10
./target/release/darkreach palindromic --base 10 --min-digits 1 --max-digits 9

# Twin primes
./target/release/darkreach twin --k 3 --base 2 --min-n 1 --max-n 10000

# Sophie Germain primes
./target/release/darkreach sophie-germain --k 3 --base 2 --min-n 1 --max-n 10000

# Primorial primes
./target/release/darkreach primorial --start 1000 --end 50000`}
      </CodeBlock>

      <h2>Connect to a Database</h2>
      <p>
        To persist discoveries and see them in the dashboard, provide a
        PostgreSQL connection:
      </p>
      <CodeBlock language="bash">
        {`export DATABASE_URL="postgres://user:pass@localhost/darkreach"
./target/release/darkreach factorial --start 1000 --end 5000`}
      </CodeBlock>
      <p>
        Results will be stored in the <code>primes</code> table and visible in
        the{" "}
        <a href="https://app.darkreach.ai">dashboard</a>.
      </p>

      <h2>Join the Network</h2>
      <p>
        Instead of running standalone searches, you can join the distributed
        network to contribute compute to coordinated campaigns:
      </p>
      <CodeBlock language="bash">
        {`# Connect to the coordinator and start working
./target/release/darkreach work \\
  --coordinator https://api.darkreach.ai`}
      </CodeBlock>
      <p>
        Your node will automatically:
      </p>
      <ul>
        <li>Register with the coordinator</li>
        <li>Claim work blocks from active searches</li>
        <li>Run the sieve &rarr; test &rarr; prove pipeline</li>
        <li>Report results and heartbeat every 10 seconds</li>
      </ul>
      <p>
        See the <a href="/docs/network">Network &amp; Operators</a> guide for
        setting up operator accounts and multi-node deployments.
      </p>

      <h2>Checkpointing</h2>
      <p>
        Searches automatically checkpoint progress every 60 seconds with
        atomic writes and SHA-256 integrity verification. If a search is
        interrupted, it resumes from the last valid checkpoint:
      </p>
      <CodeBlock language="bash">
        {`# Checkpoint is saved to darkreach.checkpoint by default
# Use --checkpoint to specify a custom path
./target/release/darkreach --checkpoint my-search.checkpoint \\
  kbn --k 3 --base 2 --min-n 100000 --max-n 500000`}
      </CodeBlock>

      <h2>Launch the Dashboard</h2>
      <p>
        The coordinator can serve the dashboard directly:
      </p>
      <CodeBlock language="bash">
        {`# Start the coordinator with dashboard
./target/release/darkreach dashboard \\
  --database-url postgres://user:pass@localhost/darkreach \\
  --port 7001

# Open http://localhost:7001 in your browser`}
      </CodeBlock>
      <p>
        Or use the hosted dashboard at{" "}
        <a href="https://app.darkreach.ai">app.darkreach.ai</a>.
      </p>

      <h2>Next Steps</h2>
      <ul>
        <li>
          Learn about the{" "}
          <a href="/docs/architecture">system architecture</a>
        </li>
        <li>
          Explore all{" "}
          <a href="/docs/prime-forms">12 prime forms</a>
        </li>
        <li>
          Understand{" "}
          <a href="/docs/verification">verification and certificates</a>
        </li>
        <li>
          See how the{" "}
          <a href="/docs/ai-engine">AI engine</a> optimizes searches
        </li>
        <li>
          Set up{" "}
          <a href="/docs/projects">projects</a> for coordinated campaigns
        </li>
        <li>
          Deploy a{" "}
          <a href="/download/server">coordinator</a> or{" "}
          <a href="/download/worker">worker</a>
        </li>
        <li>
          <a href="/docs/contributing">Contribute</a> to the project
        </li>
      </ul>
    </div>
  );
}
