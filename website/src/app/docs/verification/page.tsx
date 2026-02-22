"use client";

import { CodeBlock } from "@/components/ui/code-block";
import { Badge } from "@/components/ui/badge";

export default function VerificationPage() {
  return (
    <div className="prose-docs">
      <h1>Verification &amp; Certificates</h1>
      <p>
        darkreach uses a <strong>3-tier verification pipeline</strong> to
        classify every prime candidate. Results are either proven deterministic
        (mathematical certainty) or labeled as probable primes (PRP) with
        explicit confidence bounds.
      </p>

      <h2>3-Tier Pipeline</h2>
      <CodeBlock>
        {`Candidate
    │
    ▼
┌──────────────────────────────┐
│  Tier 1: Deterministic Proof │  Pocklington, Morrison, BLS,
│  (N-1 / N+1 factorization)  │  Proth, LLR, Pepin
│  Result: PROVEN PRIME        │
└──────────┬───────────────────┘
           │ if proof not possible
           ▼
┌──────────────────────────────┐
│  Tier 2: Strong PRP Tests    │  BPSW + 25-round Miller-Rabin
│  + Frobenius (>10K bits)     │  + Grantham RQFT
│  Result: PROBABLE PRIME      │
└──────────┬───────────────────┘
           │ for large candidates
           ▼
┌──────────────────────────────┐
│  Tier 3: External Tools      │  PFGW (50-100x faster)
│  PFGW / GWNUM / PRST         │  GWNUM FFI, PRST subprocess
│  Result: PRP or PROVEN       │
└──────────────────────────────┘`}
      </CodeBlock>

      <h2>Proof Types</h2>
      <p>
        Different prime forms admit different proof strategies, depending on
        whether the factorization of N-1 or N+1 is sufficiently known:
      </p>
      <table>
        <thead>
          <tr>
            <th>Proof</th>
            <th>Basis</th>
            <th>Forms</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td><Badge variant="green">Proth</Badge></td>
            <td>N-1 test: if <code>a^((N-1)/2) = -1 mod N</code> for some <code>a</code>, then N is prime (when <code>k &lt; 2^n</code>)</td>
            <td>k*b^n+1, Cullen, Gen Fermat</td>
          </tr>
          <tr>
            <td><Badge variant="green">LLR</Badge></td>
            <td>Lucas-Lehmer-Riesel N+1 test using Lucas sequences</td>
            <td>k*b^n-1, Woodall, Carol/Kynea</td>
          </tr>
          <tr>
            <td><Badge variant="green">Pocklington</Badge></td>
            <td>N-1 partial factorization: if factored portion F &gt; sqrt(N), then N is prime</td>
            <td>Factorial (n!+1), Primorial (p#+1)</td>
          </tr>
          <tr>
            <td><Badge variant="green">Morrison</Badge></td>
            <td>N+1 partial factorization using Lucas sequences</td>
            <td>Factorial (n!-1), Primorial (p#-1)</td>
          </tr>
          <tr>
            <td><Badge variant="green">BLS</Badge></td>
            <td>Brillhart-Lehmer-Selfridge combined N-1/N+1 theorem</td>
            <td>Near-repdigit</td>
          </tr>
          <tr>
            <td><Badge variant="green">Pepin</Badge></td>
            <td>Specialization of Proth for Fermat numbers: <code>3^((F_n-1)/2) = -1 mod F_n</code></td>
            <td>Generalized Fermat (base 2)</td>
          </tr>
          <tr>
            <td><Badge variant="purple">PRP</Badge></td>
            <td>No deterministic proof available — 25-round Miller-Rabin + BPSW</td>
            <td>Wagstaff, Repunit, Palindromic</td>
          </tr>
        </tbody>
      </table>

      <h2>Primality Certificates</h2>
      <p>
        When a deterministic proof succeeds, darkreach generates a{" "}
        <code>PrimalityCertificate</code> — a machine-verifiable witness that
        anyone can check independently:
      </p>
      <CodeBlock language="json">
        {`{
  "type": "Pocklington",
  "n": "147855! + 1",
  "factors_of_n_minus_1": [
    [2, 147855],
    [3, 71420],
    ...
  ],
  "witness_a": 5,
  "verified": true
}`}
      </CodeBlock>
      <p>
        Certificates are stored in the database alongside the prime record and
        can be exported for submission to prime databases like{" "}
        <a href="https://t5k.org" target="_blank" rel="noopener noreferrer">
          The Prime Pages (t5k.org)
        </a>.
      </p>

      <h3>Certificate variants</h3>
      <CodeBlock language="rust">
        {`enum PrimalityCertificate {
    Pocklington { a: u64, factors: Vec<(u64, u32)> },
    Morrison    { lucas_v: Integer, factors: Vec<(u64, u32)> },
    BLS         { a: u64, n_minus_factors: ..., n_plus_factors: ... },
    Proth       { a: u64 },
    LLR         { u0: Integer },
    Pepin       { base: u64 },
    PRP         { bases: Vec<u64>, rounds: u32 },
}`}
      </CodeBlock>

      <h2>Frobenius Test</h2>
      <p>
        For candidates exceeding 10,000 bits, the engine adds a{" "}
        <strong>Grantham RQFT (Restricted Quadratic Frobenius Test)</strong>{" "}
        on top of Miller-Rabin. This test operates over a quadratic extension
        field and checks both:
      </p>
      <ul>
        <li><strong>Euler criterion</strong> — Jacobi symbol consistency</li>
        <li><strong>Frobenius automorphism</strong> — Conjugation check in <code>Z[x]/(x^2-bx-c)</code></li>
      </ul>
      <p>
        Combined with BPSW and Miller-Rabin, this provides extremely strong
        composite detection with no known counterexamples.
      </p>

      <h2>Deep Composite Elimination</h2>
      <p>
        Before primality testing, candidates pass through a{" "}
        <strong>Pollard P-1 filter</strong> that catches composites with smooth
        factors:
      </p>
      <ul>
        <li><strong>Stage 1</strong> — GCD after powering through primes up to B1</li>
        <li><strong>Stage 2</strong> — Extended check for one large prime factor up to B2</li>
        <li><strong>Adaptive tuning</strong> — B1/B2 bounds auto-scale based on candidate size</li>
      </ul>
      <p>
        The P-1 filter runs on all 12 search forms and typically eliminates
        10-30% of candidates that would otherwise require expensive primality
        tests.
      </p>

      <h2>External Tool Acceleration</h2>
      <table>
        <thead>
          <tr>
            <th>Tool</th>
            <th>Speedup</th>
            <th>Forms</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <td><strong>PFGW</strong></td>
            <td>50-100x</td>
            <td>All forms (PRP testing for large candidates)</td>
          </tr>
          <tr>
            <td><strong>GWNUM</strong></td>
            <td>10-50x</td>
            <td>k*b^n forms (FFI, feature-gated)</td>
          </tr>
          <tr>
            <td><strong>PRST</strong></td>
            <td>5-20x</td>
            <td>k*b^n forms (subprocess)</td>
          </tr>
        </tbody>
      </table>
      <p>
        External tools are auto-detected at startup. If available, candidates
        above a size threshold are routed to the external tool instead of
        GMP-based testing.
      </p>

      <h2>Re-verification API</h2>
      <p>
        Any prime in the database can be re-verified via the REST API:
      </p>
      <CodeBlock language="bash">
        {`# Re-verify a specific prime
curl -X POST https://api.darkreach.ai/api/verify \\
  -H "Content-Type: application/json" \\
  -d '{"prime_id": 42}'`}
      </CodeBlock>
      <p>
        This re-runs the full verification pipeline and updates the proof status.
        Useful for auditing results or upgrading PRP results when new proof
        methods become available.
      </p>
    </div>
  );
}
