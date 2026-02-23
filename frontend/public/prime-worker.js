/**
 * Prime-hunting Web Worker — WASM-accelerated with JS BigInt fallback.
 *
 * At startup, attempts to load the darkreach WASM module for ~3-5x faster
 * arithmetic vs plain JS BigInt. Falls back to JS if WASM loading fails.
 *
 * Supports all 12 search forms:
 *   WASM + JS fallback: kbn, twin, sophie_germain, cullen_woodall, repunit,
 *                        primorial, carol_kynea, gen_fermat, wagstaff
 *   WASM only:          factorial, palindromic, near_repdigit
 *
 * Communicates with the main thread via postMessage:
 *
 * Inbound:
 *   { type: "start", block, config }  — begin searching a work block
 *   { type: "stop" }                  — graceful shutdown
 *   { type: "pause" }                — pause search (tab hidden)
 *   { type: "resume" }               — resume search (tab visible)
 *
 * Outbound:
 *   { type: "init", mode }                                — engine ready
 *   { type: "progress", tested, found, current, speed }   — periodic update
 *   { type: "prime", expression, form, digits, proof_method } — discovery
 *   { type: "done", tested, found, reason }               — block finished
 *   { type: "error", message }                            — error
 */

// ── Engine mode ─────────────────────────────────────────────────────

let wasmReady = false;
let paused = false;

// Try loading WASM module
try {
  importScripts("/wasm/darkreach_wasm.js");
  // wasm_bindgen is now a global async init function
} catch (_) {
  // WASM JS glue not available — will use JS fallback
}

// Initialize WASM asynchronously
(async function initEngine() {
  if (typeof wasm_bindgen === "function") {
    try {
      await wasm_bindgen("/wasm/darkreach_wasm_bg.wasm");
      wasmReady = true;
      self.postMessage({ type: "init", mode: "wasm" });
      return;
    } catch (e) {
      // WASM init failed — fall through to JS
    }
  }
  self.postMessage({ type: "init", mode: "js" });
})();

// ── JS BigInt fallback ──────────────────────────────────────────────
// Kept intact for browsers where WASM fails to load.

const SMALL_PRIMES = [];
{
  const sieveLimit = 8000;
  const sieve = new Uint8Array(sieveLimit);
  for (let i = 2; i * i < sieveLimit; i++) {
    if (!sieve[i]) {
      for (let j = i * i; j < sieveLimit; j += i) sieve[j] = 1;
    }
  }
  for (let i = 2; i < sieveLimit && SMALL_PRIMES.length < 1000; i++) {
    if (!sieve[i]) SMALL_PRIMES.push(BigInt(i));
  }
}

const MR_WITNESSES = [2n, 3n, 5n, 7n, 11n, 13n, 17n, 19n, 23n, 29n, 31n, 37n];

function modPow(base, exp, mod) {
  if (mod === 1n) return 0n;
  let result = 1n;
  base = ((base % mod) + mod) % mod;
  while (exp > 0n) {
    if (exp & 1n) result = (result * base) % mod;
    exp >>= 1n;
    base = (base * base) % mod;
  }
  return result;
}

function hasSmallFactor(n) {
  for (const p of SMALL_PRIMES) {
    if (n === p) return false;
    if (n % p === 0n) return true;
  }
  return false;
}

function millerRabin(n, rounds) {
  if (n < 2n) return false;
  if (n === 2n || n === 3n) return true;
  if (n % 2n === 0n) return false;

  let d = n - 1n;
  let r = 0n;
  while (d % 2n === 0n) {
    d >>= 1n;
    r++;
  }

  const witnessCount = Math.min(rounds, MR_WITNESSES.length);
  for (let i = 0; i < witnessCount; i++) {
    const a = MR_WITNESSES[i];
    if (a >= n - 1n) continue;

    let x = modPow(a, d, n);
    if (x === 1n || x === n - 1n) continue;

    let composite = true;
    for (let j = 1n; j < r; j++) {
      x = (x * x) % n;
      if (x === n - 1n) {
        composite = false;
        break;
      }
    }
    if (composite) return false;
  }
  return true;
}

