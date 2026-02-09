# C String/Memory Performance Tracker

<!-- Benchmark note: Validation benchmarks must be thorough across realistic scenarios, and explicitly test edge cases and potential performance cliff zones before marking a function as faster than glibc. -->

## Status Fields
- `Implemented`: `yes` / `no`
- `Benchmarked`: `no` / `partial` / `yes`
- `Faster than glibc`: `no` / `unknown` / `yes`

## High-ROI Completion Plan
The following APIs are the highest-return items because they sit directly under most
other libc-style string/memory routines and dominate total byte traffic in real
workloads. Do not mark this plan complete until every row below has all checks done.

| Function | Priority | Required benchmark coverage | Completion gate |
|---|---:|---|---|
| `memcpy` | P0 | size sweep + alignment cliffs + large-buffer thresholds | `yes` only if wins are consistent across small/medium/large and no major cliff regressions remain |
| `memset` | P0 | size sweep + alignment cliffs + value patterns + large-buffer thresholds | `yes` only if wins are consistent across zero/non-zero values and no major cliff regressions remain |
| `memmove` | P0 | non-overlap + overlap-forward + overlap-backward across size cliffs | `yes` only if overlap-backward regressions are closed and non-overlap remains competitive |
| `memcmp` | P0 | equal + diff-first/mid/last + alignment cliffs | `yes` only if small/medium/large comparisons consistently beat glibc |
| `memchr` | P0 | miss/hit-first/mid/last + alignment cliffs + large scans | `yes` only if miss-heavy and large-scan cases are consistently faster |
| `strlen` | P0 | NUL at head/mid/tail across size cliffs | `yes` only if head/mid/tail patterns are consistently faster |
| `strnlen` | P0 | maxlen-before-NUL/at-NUL/after-NUL + mid-NUL bounded scans | `yes` only if bounded long-scan cases are consistently faster |

High-ROI execution checklist:
- [ ] `memcpy` tuned + benchmark evidence updated
- [ ] `memset` tuned + benchmark evidence updated
- [ ] `memmove` tuned + benchmark evidence updated
- [ ] `memcmp` tuned + benchmark evidence updated
- [ ] `memchr` tuned + benchmark evidence updated
- [ ] `strlen` tuned + benchmark evidence updated
- [ ] `strnlen` tuned + benchmark evidence updated

Latest high-ROI snapshot (criterion `new/estimates.json` as of this pass):
- `memcpy`: wins 49 / 62
- `memset`: wins 55 / 68
- `memmove`: wins 53 / 93
- `memcmp`: wins 26 / 104
- `memchr`: wins 13 / 116
- `strlen`: wins 1 / 21
- `strnlen`: wins 5 / 28

