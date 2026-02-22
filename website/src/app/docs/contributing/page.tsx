"use client";

import { CodeBlock } from "@/components/ui/code-block";

export default function ContributingPage() {
  return (
    <div className="prose-docs">
      <h1>Contributing</h1>
      <p>
        darkreach is open source under the MIT license. Contributions are
        welcome — whether it is a bug fix, new prime form, performance
        improvement, or documentation update.
      </p>

      <h2>Development Setup</h2>
      <CodeBlock language="bash">
        {`# Fork and clone
git clone https://github.com/YOUR_USERNAME/darkreach.git
cd darkreach

# Install dependencies
# macOS: brew install gmp
# Linux: sudo apt install build-essential libgmp-dev m4

# Build and test
cargo build
cargo test`}
      </CodeBlock>

      <h3>Frontend development</h3>
      <CodeBlock language="bash">
        {`# Dashboard (app.darkreach.ai)
cd frontend && npm install && npm run dev

# Website (darkreach.ai)
cd website && npm install && npm run dev`}
      </CodeBlock>

      <h2>Workflow</h2>
      <ol>
        <li>Fork the repository on GitHub</li>
        <li>
          Create a feature branch:{" "}
          <code>git checkout -b feat/my-feature</code>
        </li>
        <li>Make your changes with tests</li>
        <li>
          Run the full test suite: <code>cargo test</code>
        </li>
        <li>
          Run clippy: <code>cargo clippy -- -D warnings</code>
        </li>
        <li>
          Format: <code>cargo fmt</code>
        </li>
        <li>
          Open a pull request against <code>master</code>
        </li>
      </ol>
      <p>
        Branch naming: <code>feat/</code>, <code>fix/</code>,{" "}
        <code>chore/</code>, <code>docs/</code>, <code>deploy/</code>. PRs
        use squash-and-merge by default.
      </p>

      <h2>Code Style</h2>
      <ul>
        <li>
          <strong>Rust</strong>: Follow <code>rustfmt</code> defaults. No{" "}
          <code>unsafe</code> in the main crate (except the macOS QoS syscall).
        </li>
        <li>
          <strong>Comments</strong>: This codebase is a teaching tool for
          computational number theory. Document algorithms at an academic level
          — cite theorems, link OEIS sequences, reference papers.
        </li>
        <li>
          <strong>Engine files</strong>: ~30-40% comments. Server: ~20-30%.
          Frontend: ~15-25%.
        </li>
        <li>
          All output goes to stderr (<code>eprintln!</code>). Results are logged
          to PostgreSQL.
        </li>
      </ul>

      <h2>Testing</h2>
      <CodeBlock language="bash">
        {`# Run all unit tests (1000+ passing)
cargo test

# Integration tests
cargo test --test db_integration
cargo test --test api_integration
cargo test --test cli_tests
cargo test --test property_tests
cargo test --test security_tests

# Benchmarks
cargo bench

# Run with small ranges to verify quickly
cargo run -- factorial --start 1 --end 100
cargo run -- kbn --k 3 --base 2 --min-n 1 --max-n 1000
cargo run -- palindromic --base 10 --min-digits 1 --max-digits 9

# Frontend
cd frontend && npm test           # Vitest unit tests
cd frontend && npm run test:e2e   # Playwright E2E tests`}
      </CodeBlock>

      <h2>Adding a New Prime Form</h2>
      <p>
        To add a new search form (e.g., <code>mega-primes</code>):
      </p>
      <ol>
        <li>
          Create <code>src/mega_primes.rs</code> following the
          sieve &rarr; filter &rarr; test &rarr; prove &rarr; report pipeline
        </li>
        <li>
          Add the module to <code>src/lib.rs</code>
        </li>
        <li>
          Add a CLI subcommand in <code>src/main.rs</code> and dispatch in{" "}
          <code>src/cli.rs</code>
        </li>
        <li>
          Add a checkpoint variant in <code>src/checkpoint.rs</code>
        </li>
        <li>
          Add search manager support in <code>src/search_manager.rs</code>
        </li>
        <li>
          Add deploy support in <code>src/deploy.rs</code>
        </li>
        <li>
          Add the form to <code>website/src/lib/prime-forms.ts</code>
        </li>
        <li>Write tests covering known primes and edge cases</li>
      </ol>

      <h2>Adding an API Endpoint</h2>
      <ol>
        <li>
          Create handler in appropriate <code>src/dashboard/routes_*.rs</code>{" "}
          file (or create a new route module)
        </li>
        <li>
          Register the route in <code>src/dashboard/mod.rs</code>
        </li>
        <li>
          Add DB query methods in <code>src/db/*.rs</code> submodule
        </li>
        <li>
          Add migration if new tables needed (
          <code>supabase/migrations/</code>)
        </li>
      </ol>

      <h2>Project Structure</h2>
      <CodeBlock>
        {`src/
├── main.rs              # CLI routing (clap)
├── cli.rs               # CLI execution, search dispatch
├── lib.rs               # Module re-exports, utilities
│
├── [12 Search Forms]
├── factorial.rs         # n! ± 1
├── palindromic.rs       # Palindromic primes
├── kbn.rs               # k·b^n ± 1
├── ... (9 more form modules)
│
├── [Core Primitives]
├── sieve.rs             # Sieve, Montgomery, BitSieve, wheel
├── proof.rs             # Pocklington, Morrison, BLS proofs
├── verify.rs            # 3-tier verification pipeline
├── certificate.rs       # Primality certificates
├── p1.rs                # Pollard P-1 factoring
│
├── [External Tools]
├── pfgw.rs              # PFGW subprocess
├── prst.rs              # PRST subprocess
├── gwnum.rs             # GWNUM FFI (feature-gated)
│
├── [AI & Strategy]
├── ai_engine.rs         # OODA decision loop
├── agent.rs             # AI agent infrastructure
├── classify.rs          # Result classification
│
├── [Server]
├── dashboard/           # Axum web server (15 route modules)
│   ├── mod.rs           # Router, AppState, middleware
│   ├── websocket.rs     # WebSocket (2s push)
│   ├── routes_*.rs      # 13 route modules
│   └── ...
├── db/                  # PostgreSQL via sqlx (13 submodules)
│   ├── mod.rs           # Database struct, pool, types
│   ├── primes.rs        # Prime CRUD
│   ├── workers.rs       # Worker management
│   ├── jobs.rs          # Search jobs, work blocks
│   ├── agents.rs        # Agent tasks, budgets
│   ├── projects.rs      # Project campaigns
│   ├── operators.rs     # Operator accounts
│   └── ...
├── project/             # Campaign management
│   ├── config.rs        # TOML configuration
│   ├── cost.rs          # Power-law cost model
│   ├── orchestration.rs # Phase state machine
│   └── ...
│
├── [Infrastructure]
├── checkpoint.rs        # JSON checkpoint save/load
├── search_manager.rs    # Work distribution
├── fleet.rs             # In-memory worker registry
├── pg_worker.rs         # PostgreSQL work claiming
├── worker_client.rs     # Worker HTTP client
├── operator.rs          # Operator management
├── events.rs            # Event bus
├── metrics.rs           # System metrics
├── prom_metrics.rs      # Prometheus export
├── deploy.rs            # SSH deployment
└── progress.rs          # Atomic counters

frontend/                # Dashboard (app.darkreach.ai)
├── src/app/             # 17 Next.js pages
├── src/components/      # 50+ React components
├── src/hooks/           # 18 custom hooks
└── public/              # PWA assets, icons

website/                 # Public site (darkreach.ai)
├── src/app/             # 14 pages (landing, docs, blog, etc.)
├── src/components/      # UI components + Three.js
└── src/lib/             # Data files and utilities`}
      </CodeBlock>

      <h2>Questions?</h2>
      <p>
        Open an issue on{" "}
        <a
          href="https://github.com/darkreach-ai/darkreach/issues"
          target="_blank"
          rel="noopener noreferrer"
        >
          GitHub
        </a>{" "}
        or join the{" "}
        <a
          href="https://discord.gg/2Khf4t8M33"
          target="_blank"
          rel="noopener noreferrer"
        >
          Discord
        </a>{" "}
        for help with onboarding.
      </p>
    </div>
  );
}