function isPrime(n) {
  if (n < 2n) return false;
  if (hasSmallFactor(n)) return n <= SMALL_PRIMES[SMALL_PRIMES.length - 1] && SMALL_PRIMES.includes(n);
  return millerRabin(n, 12);
}

function digitCount(n) {
  if (n < 0n) n = -n;
  if (n === 0n) return 1;
  return n.toString().length;
}

/** Quick trial-division primality for small integers (Number, not BigInt). */
function isSmallPrime(n) {
  if (n < 2) return false;
  if (n === 2 || n === 3) return true;
  if (n % 2 === 0 || n % 3 === 0) return false;
  for (let i = 5; i * i <= n; i += 6) {
    if (n % i === 0 || n % (i + 2) === 0) return false;
  }
  return true;
}

// ── JS fallback search strategies ───────────────────────────────────

let stopRequested = false;

async function searchKbnJS(params, blockStart, blockEnd, config) {
  const k = BigInt(params.k || 1);
  const base = BigInt(params.base || 2);
  const sign = BigInt(params.sign || 1);
  const batchSize = config.batchSize || 100;

  let tested = 0;
  let found = 0;
  const startTime = Date.now();

  for (let n = blockStart; n <= blockEnd; n++) {
    if (stopRequested) return { tested, found, reason: "stopped" };

    const candidate = k * (base ** BigInt(n)) + sign;
    if (candidate < 2n) { tested++; continue; }
    tested++;

    if (isPrime(candidate)) {
      found++;
      const signStr = sign === 1n ? "+" : "-";
      const expression = k === 1n
        ? `${base}^${n}${signStr}1`
        : `${k}*${base}^${n}${signStr}1`;

      self.postMessage({
        type: "prime",
        expression,
        form: "kbn",
        digits: digitCount(candidate),
        proof_method: "miller_rabin_12_browser",
      });
    }

    if (tested % batchSize === 0) {
      const elapsed = (Date.now() - startTime) / 1000;
      self.postMessage({
        type: "progress",
        tested, found, current: n,
        speed: elapsed > 0 ? Math.round(tested / elapsed) : 0,
      });
      await yieldControl();
    }
  }
  return { tested, found, reason: "completed" };
}

async function searchTwinJS(params, blockStart, blockEnd, config) {
  const batchSize = config.batchSize || 100;
  let tested = 0;
  let found = 0;
  const startTime = Date.now();

  let start = BigInt(blockStart);
  if (start < 5n) start = 5n;
  const remainder = start % 6n;
  if (remainder !== 5n) {
    start = start + (5n - remainder + 6n) % 6n;
    if (start % 6n !== 5n) start = start - (start % 6n) + 5n;
  }
  const end = BigInt(blockEnd);

  for (let p = start; p <= end; p += 6n) {
    if (stopRequested) return { tested, found, reason: "stopped" };
    tested++;

    if (isPrime(p) && isPrime(p + 2n)) {
      found++;
      self.postMessage({
        type: "prime",
        expression: `(${p}, ${p + 2n})`,
        form: "twin",
        digits: digitCount(p),
        proof_method: "miller_rabin_12_browser",
      });
    }

    if (tested % batchSize === 0) {
      const elapsed = (Date.now() - startTime) / 1000;
      self.postMessage({
        type: "progress",
        tested, found, current: Number(p),
        speed: elapsed > 0 ? Math.round(tested / elapsed) : 0,
      });
      await yieldControl();
    }
  }
  return { tested, found, reason: "completed" };
}

