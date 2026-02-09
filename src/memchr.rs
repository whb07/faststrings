//! Optimized memchr/memrchr implementations.
#![allow(unsafe_code)]

#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;

#[inline(always)]
fn first_set_bit(mask: i32) -> usize {
    (mask as u32).trailing_zeros() as usize
}

#[inline(always)]
fn last_set_bit(mask: i32) -> usize {
    31 - (mask as u32).leading_zeros() as usize
}

/// High-performance memchr over exactly `n` bytes.
///
/// Returns the index of the first matching byte.
///
/// # Safety
///
/// - `s` must be valid for reads of `n` bytes.
#[inline(always)]
pub unsafe fn optimized_memchr_unified(s: *const u8, n: usize, needle: u8) -> Option<usize> {
    if n == 0 {
        return None;
    }

    #[cfg(target_arch = "x86_64")]
    {
        return unsafe { optimized_memchr_avx2(s, n, needle) };
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        unsafe { optimized_memchr_scalar(s, n, needle) }
    }
}

/// High-performance memrchr over exactly `n` bytes.
///
/// Returns the index of the last matching byte.
///
/// # Safety
///
/// - `s` must be valid for reads of `n` bytes.
#[inline(always)]
pub unsafe fn optimized_memrchr_unified(s: *const u8, n: usize, needle: u8) -> Option<usize> {
    if n == 0 {
        return None;
    }

    #[cfg(target_arch = "x86_64")]
    {
        return unsafe { optimized_memrchr_avx2(s, n, needle) };
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        unsafe { optimized_memrchr_scalar(s, n, needle) }
    }
}

#[inline(always)]
unsafe fn optimized_memchr_scalar(s: *const u8, n: usize, needle: u8) -> Option<usize> {
    let mut i = 0usize;
    while i < n {
        let byte = unsafe { *s.add(i) };
        if byte == needle {
            return Some(i);
        }
        i += 1;
    }
    None
}

