"use client";

import { Section } from "@/components/ui/section";
import { CodeBlock } from "@/components/ui/code-block";
import Link from "next/link";

const buildCommands = `# Install dependencies
# macOS: brew install gmp rust
# Linux: sudo apt install build-essential libgmp-dev m4

git clone https://github.com/darkreach-ai/darkreach.git
cd darkreach
cargo build --release`;

const registerAndRun = `# 1. Register with the coordinator
./target/release/darkreach register \\
  --server https://api.darkreach.ai \\
  --username your-name \\
  --email you@example.com

# 2. Start the operator node (claims work automatically)
./target/release/darkreach run`;

const dockerQuickStart = `# Pull and run the node image
docker run -d --name darkreach-node \\
  -e DATABASE_URL=postgres://user:pass@host:5432/darkreach \\
  -e DARKREACH_SERVER=https://api.darkreach.ai \\
  -e DARKREACH_API_KEY=dk_your_api_key \\
  -e DARKREACH_USERNAME=your-name \\
  --restart unless-stopped \\
  ghcr.io/darkreach-ai/darkreach-node:latest`;

const composeSnippet = `# docker-compose.operator.yml
services:
  node:
    image: ghcr.io/darkreach-ai/darkreach-node:latest
    environment:
      DATABASE_URL: \${DATABASE_URL}
      DARKREACH_SERVER: \${DARKREACH_SERVER:-https://api.darkreach.ai}
      DARKREACH_API_KEY: \${DARKREACH_API_KEY}
      DARKREACH_USERNAME: \${DARKREACH_USERNAME}
      DARKREACH_LOG_FORMAT: json
    volumes:
      - \${HOME:-.}/.darkreach:/root/.darkreach:ro
      - node-checkpoints:/app
    restart: unless-stopped
    deploy:
      resources:
        limits:
          memory: 4G
    logging:
      driver: json-file
      options:
        max-size: "50m"
        max-file: "5"

volumes:
  node-checkpoints:`;

const composeCommands = `# Start with Docker Compose
docker compose -f docker-compose.operator.yml up -d

# View logs
docker compose -f docker-compose.operator.yml logs -f

# Stop
docker compose -f docker-compose.operator.yml down`;

const systemdTemplate = `[Unit]
Description=Darkreach Operator Node
After=network.target

[Service]
Type=simple
EnvironmentFile=/opt/darkreach/.env
ExecStart=/usr/local/bin/darkreach run
WorkingDirectory=/opt/darkreach
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target`;

const scalingCommands = `# Install operator service
sudo cp deploy/darkreach-operator.service /etc/systemd/system/
sudo systemctl daemon-reload

# Start the operator node
sudo systemctl enable --now darkreach-operator

# Check status
sudo systemctl status darkreach-operator`;

export default function NodeDeploymentPage() {
  return (
    <>
      <Section>
        <nav className="text-sm text-muted-foreground mb-8">
          <Link href="/download" className="hover:text-foreground transition-colors">
            Download
          </Link>
          <span className="mx-2">/</span>
          <span className="text-foreground">Node Deployment</span>
        </nav>

        <h1 className="text-4xl font-bold text-foreground mb-4">
          Node Deployment
        </h1>
        <p className="text-muted-foreground max-w-3xl mb-12">
          Operator nodes are the compute engines that run prime searches. They
          connect to the coordinator, claim work blocks, run sieves and
          primality tests, and report results. Each node can handle any of the
          12 prime forms.
        </p>

        <div className="space-y-12">
          <div>
            <h2 className="text-2xl font-semibold text-foreground mb-2">
              Prerequisites
            </h2>
            <ul className="list-disc list-inside text-muted-foreground space-y-1">
              <li>Rust toolchain (1.75+) or Docker</li>
              <li>GMP library (for native builds)</li>
              <li>Network access to the coordinator</li>
            </ul>
          </div>

          <div>
            <h2 className="text-2xl font-semibold text-foreground mb-2">
              Option A: Build from source
            </h2>
            <CodeBlock language="bash">{buildCommands}</CodeBlock>
            <p className="text-sm text-muted-foreground mt-4 mb-4">
              Register as an operator, then start the node daemon:
            </p>
            <CodeBlock language="bash">{registerAndRun}</CodeBlock>
            <p className="text-sm text-muted-foreground mt-3">
              The node will register with the coordinator, appear in the
              network dashboard, and start claiming work blocks automatically.
            </p>
          </div>

          <div>
            <h2 className="text-2xl font-semibold text-foreground mb-2">
              Option B: Docker (recommended)
            </h2>
            <p className="text-muted-foreground mb-4">
              The node image is published to GHCR for both amd64 and arm64.
              No Rust toolchain required.
            </p>
            <CodeBlock language="bash">{dockerQuickStart}</CodeBlock>
            <p className="text-sm text-muted-foreground mt-3">
              View the image on{" "}
              <a
                href="https://github.com/darkreach-ai/darkreach/pkgs/container/darkreach-node"
                className="text-primary hover:underline"
                target="_blank"
                rel="noopener noreferrer"
              >
                GitHub Container Registry
              </a>
              .
            </p>
          </div>

          <div>
            <h2 className="text-2xl font-semibold text-foreground mb-2">
              Docker Compose
            </h2>
            <p className="text-muted-foreground mb-4">
              Use the provided compose file for a managed deployment with
              log rotation and resource limits:
            </p>
            <CodeBlock language="yaml">{composeSnippet}</CodeBlock>
            <p className="text-sm text-muted-foreground mt-4 mb-4">
              Run:
            </p>
            <CodeBlock language="bash">{composeCommands}</CodeBlock>
          </div>

          <div>
            <h2 className="text-2xl font-semibold text-foreground mb-2">
              Verify heartbeat
            </h2>
            <p className="text-muted-foreground">
              Once the node is running, verify it appears in the coordinator
              dashboard under the Network tab. Nodes send heartbeats every 30
              seconds. You can also check the coordinator logs for registration
              events.
            </p>
          </div>

          <div>
            <h2 className="text-2xl font-semibold text-foreground mb-2">
              Systemd (bare metal)
            </h2>
            <p className="text-muted-foreground mb-4">
              For bare-metal deployments, use a systemd service unit.
              Register first with <code className="text-sm bg-muted px-1 rounded">darkreach register</code>,
              then install the service:
            </p>
            <CodeBlock language="ini">{systemdTemplate}</CodeBlock>
            <p className="text-sm text-muted-foreground mt-4 mb-4">
              Deploy:
            </p>
            <CodeBlock language="bash">{scalingCommands}</CodeBlock>
          </div>

          <div>
            <h2 className="text-2xl font-semibold text-foreground mb-2">
              Monitoring
            </h2>
            <p className="text-muted-foreground">
              Node status, throughput, and prime discoveries are visible in the{" "}
              <a
                href="https://app.darkreach.ai"
                className="text-primary hover:underline"
              >
                dashboard
              </a>
              . The Network tab shows all connected nodes with their current
              search form, candidates tested, and last heartbeat time.
            </p>
          </div>
        </div>
      </Section>
    </>
  );
}