async function searchSophieGermainJS(params, blockStart, blockEnd, config) {
  const batchSize = config.batchSize || 100;
  let tested = 0;
  let found = 0;
  const startTime = Date.now();

  // Handle p=2 (only even prime)
  if (blockStart <= 2 && blockEnd >= 2) {
    tested++;
    // 2 is SG: 2*2+1=5 is prime
    found++;
    self.postMessage({
      type: "prime",
      expression: "2",
      form: "sophie_germain",
      digits: 1,
      proof_method: "miller_rabin_12_browser",
    });
  }

  // Iterate odd candidates
  let pStart = blockStart <= 3 ? 3 : (blockStart % 2 === 0 ? blockStart + 1 : blockStart);

  for (let p = pStart; p <= blockEnd; p += 2) {
    if (stopRequested) return { tested, found, reason: "stopped" };
    tested++;

    const pBig = BigInt(p);
    if (isPrime(pBig)) {
      const sg = 2n * pBig + 1n;
      if (isPrime(sg)) {
        found++;
        self.postMessage({
          type: "prime",
          expression: `${p}`,
          form: "sophie_germain",
          digits: digitCount(pBig),
          proof_method: "miller_rabin_12_browser",
        });
      }
    }

    if (tested % batchSize === 0) {
      const elapsed = (Date.now() - startTime) / 1000;
      self.postMessage({
        type: "progress",
        tested, found, current: p,
        speed: elapsed > 0 ? Math.round(tested / elapsed) : 0,
      });
      await yieldControl();
    }
  }
  return { tested, found, reason: "completed" };
}

async function searchCullenWoodallJS(params, blockStart, blockEnd, config) {
  const batchSize = config.batchSize || 100;
  let tested = 0;
  let found = 0;
  const startTime = Date.now();

  for (let n = blockStart; n <= blockEnd; n++) {
    if (stopRequested) return { tested, found, reason: "stopped" };

    const nBig = BigInt(n);
    const power = 2n ** nBig; // 2^n
    const nTimesPower = nBig * power; // n * 2^n

    // Cullen: n * 2^n + 1
    const cullen = nTimesPower + 1n;
    tested++;
    if (cullen >= 2n && isPrime(cullen)) {
      found++;
      self.postMessage({
        type: "prime",
        expression: `${n}*2^${n}+1`,
        form: "cullen_woodall",
        digits: digitCount(cullen),
        proof_method: "miller_rabin_12_browser",
      });
    }

    // Woodall: n * 2^n - 1
    if (nTimesPower > 1n) {
      const woodall = nTimesPower - 1n;
      tested++;
      if (isPrime(woodall)) {
        found++;
        self.postMessage({
          type: "prime",
          expression: `${n}*2^${n}-1`,
          form: "cullen_woodall",
          digits: digitCount(woodall),
          proof_method: "miller_rabin_12_browser",
        });
      }
    }

    if (tested % batchSize === 0) {
      const elapsed = (Date.now() - startTime) / 1000;
      self.postMessage({
        type: "progress",
        tested, found, current: n,
        speed: elapsed > 0 ? Math.round(tested / elapsed) : 0,
      });
      await yieldControl();
    }
  }
  return { tested, found, reason: "completed" };
}

async function searchRepunitJS(params, blockStart, blockEnd, config) {
  const base = BigInt(params.base || 10);
  if (base < 2n) return { tested: 0, found: 0, reason: "completed" };
  const divisor = base - 1n;
  const batchSize = config.batchSize || 100;
  let tested = 0;
  let found = 0;
  const startTime = Date.now();

  for (let n = blockStart; n <= blockEnd; n++) {
    if (stopRequested) return { tested, found, reason: "stopped" };

    // Necessary condition: n must be prime
    if (!isSmallPrime(n)) continue;

    // R(base, n) = (base^n - 1) / (base - 1)
    const power = base ** BigInt(n);
    const candidate = (power - 1n) / divisor;
    tested++;

    if (isPrime(candidate)) {
      found++;
      self.postMessage({
        type: "prime",
        expression: `R(${base},${n})`,
        form: "repunit",
        digits: digitCount(candidate),
        proof_method: "miller_rabin_12_browser",
      });
    }

    if (tested % batchSize === 0) {
      const elapsed = (Date.now() - startTime) / 1000;
      self.postMessage({
        type: "progress",
        tested, found, current: n,
        speed: elapsed > 0 ? Math.round(tested / elapsed) : 0,
      });
      await yieldControl();
    }
  }
  return { tested, found, reason: "completed" };
}

