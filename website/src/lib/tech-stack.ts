export interface TechItem {
  name: string;
  description: string;
}

export const techStack: TechItem[] = [
  {
    name: "Rust",
    description:
      "Zero-cost abstractions, memory safety, and fearless concurrency for the engine and server.",
  },
  {
    name: "GMP (rug)",
    description:
      "GNU Multiple Precision Arithmetic — the gold standard for arbitrary-precision integer math.",
  },
  {
    name: "PFGW / GWNUM",
    description:
      "Specialized number theory software for 50-100x acceleration on large primality tests.",
  },
  {
    name: "PostgreSQL",
    description:
      "Relational database for primes, workers, jobs, and work distribution with row-level locking.",
  },
  {
    name: "Axum",
    description:
      "Async Rust web framework for the coordinator REST API and WebSocket server.",
  },
  {
    name: "Next.js",
    description:
      "React framework for the dashboard (app.darkreach.ai) and website (darkreach.ai).",
  },
];