#[inline(always)]
unsafe fn optimized_memrchr_scalar(s: *const u8, n: usize, needle: u8) -> Option<usize> {
    let mut i = n;
    while i > 0 {
        i -= 1;
        let byte = unsafe { *s.add(i) };
        if byte == needle {
            return Some(i);
        }
    }
    None
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn match_mask_32(s: *const u8, needle_v: __m256i) -> i32 {
    let v = unsafe { _mm256_loadu_si256(s as *const __m256i) };
    let eq = _mm256_cmpeq_epi8(v, needle_v);
    _mm256_movemask_epi8(eq)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn optimized_memchr_avx2(s: *const u8, n: usize, needle: u8) -> Option<usize> {
    if n < 32 {
        return unsafe { optimized_memchr_scalar(s, n, needle) };
    }

    let needle_v = _mm256_set1_epi8(needle as i8);
    let mut i = 0usize;

    while i + 128 <= n {
        let p0 = unsafe { s.add(i) };
        let p1 = unsafe { s.add(i + 32) };
        let p2 = unsafe { s.add(i + 64) };
        let p3 = unsafe { s.add(i + 96) };

        let m0 = unsafe { match_mask_32(p0, needle_v) };
        if m0 != 0 {
            return Some(i + first_set_bit(m0));
        }

        let m1 = unsafe { match_mask_32(p1, needle_v) };
        if m1 != 0 {
            return Some(i + 32 + first_set_bit(m1));
        }

        let m2 = unsafe { match_mask_32(p2, needle_v) };
        if m2 != 0 {
            return Some(i + 64 + first_set_bit(m2));
        }

        let m3 = unsafe { match_mask_32(p3, needle_v) };
        if m3 != 0 {
            return Some(i + 96 + first_set_bit(m3));
        }

        i += 128;
    }

    while i + 32 <= n {
        let p = unsafe { s.add(i) };
        let m = unsafe { match_mask_32(p, needle_v) };
        if m != 0 {
            return Some(i + first_set_bit(m));
        }
        i += 32;
    }

    unsafe { optimized_memchr_scalar(s.add(i), n - i, needle).map(|tail| i + tail) }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn optimized_memrchr_avx2(s: *const u8, n: usize, needle: u8) -> Option<usize> {
    if n < 32 {
        return unsafe { optimized_memrchr_scalar(s, n, needle) };
    }

    let needle_v = _mm256_set1_epi8(needle as i8);
    let mut i = n;

    while i >= 128 {
        let base = i - 128;

        let p3 = unsafe { s.add(base + 96) };
        let m3 = unsafe { match_mask_32(p3, needle_v) };
        if m3 != 0 {
            return Some(base + 96 + last_set_bit(m3));
        }

        let p2 = unsafe { s.add(base + 64) };
        let m2 = unsafe { match_mask_32(p2, needle_v) };
        if m2 != 0 {
            return Some(base + 64 + last_set_bit(m2));
        }

        let p1 = unsafe { s.add(base + 32) };
        let m1 = unsafe { match_mask_32(p1, needle_v) };
        if m1 != 0 {
            return Some(base + 32 + last_set_bit(m1));
        }

        let p0 = unsafe { s.add(base) };
        let m0 = unsafe { match_mask_32(p0, needle_v) };
        if m0 != 0 {
            return Some(base + last_set_bit(m0));
        }

        i = base;
    }

    while i >= 32 {
        let base = i - 32;
        let p = unsafe { s.add(base) };
        let m = unsafe { match_mask_32(p, needle_v) };
        if m != 0 {
            return Some(base + last_set_bit(m));
        }
        i = base;
    }

    unsafe { optimized_memrchr_scalar(s, i, needle) }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NEEDLE: u8 = 0xA5;

    fn seeded_buf() -> [u8; 1200] {
        let mut buf = [0u8; 1200];
        for (i, b) in buf.iter_mut().enumerate() {
            let mut v = (i % 251) as u8;
            if v == NEEDLE {
                v ^= 0x3C;
            }
            *b = v;
        }
        buf
    }

    #[test]
    fn test_memchr_memrchr_0_to_1024() {
        let source = seeded_buf();

        for n in 0..=1024 {
            let miss_fwd = unsafe { optimized_memchr_unified(source.as_ptr(), n, NEEDLE) };
            let miss_rev = unsafe { optimized_memrchr_unified(source.as_ptr(), n, NEEDLE) };
            assert_eq!(miss_fwd, None, "memchr miss failed at size {n}");
            assert_eq!(miss_rev, None, "memrchr miss failed at size {n}");

            if n == 0 {
                continue;
            }

            let mut first_hit = source;
            first_hit[0] = NEEDLE;
            assert_eq!(
                unsafe { optimized_memchr_unified(first_hit.as_ptr(), n, NEEDLE) },
                Some(0),
                "memchr first-hit failed at size {n}"
            );
            assert_eq!(
                unsafe { optimized_memrchr_unified(first_hit.as_ptr(), n, NEEDLE) },
                Some(0),
                "memrchr first-hit failed at size {n}"
            );

            let mut mid_hit = source;
            let mid = n / 2;
            mid_hit[mid] = NEEDLE;
            assert_eq!(
                unsafe { optimized_memchr_unified(mid_hit.as_ptr(), n, NEEDLE) },
                Some(mid),
                "memchr mid-hit failed at size {n}"
            );
            assert_eq!(
                unsafe { optimized_memrchr_unified(mid_hit.as_ptr(), n, NEEDLE) },
                Some(mid),
                "memrchr mid-hit failed at size {n}"
            );

            let mut mixed = source;
            mixed[0] = NEEDLE;
            mixed[mid] = NEEDLE;
            mixed[n - 1] = NEEDLE;
            assert_eq!(
                unsafe { optimized_memchr_unified(mixed.as_ptr(), n, NEEDLE) },
                Some(0),
                "memchr mixed-hit failed at size {n}"
            );
            assert_eq!(
                unsafe { optimized_memrchr_unified(mixed.as_ptr(), n, NEEDLE) },
                Some(n - 1),
                "memrchr mixed-hit failed at size {n}"
            );
        }
    }

    #[test]
    fn test_memchr_memrchr_alignment() {
        let base = seeded_buf();
        let lengths = [
            1usize, 7, 8, 15, 16, 31, 32, 63, 64, 65, 127, 128, 129, 255, 256, 257,
        ];

        for off in 0..32 {
            for n in lengths {
                let mut local = base;
                let first = off + (n / 3);
                let last = off + n - 1;
                local[first] = NEEDLE;
                local[last] = NEEDLE;

                let ptr = unsafe { local.as_ptr().add(off) };

                assert_eq!(
                    unsafe { optimized_memchr_unified(ptr, n, NEEDLE) },
                    Some(n / 3),
                    "memchr alignment failed off={off} n={n}"
                );
                assert_eq!(
                    unsafe { optimized_memrchr_unified(ptr, n, NEEDLE) },
                    Some(n - 1),
                    "memrchr alignment failed off={off} n={n}"
                );
            }
        }
    }
}
