# C String/Memory Performance Tracker

<!-- Benchmark note: Validation benchmarks must be thorough across realistic scenarios, and explicitly test edge cases and potential performance cliff zones before marking a function as faster than glibc. -->

## Status Fields
- `Implemented`: `yes` / `no`
- `Benchmarked`: `no` / `partial` / `yes`
- `Faster than glibc`: `no` / `unknown` / `yes`

## Narrow String and Memory APIs
| Function | Standard/Origin | Implemented | Benchmarked | Faster than glibc | Notes |
|---|---|---:|---:|---:|---|
| `memcpy` | C/POSIX | yes | yes | no | AVX2 tuned with 63/64-byte cliff handling; latest full run wins 49/62 with remaining losses centered on 95-1024B cliffs and 8MiB +/- 1 |
| `memmove` | C/POSIX | yes | partial | no | AVX2 overlap path now uses 1MiB `rep movsb` thresholds; focused 1024B/256KiB subset improved to wins 7/14, but backward-overlap cases still lag badly |
| `memset` | C/POSIX | yes | yes | no | AVX2/NT tuned with 480-512-byte fast path; latest full run wins 54/68, with remaining misses around 64B misalignment, 256B alignment corners, and 1MiB |
| `memcmp` | C/POSIX | yes | partial | no | AVX2 draft + focused benchmark harness are in place; latest captured run is 13/78 wins, with major regressions on 31-256B equal/diff-last paths |
| `memchr` | C/POSIX | yes | partial | no | AVX2 scan path + criterion harness landed; focused run is 22/54 wins, with regressions concentrated at 31/63B misses and 4KiB full-scan cases |
| `memrchr` | GNU/POSIX ext | yes | partial | no | AVX2 reverse-scan path + criterion harness landed; focused run is 8/54 wins, with large miss-path regressions (especially 31/63B and 4KiB scans) |
| `memccpy` | C/POSIX | yes | partial | no | New memchr+copy implementation benchmarked at 12/28 wins; faster on early-stop and large-size misses, but 31/63B stop-last and miss paths regress notably |
| `memmem` | GNU ext | yes | partial | no | New candidate-filtered memmem path benchmarked at 37/48 wins; strong gains for `needle` lengths >=4, but `needle` length 1 mid/tail/miss scans remain 1.5-2.5x slower |
| `explicit_bzero` | BSD/GNU ext | yes | partial | no | New dedicated bench run is 16/21 wins; strong gains at 31-256B, but slight regressions remain around 4KiB alignment corners |
| `bzero` | BSD legacy | yes | partial | no | New dedicated bench run is 18/21 wins; fast on 31-1024B and 64KiB, but 4KiB aligned cases are still a little slower |
| `bcmp` | BSD legacy | yes | partial | no | New dedicated bench run is 1/28 wins; current memcmp-backed path regresses heavily on 63-256B equal/diff-last cases |
| `bcopy` | BSD legacy | yes | partial | no | Delegating to current optimized memmove path yields 16/30 wins in focused overlap/non-overlap runs; backward-overlap at 1KiB/64KiB regresses heavily (~2.2-2.5x) |
| `ffs` | POSIX | yes | yes | no | Dedicated value-pattern benchmark completed; current run is 21/44 wins with sub-1.1% deltas, so this is near parity rather than a consistent glibc win |
| `strlen` | C/POSIX | yes | partial | no | memchr-backed path benchmarked at 0/21 wins in current C-string cases; biggest losses are mid/tail scans at 1KiB-64KiB |
| `strnlen` | POSIX | yes | partial | no | memchr-backed path benchmarked at 0/28 wins; especially weak when scanning long bounded ranges and `maxlen`-before-terminator scenarios |
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
| `strspn` | C/POSIX | yes | partial | no | Dedicated benchmark run is 0/12 wins; current linear `accept.contains` checks regress across all tested sizes |
| `strcspn` | C/POSIX | yes | partial | no | Dedicated benchmark run is 0/12 wins; reject-set membership checks are consistently slower than glibc |
| `strpbrk` | C/POSIX | yes | partial | no | Dedicated benchmark run is 0/12 wins; first/mid/tail and miss patterns all regress versus glibc |
| `index` | BSD legacy | yes | partial | no | Alias of `strchr`; inherits current `strchr` benchmark profile (0/30 wins) |
| `rindex` | BSD legacy | yes | partial | no | Alias of `strrchr`; inherits current `strrchr` benchmark profile (0/30 wins) |
| `strtok` | C/POSIX | yes | no | unknown | safe state-based API |
| `strtok_r` | POSIX | yes | no | unknown |  |
| `strxfrm` | C/POSIX | yes | partial | no | Dedicated benchmark run is 4/6 wins in C-locale copy-like cases, but 4KiB paths still regress so this is not a consistent overall win |
| `strdup` | POSIX | yes | no | unknown |  |
| `strndup` | POSIX | yes | no | unknown |  |
| `strerror` | C/POSIX | no | no | unknown | deferred for now |
| `strerror_r` | POSIX | no | no | unknown | deferred for now |

## Wide String and Wide Memory APIs
| Function | Standard/Origin | Implemented | Benchmarked | Faster than glibc | Notes |
|---|---|---:|---:|---:|---|
| `wcslen` | C/POSIX | yes | no | unknown |  |
| `wcsnlen` | POSIX | yes | no | unknown |  |
| `wcscpy` | C/POSIX | yes | no | unknown |  |
| `wcsncpy` | C/POSIX | yes | no | unknown |  |
| `wcpcpy` | GNU ext | yes | no | unknown |  |
| `wcpncpy` | GNU ext | yes | no | unknown |  |
| `wcscat` | C/POSIX | yes | no | unknown |  |
| `wcsncat` | C/POSIX | yes | no | unknown |  |
| `wcscmp` | C/POSIX | yes | no | unknown |  |
| `wcsncmp` | C/POSIX | yes | no | unknown |  |
| `wcscoll` | C/POSIX | yes | no | unknown | locale-sensitive in libc |
| `wcschr` | C/POSIX | yes | no | unknown |  |
| `wcsrchr` | C/POSIX | yes | no | unknown |  |
| `wcsstr` | C/POSIX | yes | no | unknown |  |
| `wcsspn` | C/POSIX | yes | no | unknown |  |
| `wcscspn` | C/POSIX | yes | no | unknown |  |
| `wcspbrk` | C/POSIX | yes | no | unknown |  |
| `wcscasecmp` | POSIX | yes | no | unknown |  |
| `wcsncasecmp` | POSIX | yes | no | unknown |  |
| `wcschrnul` | GNU ext | yes | no | unknown |  |
| `wcslcpy` | BSD ext | yes | no | unknown |  |
| `wcslcat` | BSD ext | yes | no | unknown |  |
| `wcstok` | C/POSIX | yes | no | unknown |  |
| `wcsxfrm` | C/POSIX | yes | no | unknown | locale-sensitive in libc |
| `wcsdup` | POSIX | yes | no | unknown |  |
| `wmemcpy` | C/POSIX | yes | no | unknown |  |
| `wmempcpy` | GNU ext | yes | no | unknown |  |
| `wmemmove` | C/POSIX | yes | no | unknown |  |
| `wmemset` | C/POSIX | yes | no | unknown |  |
| `wmemcmp` | C/POSIX | yes | no | unknown |  |
| `wmemchr` | C/POSIX | yes | no | unknown |  |
| `wmemrchr` | GNU ext | yes | no | unknown |  |