async function searchPrimorialJS(params, blockStart, blockEnd, config) {
  const batchSize = config.batchSize || 100;
  let tested = 0;
  let found = 0;
  const startTime = Date.now();

  // Collect all primes up to blockEnd
  const primeList = [];
  for (let i = 2; i <= blockEnd; i++) {
    if (isSmallPrime(i)) primeList.push(i);
  }

  // Accumulate primorial up to start
  let primorial = 1n;
  let idx = 0;
  while (idx < primeList.length && primeList[idx] < blockStart) {
    primorial *= BigInt(primeList[idx]);
    idx++;
  }

  // Test each prime p in [blockStart, blockEnd]
  while (idx < primeList.length) {
    if (stopRequested) return { tested, found, reason: "stopped" };
    const p = primeList[idx];
    primorial *= BigInt(p);

    // Test p# + 1
    const plusOne = primorial + 1n;
    tested++;
    if (isPrime(plusOne)) {
      found++;
      self.postMessage({
        type: "prime",
        expression: `${p}#+1`,
        form: "primorial",
        digits: digitCount(plusOne),
        proof_method: "miller_rabin_12_browser",
      });
    }

    // Test p# - 1
    if (primorial > 1n) {
      const minusOne = primorial - 1n;
      tested++;
      if (isPrime(minusOne)) {
        found++;
        self.postMessage({
          type: "prime",
          expression: `${p}#-1`,
          form: "primorial",
          digits: digitCount(minusOne),
          proof_method: "miller_rabin_12_browser",
        });
      }
    }

    if (tested % batchSize === 0) {
      const elapsed = (Date.now() - startTime) / 1000;
      self.postMessage({
        type: "progress",
        tested, found, current: p,
        speed: elapsed > 0 ? Math.round(tested / elapsed) : 0,
      });
      await yieldControl();
    }

    idx++;
  }
  return { tested, found, reason: "completed" };
}

async function searchCarolKyneaJS(params, blockStart, blockEnd, config) {
  const batchSize = config.batchSize || 100;
  let tested = 0;
  let found = 0;
  const startTime = Date.now();

  for (let n = blockStart; n <= blockEnd; n++) {
    if (stopRequested) return { tested, found, reason: "stopped" };

    const power = 2n ** BigInt(n); // 2^n

    // Carol: (2^n - 1)^2 - 2
    if (power > 1n) {
      const pm1 = power - 1n;
      const sq = pm1 * pm1;
      if (sq > 2n) {
        const carol = sq - 2n;
        tested++;
        if (isPrime(carol)) {
          found++;
          self.postMessage({
            type: "prime",
            expression: `(2^${n}-1)^2-2`,
            form: "carol_kynea",
            digits: digitCount(carol),
            proof_method: "miller_rabin_12_browser",
          });
        }
      }
    }

    // Kynea: (2^n + 1)^2 - 2
    const pp1 = power + 1n;
    const sq = pp1 * pp1;
    const kynea = sq - 2n;
    tested++;
    if (isPrime(kynea)) {
      found++;
      self.postMessage({
        type: "prime",
        expression: `(2^${n}+1)^2-2`,
        form: "carol_kynea",
        digits: digitCount(kynea),
        proof_method: "miller_rabin_12_browser",
      });
    }

    if (tested % batchSize === 0) {
      const elapsed = (Date.now() - startTime) / 1000;
      self.postMessage({
        type: "progress",
        tested, found, current: n,
        speed: elapsed > 0 ? Math.round(tested / elapsed) : 0,
      });
      await yieldControl();
    }
  }
  return { tested, found, reason: "completed" };
}

