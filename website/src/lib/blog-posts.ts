export interface BlogPost {
  slug: string;
  title: string;
  excerpt: string;
  /** MDX source content for the full blog post. */
  content: string;
  date: string;
  author: string;
  tags: string[];
  readingTime?: string;
}

export const blogPosts: BlogPost[] = [
  {
    slug: "announcing-darkreach",
    title: "Announcing darkreach: Building the World's Biggest Supercomputer",
    excerpt:
      "Most of the world's computing power sits idle. We're building an open-source platform that turns spare cycles into scientific breakthroughs — starting with prime number discovery.",
    content:
      "Most of the world's computing power sits idle. We're building an open-source platform that turns spare cycles into scientific breakthroughs — starting with prime number discovery.\n\nToday we're publicly launching **darkreach**, a distributed computing platform that coordinates thousands of machines to search for special-form prime numbers. Our mission is simple: take the idle compute that exists on every laptop, server, and workstation and point it at open problems in mathematics.\n\n## Why primes?\n\nPrime number discovery sits at the intersection of pure mathematics and applied computing. Every new large prime validates our software pipeline, stress-tests our distributed coordination layer, and contributes to the public mathematical record.\n\n## What's next\n\nWe're inviting operators to join the network. Install the CLI, register, and start contributing compute in under five minutes.",
    date: "2026-02-20",
    author: "darkreach team",
    tags: ["announcement", "launch"],
    readingTime: "5 min read",
  },
  {
    slug: "prime-discovery-initiative",
    title: "Our First Research Initiative: Prime Number Discovery",
    excerpt:
      "From factorial primes to Sophie Germain pairs, we've built 12 specialized search algorithms with a sieve-test-prove pipeline that produces independently verifiable results.",
    content:
      "From factorial primes to Sophie Germain pairs, we've built 12 specialized search algorithms with a sieve-test-prove pipeline that produces independently verifiable results.\n\n## The pipeline\n\nEvery search form in darkreach follows the same four-stage pattern:\n\n1. **Sieve** — eliminate candidates with small factors\n2. **Test** — parallel Miller-Rabin and form-specific primality tests\n3. **Prove** — deterministic proofs (Proth, Pocklington, Morrison, BLS)\n4. **Log** — record to the database with full certificate\n\n## 12 search forms\n\nWe currently support factorial (n! +/- 1), palindromic, k*b^n +/- 1, near-repdigit, primorial, Cullen-Woodall, Wagstaff, Carol-Kynea, twin primes, Sophie Germain, repunit, and generalized Fermat forms.\n\nEach form has a specialized sieve that exploits the algebraic structure of the candidates to eliminate composites far more efficiently than generic trial division.",
    date: "2026-02-18",
    author: "darkreach team",
    tags: ["mathematics", "research"],
    readingTime: "7 min read",
  },
  {
    slug: "why-open-source",
    title: "Why We're Open Source",
    excerpt:
      "Open source wasn't a marketing decision — it was the only option. Scientific computing demands transparency, reproducibility, and trust. Here's why we chose the MIT license.",
    content:
      "Open source wasn't a marketing decision — it was the only option. Scientific computing demands transparency, reproducibility, and trust. Here's why we chose the MIT license.\n\n## Trust requires transparency\n\nWhen you claim to have found a new prime number, the world needs to verify your work. Closed-source software makes verification impossible — you'd be asking the mathematical community to trust a black box.\n\n## Reproducibility is non-negotiable\n\nEvery result we produce must be independently reproducible. That means our sieve code, our primality tests, and our proof generators are all available for inspection and re-execution.\n\n## The MIT license\n\nWe chose MIT because it's the simplest, most permissive option. Anyone can use darkreach — researchers, hobbyists, and companies alike. The only requirement is attribution.",
    date: "2026-02-15",
    author: "darkreach team",
    tags: ["open-source", "philosophy"],
    readingTime: "4 min read",
  },
];

/**
 * Return all blog posts sorted by date descending (newest first).
 */
export function getAllPosts(): BlogPost[] {
  return [...blogPosts].sort((a, b) => (a.date > b.date ? -1 : 1));
}

/**
 * Retrieve a single blog post by slug.
 * Returns undefined if no matching post is found or the slug is invalid.
 */
export function getPostBySlug(slug: string): BlogPost | undefined {
  // Sanitize slug - only allow alphanumeric, hyphens, underscores
  if (!/^[a-zA-Z0-9_-]+$/.test(slug)) return undefined;
  return blogPosts.find((p) => p.slug === slug);
}