## Narrow String and Memory APIs
| Function | Standard/Origin | Implemented | Benchmarked | Faster than glibc | Notes |
|---|---|---:|---:|---:|---|
| `memcpy` | C/POSIX | yes | yes | no | AVX2 tuned with 63/64-byte cliff handling; latest full run wins 49/62 with remaining losses centered on 95-1024B cliffs and 8MiB +/- 1 |
| `memmove` | C/POSIX | yes | partial | no | Backward overlap path now uses 256B AVX2 descending chunks plus end-alignment, raising the current snapshot to 53/93 wins; still partial with persistent forward 511/512 cliffs and remaining overlap losses |
| `memset` | C/POSIX | yes | yes | no | AVX2/NT tuned with 480-512-byte fast path; latest full run wins 55/68, with remaining misses concentrated in 64B misalignment and a few 256B/large-value corners |
| `memcmp` | C/POSIX | yes | partial | no | AVX2 now covers >=32B plus a dedicated 32-64B fast path; latest snapshot is 26/104 wins, but equal/diff-last and alignment-heavy cases remain behind glibc |
| `memchr` | C/POSIX | yes | partial | no | New small-size AVX2/SSE kernels and first-byte fast path improved 31/63 miss and hit-first cases, but overall snapshot remains 13/116 wins with large miss/tail patterns still lagging |
| `memrchr` | GNU/POSIX ext | yes | partial | no | AVX2 reverse-scan path + criterion harness landed; focused run is 8/54 wins, with large miss-path regressions (especially 31/63B and 4KiB scans) |
| `memccpy` | C/POSIX | yes | partial | no | New memchr+copy implementation benchmarked at 12/28 wins; faster on early-stop and large-size misses, but 31/63B stop-last and miss paths regress notably |
| `memmem` | GNU ext | yes | partial | no | New candidate-filtered memmem path benchmarked at 37/48 wins; strong gains for `needle` lengths >=4, but `needle` length 1 mid/tail/miss scans remain 1.5-2.5x slower |
| `explicit_bzero` | BSD/GNU ext | yes | partial | no | New dedicated bench run is 16/21 wins; strong gains at 31-256B, but slight regressions remain around 4KiB alignment corners |
| `bzero` | BSD legacy | yes | partial | no | New dedicated bench run is 18/21 wins; fast on 31-1024B and 64KiB, but 4KiB aligned cases are still a little slower |
| `bcmp` | BSD legacy | yes | partial | no | New dedicated bench run is 1/28 wins; current memcmp-backed path regresses heavily on 63-256B equal/diff-last cases |
| `bcopy` | BSD legacy | yes | partial | no | Delegating to current optimized memmove path yields 16/30 wins in focused overlap/non-overlap runs; backward-overlap at 1KiB/64KiB regresses heavily (~2.2-2.5x) |
| `ffs` | POSIX | yes | yes | no | Dedicated value-pattern benchmark completed; current run is 21/44 wins with sub-1.1% deltas, so this is near parity rather than a consistent glibc win |
| `strlen` | C/POSIX | yes | partial | no | Hybrid scan (`testz` AVX2 + memchr-backed medium/large path) now wins head-heavy cases (snapshot 1/21), but mid/tail long scans are still slower than glibc |
| `strnlen` | POSIX | yes | partial | no | Reuses the updated scan/memchr path and now reaches 5/28 wins in snapshot runs; bounded long-scan and mid-NUL cases remain behind glibc |
| `strverscmp` | GNU ext | yes | partial | no | Dedicated benchmark run is 2/10 wins; only a couple long-string cases edge out glibc while most numeric/version-ordering cases regress |
| `strcpy` | C/POSIX | yes | partial | no | Dedicated benchmark run is 0/3 wins; copy path remains ~1.4-2.3x slower than glibc across tested sizes |
| `strncpy` | C/POSIX | yes | partial | no | Dedicated benchmark run is 0/6 wins; both truncation and pad cases lag glibc (worst around small pad paths) |
| `stpcpy` | POSIX/GNU ext | yes | partial | no | Dedicated benchmark run is 0/3 wins; consistently behind glibc from 31B through 4KiB |
| `stpncpy` | POSIX | yes | partial | no | Dedicated benchmark run is 0/6 wins; truncation and pad workloads are slower than glibc in all measured cases |
| `strcat` | C/POSIX | yes | partial | no | Dedicated benchmark run is 0/3 wins; append path is ~1.2-1.5x slower in current cases |
| `strncat` | C/POSIX | yes | partial | no | Dedicated benchmark run is 1/6 wins; near-parity at 256B `n_full`, but other n/size combinations still regress |
| `strcmp` | C/POSIX | yes | partial | no | New dedicated benchmark run is 0/30 wins; equal/diff and shorter-string cases are consistently ~3-40x slower than glibc |
| `strncmp` | C/POSIX | yes | partial | no | New dedicated benchmark run is 0/30 wins; small `n` and long-scan bounded cases remain consistently far behind glibc |
| `strcoll` | C/POSIX | yes | partial | no | Dedicated benchmark run is 0/18 wins in C-locale style cases; current path is consistently slower than glibc collation |
| `strcasecmp` | POSIX | yes | partial | no | Dedicated benchmark run is 0/18 wins; equal/diff/shorter-string cases are broadly slower, especially at larger sizes |
| `strncasecmp` | POSIX | yes | partial | no | Dedicated benchmark run is 0/18 wins; bounded and shorter-string scenarios all regress against glibc |
| `strlcpy` | BSD ext | yes | partial | no | Dedicated benchmark run is 1/6 wins; near-parity on 256B fit, but truncate and large cases still slower overall |
| `strlcat` | BSD ext | yes | partial | no | Dedicated benchmark run is 0/6 wins; mostly near-parity but still consistently behind glibc |
| `strchr` | C/POSIX | yes | partial | no | memchr-backed path benchmarked at 0/30 wins; especially severe regressions on early-hit cases (e.g. 64KiB hit-head) and mid/miss scans |
| `strchrnul` | GNU ext | yes | partial | no | memchr-backed path benchmarked at 0/30 wins; large losses on hit-head and small miss/find-nul scenarios |
| `strrchr` | C/POSIX | yes | partial | no | memrchr-backed path benchmarked at 0/30 wins; misses and head/mid hit patterns remain significantly behind glibc |
| `strstr` | C/POSIX | yes | partial | no | Dedicated benchmark run is 0/15 wins; tail/miss scans are substantially behind glibc (worst case ~350x on 4KiB miss path) |
| `strcasestr` | GNU ext | yes | partial | no | Dedicated benchmark run is 2/15 wins (small hit-head only); most medium/large scans remain slower, including empty-needle cases |
| `strspn` | C/POSIX | yes | partial | no | AVX2 small-set scan + bitmap path improved to 2/12 wins (4KiB tail/full-match), but most small/medium cases still regress |
| `strcspn` | C/POSIX | yes | partial | no | Bitmap + memchr small-set path improved to 6/12 wins (mainly mid/tail/miss), but hit-first cases still regress |
| `strpbrk` | C/POSIX | yes | partial | no | AVX2 find-any small-set path improved to 6/12 wins; medium/large scans are faster, but most 31B and hit-first cases remain slower |
| `index` | BSD legacy | yes | partial | no | Alias of `strchr`; inherits current `strchr` benchmark profile (0/30 wins) |
| `rindex` | BSD legacy | yes | partial | no | Alias of `strrchr`; inherits current `strrchr` benchmark profile (0/30 wins) |
| `strtok` | C/POSIX | yes | partial | no | Dedicated benchmark run is 1/3 wins; near-parity at 256B token streams but large (4KiB) tokenization remains much slower |
| `strtok_r` | POSIX | yes | partial | no | Dedicated benchmark run is 1/3 wins; slight edge at 256B, but 4KiB tokenization is still significantly behind glibc |
| `strxfrm` | C/POSIX | yes | partial | no | Dedicated benchmark run is 4/6 wins in C-locale copy-like cases, but 4KiB paths still regress so this is not a consistent overall win |
| `strdup` | POSIX | yes | partial | no | Dedicated benchmark run is 1/3 wins; very strong large-string win, but small/medium sizes remain slower overall |
| `strndup` | POSIX | yes | partial | no | Dedicated benchmark run is 0/6 wins; truncating and full-dup scenarios all regress versus glibc |
| `strerror` | C/POSIX | yes | partial | yes | New dedicated benchmark run is 6/6 wins (known + unknown errno cases), but coverage is still a focused subset |
| `strerror_r` | POSIX | yes | partial | yes | New dedicated benchmark run is 6/6 wins across fit/tight known and unknown cases; broader locale/platform coverage still pending |