async function searchGenFermatJS(params, blockStart, blockEnd, config) {
  const base = BigInt(params.base || 2);
  const batchSize = config.batchSize || 100;
  let tested = 0;
  let found = 0;
  const startTime = Date.now();

  for (let n = blockStart; n <= blockEnd; n++) {
    if (stopRequested) return { tested, found, reason: "stopped" };

    // Compute base^(2^n) via repeated squaring
    let x = base;
    for (let i = 0; i < n; i++) {
      x = x * x;
    }
    const candidate = x + 1n;

    tested++;
    if (isPrime(candidate)) {
      found++;
      self.postMessage({
        type: "prime",
        expression: `${base}^(2^${n})+1`,
        form: "gen_fermat",
        digits: digitCount(candidate),
        proof_method: "miller_rabin_12_browser",
      });
    }

    if (tested % batchSize === 0) {
      const elapsed = (Date.now() - startTime) / 1000;
      self.postMessage({
        type: "progress",
        tested, found, current: n,
        speed: elapsed > 0 ? Math.round(tested / elapsed) : 0,
      });
      await yieldControl();
    }
  }
  return { tested, found, reason: "completed" };
}

async function searchWagstaffJS(params, blockStart, blockEnd, config) {
  const batchSize = config.batchSize || 100;
  let tested = 0;
  let found = 0;
  const startTime = Date.now();

  for (let p = blockStart; p <= blockEnd; p++) {
    if (stopRequested) return { tested, found, reason: "stopped" };

    // Must be an odd prime (p=2 gives non-integer (4+1)/3)
    if (p === 2 || !isSmallPrime(p)) continue;

    // (2^p + 1) / 3
    const power = 2n ** BigInt(p);
    const candidate = (power + 1n) / 3n;

    tested++;
    if (isPrime(candidate)) {
      found++;
      self.postMessage({
        type: "prime",
        expression: `(2^${p}+1)/3`,
        form: "wagstaff",
        digits: digitCount(candidate),
        proof_method: "miller_rabin_12_browser",
      });
    }

    if (tested % batchSize === 0) {
      const elapsed = (Date.now() - startTime) / 1000;
      self.postMessage({
        type: "progress",
        tested, found, current: p,
        speed: elapsed > 0 ? Math.round(tested / elapsed) : 0,
      });
      await yieldControl();
    }
  }
  return { tested, found, reason: "completed" };
}

// ── WASM-only forms ─────────────────────────────────────────────────
// factorial, palindromic, and near_repdigit require WASM — their
// generation logic is too complex for a performant JS fallback.

const WASM_ONLY_FORMS = new Set(["factorial", "palindromic", "near_repdigit"]);

// ── Yield helper ────────────────────────────────────────────────────

/** Yield to the event loop. Sleeps 500ms while paused, 0ms otherwise. */
function yieldControl() {
  return new Promise((resolve) => setTimeout(resolve, paused ? 500 : 0));
}

// ── WASM sub-batch dispatcher ───────────────────────────────────────

const WASM_SUB_BATCH = 50;

/**
 * Run a search using the WASM engine in sub-batches of 50, yielding
 * between each call so stop/pause messages can be processed.
 */
