// darkreach-research — Web research skill for the Researcher agent.
//
// Uses Firecrawl to scrape t5k.org, OEIS, and mersenneforum.
// Compares findings against darkreach records via the REST API.
//
// Endpoints used:
//   Firecrawl API — web scraping
//   GET /api/records — darkreach world records
//   GET /api/primes — darkreach prime database

const API_BASE = process.env.DARKREACH_API_URL || "https://api.darkreach.ai";

/**
 * Scrape top N primes from t5k.org for a given form.
 * Returns structured data: { form, records: [{ rank, digits, discoverer, date }] }
 */
async function scrapeT5kTop(firecrawl, form, limit = 20) {
  const url = `https://t5k.org/top20/page.php?id=${encodeURIComponent(form)}`;
  const result = await firecrawl.scrapeUrl(url, {
    formats: ["markdown"],
  });

  // Parse the t5k.org top-20 page for record entries.
  // The page structure varies by form but generally has a table with
  // rank, prime description, digit count, discoverer, and date.
  const lines = (result.markdown || "").split("\n");
  const records = [];

  for (const line of lines) {
    // Match table rows: | rank | description | digits | who | date |
    const match = line.match(
      /\|\s*(\d+)\s*\|.*?\|\s*([\d,]+)\s*\|.*?\|\s*(\d{4})/
    );
    if (match) {
      records.push({
        rank: parseInt(match[1]),
        digits: parseInt(match[2].replace(/,/g, "")),
        date: match[3],
      });
      if (records.length >= limit) break;
    }
  }

  return { form, source: "t5k.org", records };
}

/**
 * Fetch OEIS sequence data.
 * Returns: { sequence_id, name, values: number[], references: string[] }
 */
async function scrapeOeis(firecrawl, sequenceId) {
  const url = `https://oeis.org/${sequenceId}`;
  const result = await firecrawl.scrapeUrl(url, {
    formats: ["markdown"],
  });

  const markdown = result.markdown || "";

  // Extract sequence name from first heading
  const nameMatch = markdown.match(/^#\s*(.+)/m);
  const name = nameMatch ? nameMatch[1].trim() : sequenceId;

  // Extract initial values
  const valuesMatch = markdown.match(/(\d+(?:,\s*\d+)+)/);
  const values = valuesMatch
    ? valuesMatch[1].split(",").map((v) => parseInt(v.trim()))
    : [];

  return { sequence_id: sequenceId, name, values, source: "oeis.org" };
}

/**
 * Check competitor activity on mersenneforum and PrimeGrid.
 * Returns: { competitors: [{ source, form, event, date }] }
 */
async function checkCompetitors(firecrawl) {
  const sources = [
    {
      name: "mersenneforum",
      url: "https://www.mersenneforum.org/forumdisplay.php?f=80",
    },
    { name: "primegrid", url: "https://www.primegrid.com/recent_primes.php" },
  ];

  const events = [];

  for (const source of sources) {
    try {
      const result = await firecrawl.scrapeUrl(source.url, {
        formats: ["markdown"],
      });
      // Extract recent activity — prime discoveries, new searches, etc.
      // This is a best-effort parse since forum structures vary.
      const markdown = result.markdown || "";
      const lines = markdown.split("\n").slice(0, 50); // First 50 lines

      for (const line of lines) {
        if (
          line.match(
            /prime|record|discovery|found|new|PRP|proof/i
          )
        ) {
          events.push({
            source: source.name,
            summary: line.trim().slice(0, 200),
          });
        }
      }
    } catch (err) {
      events.push({
        source: source.name,
        summary: `Scrape failed: ${err.message}`,
      });
    }
  }

  return { competitors: events };
}

/**
 * Compare darkreach records against world records from t5k.org.
 * Returns: { comparisons: [{ form, our_best, world_best, gap_digits, gap_pct }] }
 */
async function compareRecords(firecrawl) {
  // Fetch our records
  const res = await fetch(`${API_BASE}/api/records`);
  if (!res.ok) throw new Error(`darkreach API error: ${res.status}`);
  const ourRecords = await res.json();

  // Forms to compare (map darkreach form names to t5k.org page IDs)
  const formMap = {
    factorial: "16",
    palindromic: "53",
    primorial: "15",
    twin: "1",
    "sophie-germain": "2",
    wagstaff: "67",
    "cullen-woodall": "39",
    repunit: "57",
  };

  const comparisons = [];

  for (const [form, t5kId] of Object.entries(formMap)) {
    try {
      const world = await scrapeT5kTop(firecrawl, t5kId, 1);
      const worldBest = world.records[0]?.digits || 0;

      const ourBest = ourRecords.find((r) => r.form === form);
      const ourDigits = ourBest?.digit_count || 0;

      comparisons.push({
        form,
        our_best_digits: ourDigits,
        world_best_digits: worldBest,
        gap_digits: worldBest - ourDigits,
        gap_pct:
          worldBest > 0
            ? ((worldBest - ourDigits) / worldBest * 100).toFixed(1)
            : "N/A",
      });
    } catch (err) {
      comparisons.push({ form, error: err.message });
    }
  }

  return { comparisons };
}

module.exports = {
  scrapeT5kTop,
  scrapeOeis,
  checkCompetitors,
  compareRecords,
};
