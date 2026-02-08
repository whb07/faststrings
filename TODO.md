# C String/Memory Performance Tracker

<!-- Benchmark note: Validation benchmarks must be thorough across realistic scenarios, and explicitly test edge cases and potential performance cliff zones before marking a function as faster than glibc. -->

## Status Fields
- `Implemented`: `yes` / `no`
- `Benchmarked`: `no` / `partial` / `yes`
- `Faster than glibc`: `no` / `unknown` / `yes`

## Narrow String and Memory APIs
| Function | Standard/Origin | Implemented | Benchmarked | Faster than glibc | Notes |
|---|---|---:|---:|---:|---|
| `memcpy` | C/POSIX | yes | yes | no | AVX2 tuned with 63/64-byte cliff handling; latest full run wins 46/62 but still loses on several 95-1024B cases and 8MiB +/- 1 |
| `memmove` | C/POSIX | yes | no | unknown |  |
| `memset` | C/POSIX | yes | yes | no | AVX2/NT tuned; latest full run wins 53/68 with remaining misses around 64B misalignment, 511B, and ~256KiB |
| `memcmp` | C/POSIX | yes | no | unknown |  |
| `memchr` | C/POSIX | yes | no | unknown |  |
| `memrchr` | GNU/POSIX ext | yes | no | unknown |  |
| `memccpy` | C/POSIX | yes | no | unknown |  |
| `memmem` | GNU ext | yes | no | unknown |  |
| `explicit_bzero` | BSD/GNU ext | yes | no | unknown |  |
| `bzero` | BSD legacy | yes | no | unknown |  |
| `bcmp` | BSD legacy | yes | no | unknown |  |
| `bcopy` | BSD legacy | yes | no | unknown |  |
| `ffs` | POSIX | yes | no | unknown |  |
| `strlen` | C/POSIX | yes | no | unknown |  |
| `strnlen` | POSIX | yes | no | unknown |  |
| `strverscmp` | GNU ext | yes | no | unknown |  |
| `strcpy` | C/POSIX | yes | no | unknown |  |
| `strncpy` | C/POSIX | yes | no | unknown |  |
| `stpcpy` | POSIX/GNU ext | yes | no | unknown |  |
| `stpncpy` | POSIX | yes | no | unknown |  |
| `strcat` | C/POSIX | yes | no | unknown |  |
| `strncat` | C/POSIX | yes | no | unknown |  |
| `strcmp` | C/POSIX | yes | no | unknown |  |
| `strncmp` | C/POSIX | yes | no | unknown |  |
| `strcoll` | C/POSIX | yes | no | unknown | locale-sensitive in libc |
| `strcasecmp` | POSIX | yes | no | unknown |  |
| `strncasecmp` | POSIX | yes | no | unknown |  |
| `strlcpy` | BSD ext | yes | no | unknown |  |
| `strlcat` | BSD ext | yes | no | unknown |  |
| `strchr` | C/POSIX | yes | no | unknown |  |
| `strchrnul` | GNU ext | yes | no | unknown |  |
| `strrchr` | C/POSIX | yes | no | unknown |  |
| `strstr` | C/POSIX | yes | no | unknown |  |
| `strcasestr` | GNU ext | yes | no | unknown |  |
| `strspn` | C/POSIX | yes | no | unknown |  |
| `strcspn` | C/POSIX | yes | no | unknown |  |
| `strpbrk` | C/POSIX | yes | no | unknown |  |
| `index` | BSD legacy | yes | no | unknown | alias-style API |
| `rindex` | BSD legacy | yes | no | unknown | alias-style API |
| `strtok` | C/POSIX | yes | no | unknown | safe state-based API |
| `strtok_r` | POSIX | yes | no | unknown |  |
| `strxfrm` | C/POSIX | yes | no | unknown | locale-sensitive in libc |
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