async function searchWasm(searchType, params, blockStart, blockEnd, config) {
  const batchSize = config.batchSize || 100;
  const paramsJson = JSON.stringify(params);
  const startTime = Date.now();

  let totalTested = 0;
  let totalFound = 0;
  let lastResultHash = null;
  let n = blockStart;

  while (n <= blockEnd && !stopRequested) {
    const subEnd = Math.min(n + WASM_SUB_BATCH - 1, blockEnd);

    // Prefer hashed entry point for content-addressed verification
    const searchFn = wasm_bindgen.search_block_hashed || wasm_bindgen.search_block;
    const resultJson = searchFn(
      searchType,
      paramsJson,
      BigInt(n),
      BigInt(subEnd)
    );
    const result = JSON.parse(resultJson);

    // Capture result hash from the last sub-batch (covers final state)
    if (result.result_hash) {
      lastResultHash = result.result_hash;
    }

    // Emit any discovered primes
    for (const prime of result.primes) {
      self.postMessage({ type: "prime", ...prime });
    }
    totalTested += result.tested;
    totalFound += result.primes.length;

    // Report progress periodically
    if (totalTested % batchSize < WASM_SUB_BATCH || subEnd >= blockEnd) {
      const elapsed = (Date.now() - startTime) / 1000;
      self.postMessage({
        type: "progress",
        tested: totalTested,
        found: totalFound,
        current: subEnd,
        speed: elapsed > 0 ? Math.round(totalTested / elapsed) : 0,
      });
    }

    n = subEnd + 1;

    // Yield to event loop for stop/pause processing
    await yieldControl();
  }

  return {
    tested: totalTested,
    found: totalFound,
    reason: stopRequested ? "stopped" : "completed",
    result_hash: lastResultHash,
  };
}

// ── Message handler ─────────────────────────────────────────────────

self.onmessage = async function (e) {
  const msg = e.data;

  if (msg.type === "stop") {
    stopRequested = true;
    return;
  }

  if (msg.type === "pause") {
    paused = true;
    return;
  }

  if (msg.type === "resume") {
    paused = false;
    return;
  }

  if (msg.type === "start") {
    stopRequested = false;
    paused = false;
    const { block, config } = msg;
    const searchType = block.search_type;
    const blockStart = block.block_start;
    const blockEnd = block.block_end;
    const params = typeof block.params === "string"
      ? JSON.parse(block.params)
      : (block.params || {});
    const workerConfig = config || { batchSize: 100 };

    try {
      let result;

      if (wasmReady) {
        // WASM path — supports all 12 forms
        result = await searchWasm(searchType, params, blockStart, blockEnd, workerConfig);
      } else {
        // JS fallback — supports 9 of 12 forms
        if (WASM_ONLY_FORMS.has(searchType)) {
          self.postMessage({
            type: "error",
            message: `${searchType} search requires WASM engine (not available in JS fallback)`,
          });
          return;
        }

        switch (searchType) {
          case "kbn":
            result = await searchKbnJS(params, blockStart, blockEnd, workerConfig);
            break;
          case "twin":
            result = await searchTwinJS(params, blockStart, blockEnd, workerConfig);
            break;
          case "sophie_germain":
            result = await searchSophieGermainJS(params, blockStart, blockEnd, workerConfig);
            break;
          case "cullen_woodall":
            result = await searchCullenWoodallJS(params, blockStart, blockEnd, workerConfig);
            break;
          case "repunit":
            result = await searchRepunitJS(params, blockStart, blockEnd, workerConfig);
            break;
          case "primorial":
            result = await searchPrimorialJS(params, blockStart, blockEnd, workerConfig);
            break;
          case "carol_kynea":
            result = await searchCarolKyneaJS(params, blockStart, blockEnd, workerConfig);
            break;
          case "gen_fermat":
            result = await searchGenFermatJS(params, blockStart, blockEnd, workerConfig);
            break;
          case "wagstaff":
            result = await searchWagstaffJS(params, blockStart, blockEnd, workerConfig);
            break;
          default:
            self.postMessage({
              type: "error",
              message: `Unsupported search type for browser: ${searchType}`,
            });
            return;
        }
      }

      self.postMessage({
        type: "done",
        tested: result.tested,
        found: result.found,
        reason: result.reason,
        result_hash: result.result_hash || null,
      });
    } catch (err) {
      self.postMessage({
        type: "error",
        message: err.message || String(err),
      });
    }
  }
};