## Wide String and Wide Memory APIs
| Function | Standard/Origin | Implemented | Benchmarked | Faster than glibc | Notes |
|---|---|---:|---:|---:|---|
| `wcslen` | C/POSIX | yes | partial | no | Focused benchmark run is 0/3 wins; current scan path is slower than glibc across tested sizes |
| `wcsnlen` | POSIX | yes | partial | no | Focused benchmark run is 0/3 wins (`maxlen = len/2`), with consistent regressions versus glibc |
| `wcscpy` | C/POSIX | yes | partial | no | Focused benchmark run is 0/3 wins; copy throughput is behind glibc at small/medium/large sizes |
| `wcsncpy` | C/POSIX | yes | partial | no | Focused benchmark run is 0/3 wins for `n = len/2`; truncating copy remains slower than glibc |
| `wcpcpy` | GNU ext | yes | partial | no | Focused benchmark run is 0/3 wins; return-end copy path trails glibc in all measured sizes |
| `wcpncpy` | GNU ext | yes | partial | no | Focused benchmark run is 0/3 wins for `n = len/2`; current path is significantly slower at larger sizes |
| `wcscat` | C/POSIX | yes | partial | no | Focused benchmark run is 0/3 wins; append path regresses versus glibc across tested sizes |
| `wcsncat` | C/POSIX | yes | partial | no | Focused benchmark run is 0/3 wins (`n = src/2`); bounded append is slower than glibc |
| `wcscmp` | C/POSIX | yes | partial | no | Focused benchmark run is 0/3 wins on diff-mid cases; comparison throughput is well below glibc |
| `wcsncmp` | C/POSIX | yes | partial | no | Focused benchmark run is 0/3 wins (`n = len/2`, diff-first); bounded compare path regresses heavily |
| `wcscoll` | C/POSIX | yes | partial | no | Focused C-locale-style diff-mid run is 0/3 wins; still slower than glibc collation |
| `wcschr` | C/POSIX | yes | partial | no | Focused benchmark run is 0/3 wins (hit-mid); search path lags glibc substantially |
| `wcsrchr` | C/POSIX | yes | partial | no | Focused benchmark run is 0/3 wins (hit-tail); reverse search remains slower than glibc |
| `wcsstr` | C/POSIX | yes | partial | no | Focused benchmark run is 0/3 wins (hit-mid); substring search is far behind glibc |
| `wcsspn` | C/POSIX | yes | partial | no | Focused benchmark run is 0/3 wins on full-prefix cases; set-scan implementation is slower |
| `wcscspn` | C/POSIX | yes | partial | no | Focused benchmark run is 3/3 wins on hit-mid reject cases, but coverage is narrow and not yet performance-complete |
| `wcspbrk` | C/POSIX | yes | partial | no | Focused benchmark run is 3/3 wins on hit-mid accept cases, but broader scenario coverage is still missing |
| `wcscasecmp` | POSIX | yes | partial | no | Focused benchmark run is 3/3 wins on equal case-folded inputs, but this is a narrow workload slice |
| `wcsncasecmp` | POSIX | yes | partial | no | Focused benchmark run is 3/3 wins (`n = len/2`, equal case-folded), pending broader validation |
| `wcschrnul` | GNU ext | yes | partial | no | Focused benchmark run is 0/3 wins on miss-paths; current implementation is slower across sizes |
| `wcslcpy` | BSD ext | yes | partial | no | Focused benchmark run is 0/3 wins (fit capacity); current copy-limit path trails glibc |
| `wcslcat` | BSD ext | yes | partial | no | Focused benchmark run is 0/3 wins (fit capacity); append-limit path remains slower |
| `wcstok` | C/POSIX | yes | partial | no | Focused benchmark run is 0/3 wins; large tokenization workloads are much slower than glibc |
| `wcsxfrm` | C/POSIX | yes | partial | no | Focused C-locale copy-like run is 0/3 wins; current transform path lags glibc |
| `wcsdup` | POSIX | yes | partial | no | Focused benchmark run is 0/3 wins; duplicate path is slower than glibc across tested sizes |
| `wmemcpy` | C/POSIX | yes | partial | no | Dedicated benchmark run is 1/3 wins; near-parity at medium size but slower on small and large copies |
| `wmempcpy` | GNU ext | yes | partial | no | Dedicated benchmark run is 0/3 wins; current path trails glibc across tested sizes |
| `wmemmove` | C/POSIX | yes | partial | no | Dedicated benchmark run is 1/3 wins (small size), with medium/large non-overlap paths still slower |
| `wmemset` | C/POSIX | yes | partial | no | Dedicated benchmark run is 0/3 wins; near parity, but still consistently behind glibc |
| `wmemcmp` | C/POSIX | yes | partial | no | Dedicated benchmark run is 0/3 wins on equal buffers; current implementation is much slower than glibc |
| `wmemchr` | C/POSIX | yes | partial | no | Dedicated benchmark run is 0/3 wins; hit-mid scans are significantly slower than glibc |
| `wmemrchr` | GNU ext | yes | partial | unknown | Dedicated benchmark compares against a scalar reverse-scan baseline (glibc lacks `wmemrchr`); current path is slower in all tested sizes |
