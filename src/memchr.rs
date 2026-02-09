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

#[inline(always)]
fn has_zero_byte_u64(x: u64) -> bool {
    ((x.wrapping_sub(0x0101_0101_0101_0101)) & !x & 0x8080_8080_8080_8080) != 0
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
    if unsafe { *s } == needle {
        return Some(0);
    }
    if n == 1 {
        return None;
    }

    #[cfg(target_arch = "x86_64")]
    {
        return unsafe { optimized_memchr_avx2(s.add(1), n - 1, needle).map(|i| i + 1) };
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        unsafe { optimized_memchr_scalar(s.add(1), n - 1, needle).map(|i| i + 1) }
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
    let last = n - 1;
    if unsafe { *s.add(last) } == needle {
        return Some(last);
    }
    if n == 1 {
        return None;
    }

    #[cfg(target_arch = "x86_64")]
    {
        return unsafe { optimized_memrchr_avx2(s, n - 1, needle) };
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        unsafe { optimized_memrchr_scalar(s, n - 1, needle) }
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

#[inline(always)]
unsafe fn optimized_memchr_scalar_wide(s: *const u8, n: usize, needle: u8) -> Option<usize> {
    let needle_64 = u64::from_ne_bytes([needle; 8]);
    let mut i = 0usize;

    while i + 8 <= n {
        let word = core::ptr::read_unaligned(s.add(i) as *const u64);
        if has_zero_byte_u64(word ^ needle_64) {
            let mut j = 0usize;
            while j < 8 {
                if *s.add(i + j) == needle {
                    return Some(i + j);
                }
                j += 1;
            }
        }
        i += 8;
    }

    optimized_memchr_scalar(s.add(i), n - i, needle).map(|tail| i + tail)
}

#[inline(always)]
unsafe fn optimized_memrchr_scalar_wide(s: *const u8, n: usize, needle: u8) -> Option<usize> {
    let needle_64 = u64::from_ne_bytes([needle; 8]);
    let mut i = n;

    while i >= 8 {
        i -= 8;
        let word = core::ptr::read_unaligned(s.add(i) as *const u64);
        if has_zero_byte_u64(word ^ needle_64) {
            let mut j = 8usize;
            while j > 0 {
                j -= 1;
                if *s.add(i + j) == needle {
                    return Some(i + j);
                }
            }
        }
    }

    optimized_memrchr_scalar(s, i, needle)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn optimized_memchr_avx2(s: *const u8, n: usize, needle: u8) -> Option<usize> {
    if n < 64 {
        return unsafe { optimized_memchr_small_avx2(s, n, needle) };
    }

    let needle_v = _mm256_set1_epi8(needle as i8);
    let mut i = 0usize;

    while i + 128 <= n {
        let p0 = unsafe { s.add(i) };
        let p1 = unsafe { s.add(i + 32) };
        let p2 = unsafe { s.add(i + 64) };
        let p3 = unsafe { s.add(i + 96) };

        let v0 = _mm256_loadu_si256(p0 as *const __m256i);
        let v1 = _mm256_loadu_si256(p1 as *const __m256i);
        let v2 = _mm256_loadu_si256(p2 as *const __m256i);
        let v3 = _mm256_loadu_si256(p3 as *const __m256i);
        let eq0 = _mm256_cmpeq_epi8(v0, needle_v);
        let eq1 = _mm256_cmpeq_epi8(v1, needle_v);
        let eq2 = _mm256_cmpeq_epi8(v2, needle_v);
        let eq3 = _mm256_cmpeq_epi8(v3, needle_v);
        let any = _mm256_or_si256(_mm256_or_si256(eq0, eq1), _mm256_or_si256(eq2, eq3));

        if _mm256_testz_si256(any, any) == 1 {
            i += 128;
            continue;
        }

        let m0 = _mm256_movemask_epi8(eq0);
        if m0 != 0 {
            return Some(i + first_set_bit(m0));
        }

        let m1 = _mm256_movemask_epi8(eq1);
        if m1 != 0 {
            return Some(i + 32 + first_set_bit(m1));
        }

        let m2 = _mm256_movemask_epi8(eq2);
        if m2 != 0 {
            return Some(i + 64 + first_set_bit(m2));
        }

        let m3 = _mm256_movemask_epi8(eq3);
        if m3 != 0 {
            return Some(i + 96 + first_set_bit(m3));
        }
    }

    while i + 32 <= n {
        let p = s.add(i);
        let v = _mm256_loadu_si256(p as *const __m256i);
        let eq = _mm256_cmpeq_epi8(v, needle_v);
        let m = _mm256_movemask_epi8(eq);
        if m != 0 {
            return Some(i + first_set_bit(m));
        }
        i += 32;
    }

    unsafe { optimized_memchr_scalar_wide(s.add(i), n - i, needle).map(|tail| i + tail) }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn optimized_memchr_small_avx2(s: *const u8, n: usize, needle: u8) -> Option<usize> {
    debug_assert!(n > 0 && n < 64);

    if n >= 32 {
        let needle_v = _mm256_set1_epi8(needle as i8);
        let v0 = _mm256_loadu_si256(s as *const __m256i);
        let m0 = _mm256_movemask_epi8(_mm256_cmpeq_epi8(v0, needle_v));
        if m0 != 0 {
            return Some(first_set_bit(m0));
        }
        if n == 32 {
            return None;
        }
        let off = n - 32;
        let v1 = _mm256_loadu_si256(s.add(off) as *const __m256i);
        let m1 = _mm256_movemask_epi8(_mm256_cmpeq_epi8(v1, needle_v));
        if m1 != 0 {
            return Some(off + first_set_bit(m1));
        }
        return None;
    }

    if n >= 16 {
        let needle_v = _mm_set1_epi8(needle as i8);
        let v0 = _mm_loadu_si128(s as *const __m128i);
        let m0 = _mm_movemask_epi8(_mm_cmpeq_epi8(v0, needle_v));
        if m0 != 0 {
            return Some((m0 as u32).trailing_zeros() as usize);
        }
        if n == 16 {
            return None;
        }
        let off = n - 16;
        let v1 = _mm_loadu_si128(s.add(off) as *const __m128i);
        let m1 = _mm_movemask_epi8(_mm_cmpeq_epi8(v1, needle_v));
        if m1 != 0 {
            return Some(off + (m1 as u32).trailing_zeros() as usize);
        }
        return None;
    }

    optimized_memchr_scalar(s, n, needle)
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn optimized_memrchr_avx2(s: *const u8, n: usize, needle: u8) -> Option<usize> {
    if n < 64 {
        return unsafe { optimized_memrchr_small_avx2(s, n, needle) };
    }

    let needle_v = _mm256_set1_epi8(needle as i8);
    let mut i = n;

    while i >= 128 {
        let base = i - 128;

        let p0 = s.add(base);
        let p1 = s.add(base + 32);
        let p2 = s.add(base + 64);
        let p3 = s.add(base + 96);

        let v0 = _mm256_loadu_si256(p0 as *const __m256i);
        let v1 = _mm256_loadu_si256(p1 as *const __m256i);
        let v2 = _mm256_loadu_si256(p2 as *const __m256i);
        let v3 = _mm256_loadu_si256(p3 as *const __m256i);
        let eq0 = _mm256_cmpeq_epi8(v0, needle_v);
        let eq1 = _mm256_cmpeq_epi8(v1, needle_v);
        let eq2 = _mm256_cmpeq_epi8(v2, needle_v);
        let eq3 = _mm256_cmpeq_epi8(v3, needle_v);
        let any = _mm256_or_si256(_mm256_or_si256(eq0, eq1), _mm256_or_si256(eq2, eq3));

        if _mm256_testz_si256(any, any) == 1 {
            i = base;
            continue;
        }

        let m3 = _mm256_movemask_epi8(eq3);
        if m3 != 0 {
            return Some(base + 96 + last_set_bit(m3));
        }

        let m2 = _mm256_movemask_epi8(eq2);
        if m2 != 0 {
            return Some(base + 64 + last_set_bit(m2));
        }

        let m1 = _mm256_movemask_epi8(eq1);
        if m1 != 0 {
            return Some(base + 32 + last_set_bit(m1));
        }

        let m0 = _mm256_movemask_epi8(eq0);
        if m0 != 0 {
            return Some(base + last_set_bit(m0));
        }

        debug_assert!(_mm256_testz_si256(any, any) == 0);
    }

    while i >= 32 {
        let base = i - 32;
        let p = s.add(base);
        let v = _mm256_loadu_si256(p as *const __m256i);
        let eq = _mm256_cmpeq_epi8(v, needle_v);
        let m = _mm256_movemask_epi8(eq);
        if m != 0 {
            return Some(base + last_set_bit(m));
        }
        i = base;
    }

    unsafe { optimized_memrchr_scalar_wide(s, i, needle) }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn optimized_memrchr_small_avx2(s: *const u8, n: usize, needle: u8) -> Option<usize> {
    debug_assert!(n > 0 && n < 64);

    if n >= 32 {
        let needle_v = _mm256_set1_epi8(needle as i8);
        let off = n - 32;
        let v1 = _mm256_loadu_si256(s.add(off) as *const __m256i);
        let m1 = _mm256_movemask_epi8(_mm256_cmpeq_epi8(v1, needle_v));
        if m1 != 0 {
            return Some(off + last_set_bit(m1));
        }
        if n == 32 {
            return None;
        }
        let v0 = _mm256_loadu_si256(s as *const __m256i);
        let m0 = _mm256_movemask_epi8(_mm256_cmpeq_epi8(v0, needle_v));
        if m0 != 0 {
            return Some(last_set_bit(m0));
        }
        return None;
    }

    if n >= 16 {
        let needle_v = _mm_set1_epi8(needle as i8);
        let off = n - 16;
        let v1 = _mm_loadu_si128(s.add(off) as *const __m128i);
        let m1 = _mm_movemask_epi8(_mm_cmpeq_epi8(v1, needle_v));
        if m1 != 0 {
            return Some(off + (31 - (m1 as u32).leading_zeros()) as usize);
        }
        if n == 16 {
            return None;
        }
        let v0 = _mm_loadu_si128(s as *const __m128i);
        let m0 = _mm_movemask_epi8(_mm_cmpeq_epi8(v0, needle_v));
        if m0 != 0 {
            return Some((31 - (m0 as u32).leading_zeros()) as usize);
        }
        return None;
    }

    optimized_memrchr_scalar(s, n, needle)
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
