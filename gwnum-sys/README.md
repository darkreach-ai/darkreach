# gwnum-sys — FFI Bindings to GWNUM

Raw Rust FFI bindings to George Woltman's [GWNUM](https://www.mersenne.org/download/) library, the world's fastest modular arithmetic engine for numbers of the form k*b^n+c.

GWNUM uses hand-tuned SSE2/AVX/FMA3/AVX-512 assembly for x86-64 processors to perform modular squaring via Irrational Base Discrete Weighted Transforms (IBDWT). It is the engine behind [Prime95/mprime](https://www.mersenne.org/download/) and [GIMPS](https://www.mersenne.org/).

## Platform

**x86-64 only.** GWNUM contains hand-written assembly for Intel/AMD processors. ARM/Apple Silicon is not supported — use FLINT (`--features flint`) or PFGW subprocess instead.

## Installation

### 1. Download Prime95 source

```bash
# From Mersenne.org
wget https://www.mersenneforum.org/download/prime95/v30.19/p95v3019b20.source.zip
unzip p95v3019b20.source.zip
```

### 2. Build gwnum.a

```bash
cd gwnum
make -f make64
# Produces: gwnum.a (~15MB static library)
```

### 3. Install

```bash
# Library
sudo cp gwnum.a /usr/local/lib/gwnum.a

# Headers (for bindgen, optional)
sudo mkdir -p /usr/local/include/gwnum
sudo cp gwnum.h cpuid.h giants.h /usr/local/include/gwnum/
```

### 4. Build darkreach with GWNUM

```bash
cargo build --release --features gwnum
```

Or set the library path explicitly:

```bash
GWNUM_LIB_DIR=/path/to/gwnum cargo build --release --features gwnum
```

## Search Paths

The build script (`build.rs`) searches for `gwnum.a` in:

1. `/usr/local/lib/gwnum.a`
2. `/usr/lib/gwnum.a`
3. `/opt/gwnum/lib/gwnum.a`
4. `$GWNUM_LIB_DIR/gwnum.a` (environment variable)

If not found, the build succeeds with a warning. The safe wrapper in `src/gwnum.rs` returns `GwError::Unavailable` and falls back to GMP/PFGW.

## What's Accelerated

| Form | GWNUM Function | Speedup vs GMP |
|------|---------------|----------------|
| k*b^n+1 (Proth) | `gwnum_proth()` | 50-100x at >10K digits |
| k*2^n-1 (LLR) | `gwnum_llr()` | 50-100x at >10K digits |
| (2^p+1)/3 (Wagstaff) | `vrba_reix_test()` | 50-100x (only efficient test) |

Forms like factorial, primorial, and palindromic use PFGW subprocess instead (better deterministic proof support).

## Error Checking

The safe wrapper includes Gerbicz error checking for LLR and Vrba-Reix tests. On checksum mismatch, it rolls back to the last verified checkpoint and re-verifies against GMP.

## Docker

```dockerfile
# In a multi-stage Docker build, install GWNUM in the build stage:
FROM rust:1-bookworm AS rust-build
RUN apt-get update && apt-get install -y wget unzip
RUN wget -q https://www.mersenneforum.org/download/prime95/v30.19/p95v3019b20.source.zip \
    && unzip p95v3019b20.source.zip \
    && cd gwnum && make -f make64 \
    && cp gwnum.a /usr/local/lib/
COPY . /app
RUN cd /app && cargo build --release --features gwnum
```

## License

GWNUM is distributed under a custom license by George Woltman (part of Prime95). The gwnum-sys crate provides Rust FFI declarations only and does not redistribute GWNUM source or binaries.
