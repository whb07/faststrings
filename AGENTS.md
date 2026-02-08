# Repository Guidelines

## Project Scope & Performance Bar
- This crate is intended to be a systems library focused on ISO C standard library functions and widely used POSIX string/memory routines.
- Contributions should preserve C/POSIX-compatible behavior while prioritizing high-performance implementations.
- Acceptance criteria for each implemented function: it should thoroughly outperform the corresponding glibc function in throughput under representative benchmarks.
- When adding or changing hot-path code, include benchmark evidence and methodology in the PR notes.

## CPU Baseline Assumptions
- Assume SSE/AVX-capable targets as the baseline.
- Assume CPU features commonly available on post-2020 hardware.
- Legacy CPU support is out of scope.
- Do not add runtime feature-detection fallbacks for legacy paths.
- Prefer compile-time-selected implementations and feature gating.

## no_std Workspace Assumption
- Treat this workspace as `no_std` first.
- Production/library code should not depend on `std`.
- Tests and benchmarks may use `std` for harnessing, allocation helpers, fixtures, and validation utilities.

## CRITICAL: Intrinsics and libc Call-Through
- Do not default to compiler/standard-library memory intrinsics as the core implementation strategy for libc-style APIs.
- Avoid implementation patterns that may lower into libc calls (for example, `memcpy`/`memset` symbol call-through to glibc under some codegen decisions).
- The purpose of this library is to provide independent, high-performance implementations that beat glibc, not wrappers that may delegate back to glibc.
- If intrinsics are used in limited cases, document why they cannot lower to glibc for that path and provide benchmark evidence.

## Project Structure & Module Organization
- `Cargo.toml` defines crate metadata and dependencies.
- `src/lib.rs` is the crate entry point (currently minimal) and should expose new modules via `mod`/`pub mod`.
- Core implementations live in `src/*.rs`:
  - `mem.rs`, `str.rs`, `search.rs`, `token.rs` for byte-string and memory helpers.
  - `wide.rs`, `wmem.rs` for wide-character equivalents.
  - `simd.rs`, `memcpy.rs`, `memset.rs` for optimized low-level paths.
- Add new functionality in a focused module file (for example, `src/parse.rs`) and document public APIs with rustdoc.

## Benchmarking & Function Layout
- Keep function implementations isolated and focused; prefer one primary API per standalone source file when adding new libc-style functions.
- Every performance-sensitive function should have a benchmark plan before optimization work starts.
- Benchmarks must cover multiple data sizes and value distributions, not only a single representative input.
- Do not benchmark only happy paths; include adversarial and boundary scenarios.
- Explicitly benchmark potential cliff zones (threshold edges, alignment boundaries, short/medium/large transitions, overlap/non-overlap cases where relevant).
- Compare against glibc baselines with identical workload shape and report methodology in PR notes.
- A function should only be marked performance-complete after benchmark evidence shows consistent wins over glibc across target scenarios.

## Build, Test, and Development Commands
- `cargo check`: fast compile validation during development.
- `cargo test`: run all unit tests and doc tests.
- `cargo test <name>`: run targeted tests (example: `cargo test mem::tests`).
- `cargo fmt --all`: apply standard Rust formatting.
- `cargo clippy --all-targets --all-features -D warnings`: run lint checks and fail on warnings.

## Coding Style & Naming Conventions
- Follow Rust 2024 defaults: 4-space indentation and standard rustfmt layout.
- Naming:
  - `snake_case` for functions/modules.
  - `CamelCase` for types/traits.
  - `SCREAMING_SNAKE_CASE` for constants.
- Prefer safe Rust first. If `unsafe` is required (SIMD or raw-pointer code), keep blocks small and explain invariants in a `# Safety` section.
- Keep C-style behavior explicit in docs for compatibility-oriented APIs (`memcpy`, `strncpy`, etc.).

## Testing Guidelines
- Keep tests close to code using `mod tests` inside each module file.
- Cover edge cases: empty buffers, null terminators, overlap semantics, and threshold boundaries.
- Add a regression test with every bug fix.
- No formal coverage gate is configured yet; new behavior should ship with direct tests.

## Commit & Pull Request Guidelines
- Git history is currently minimal (`initial`), so no strict message format is established.
- Use concise imperative commit subjects with optional scope (example: `search: fix null-terminator scan`).
- PRs should include:
  - What changed and why.
  - How it was validated (`cargo test`, `cargo clippy`).
  - Performance notes when touching SIMD/unsafe paths.
