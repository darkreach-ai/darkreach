export interface BlogPost {
  slug: string;
  title: string;
  excerpt: string;
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
    date: "2026-02-15",
    author: "darkreach team",
    tags: ["open-source", "philosophy"],
    readingTime: "4 min read",
  },
];
