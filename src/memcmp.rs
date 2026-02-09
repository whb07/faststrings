//! Optimized memcmp with AVX2 scanning.
#![allow(unsafe_code)]

use core::arch::x86_64::*;

/// High-performance memcmp over exactly `n` bytes.
///
/// Returns:
/// - < 0 if first differing byte in `s1` is less than `s2`
/// - 0 if all `n` bytes are equal
/// - > 0 if first differing byte in `s1` is greater than `s2`
///
/// # Safety
///
/// - `s1` and `s2` must be valid for reads of `n` bytes.
#[inline(always)]
pub unsafe fn optimized_memcmp_unified(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    if n <= 31 {
        return unsafe { optimized_memcmp_scalar_wide(s1, s2, n) };
    }

    #[cfg(target_arch = "x86_64")]
    {
        return unsafe { optimized_memcmp_avx2(s1, s2, n) };
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        unsafe { optimized_memcmp_scalar(s1, s2, n) }
    }
}

unsafe fn optimized_memcmp_scalar_wide(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    let mut i = 0usize;

    while i + 8 <= n {
        let a = core::ptr::read_unaligned(s1.add(i) as *const u64);
        let b = core::ptr::read_unaligned(s2.add(i) as *const u64);
        if a != b {
            let mut j = 0usize;
            while j < 8 {
                let idx = i + j;
                let x = *s1.add(idx);
                let y = *s2.add(idx);
                if x != y {
                    return (x as i32) - (y as i32);
                }
                j += 1;
            }
        }
        i += 8;
    }

    while i < n {
        let a = *s1.add(i);
        let b = *s2.add(i);
        if a != b {
            return (a as i32) - (b as i32);
        }
        i += 1;
    }
    0
}

#[target_feature(enable = "avx2")]
unsafe fn first_diff_32(s1: *const u8, s2: *const u8) -> usize {
    let a = _mm256_loadu_si256(s1 as *const __m256i);
    let b = _mm256_loadu_si256(s2 as *const __m256i);
    let eq = _mm256_cmpeq_epi8(a, b);
    let mask = _mm256_movemask_epi8(eq) as u32;
    (!mask).trailing_zeros() as usize
}

#[target_feature(enable = "avx2")]
unsafe fn optimized_memcmp_avx2(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    let mut i = 0usize;

    while i + 128 <= n {
        let a0 = _mm256_loadu_si256(s1.add(i) as *const __m256i);
        let b0 = _mm256_loadu_si256(s2.add(i) as *const __m256i);
        let x0 = _mm256_xor_si256(a0, b0);
        if _mm256_testz_si256(x0, x0) == 0 {
            let d = first_diff_32(s1.add(i), s2.add(i));
            let idx = i + d;
            return (*s1.add(idx) as i32) - (*s2.add(idx) as i32);
        }

        let a1 = _mm256_loadu_si256(s1.add(i + 32) as *const __m256i);
        let b1 = _mm256_loadu_si256(s2.add(i + 32) as *const __m256i);
        let x1 = _mm256_xor_si256(a1, b1);
        if _mm256_testz_si256(x1, x1) == 0 {
            let d = first_diff_32(s1.add(i + 32), s2.add(i + 32));
            let idx = i + 32 + d;
            return (*s1.add(idx) as i32) - (*s2.add(idx) as i32);
        }

        let a2 = _mm256_loadu_si256(s1.add(i + 64) as *const __m256i);
        let b2 = _mm256_loadu_si256(s2.add(i + 64) as *const __m256i);
        let x2 = _mm256_xor_si256(a2, b2);
        if _mm256_testz_si256(x2, x2) == 0 {
            let d = first_diff_32(s1.add(i + 64), s2.add(i + 64));
            let idx = i + 64 + d;
            return (*s1.add(idx) as i32) - (*s2.add(idx) as i32);
        }

        let a3 = _mm256_loadu_si256(s1.add(i + 96) as *const __m256i);
        let b3 = _mm256_loadu_si256(s2.add(i + 96) as *const __m256i);
        let x3 = _mm256_xor_si256(a3, b3);
        if _mm256_testz_si256(x3, x3) == 0 {
            let d = first_diff_32(s1.add(i + 96), s2.add(i + 96));
            let idx = i + 96 + d;
            return (*s1.add(idx) as i32) - (*s2.add(idx) as i32);
        }

        i += 128;
    }

    while i + 32 <= n {
        let a = _mm256_loadu_si256(s1.add(i) as *const __m256i);
        let b = _mm256_loadu_si256(s2.add(i) as *const __m256i);
        let x = _mm256_xor_si256(a, b);
        if _mm256_testz_si256(x, x) == 0 {
            let d = first_diff_32(s1.add(i), s2.add(i));
            let idx = i + d;
            return (*s1.add(idx) as i32) - (*s2.add(idx) as i32);
        }
        i += 32;
    }

    optimized_memcmp_scalar_wide(s1.add(i), s2.add(i), n - i)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ref_memcmp(a: &[u8], b: &[u8]) -> i32 {
        for (&x, &y) in a.iter().zip(b.iter()) {
            if x != y {
                return (x as i32) - (y as i32);
            }
        }
        0
    }

    #[test]
    fn test_memcmp_exact_0_to_1024() {
        let mut a = [0u8; 1024];
        let mut b = [0u8; 1024];
        for (i, byte) in a.iter_mut().enumerate() {
            *byte = (i % 251) as u8;
        }
        b.copy_from_slice(&a);

        for n in 0..=1024 {
            let got = unsafe { optimized_memcmp_unified(a.as_ptr(), b.as_ptr(), n) };
            assert_eq!(got, 0, "equal case failed at size {n}");

            if n > 0 {
                let mut local = b;
                local[n - 1] ^= 0xFF;
                let got_diff = unsafe { optimized_memcmp_unified(a.as_ptr(), local.as_ptr(), n) };
                let expect_diff = ref_memcmp(&a[..n], &local[..n]);
                assert_eq!(got_diff, expect_diff, "diff-last failed at size {n}");
            }
        }
    }

    #[test]
    fn test_memcmp_alignment_and_positions() {
        let mut a_full = [0u8; 1200];
        let mut b_full = [0u8; 1200];
        for (i, byte) in a_full.iter_mut().enumerate() {
            *byte = (i % 251) as u8;
        }
        b_full.copy_from_slice(&a_full);

        for a_off in 0..32 {
            for b_off in 0..32 {
                for n in [1usize, 7, 8, 15, 16, 31, 32, 63, 64, 65, 127, 128, 129, 255, 256, 257]
                {
                    let a = &a_full[a_off..a_off + n];
                    let mut b = b_full;
                    let s = &mut b[b_off..b_off + n];
                    s.copy_from_slice(a);

                    let equal = unsafe { optimized_memcmp_unified(a.as_ptr(), s.as_ptr(), n) };
                    assert_eq!(equal, 0, "equal failed a_off={a_off} b_off={b_off} n={n}");

                    s[n / 2] ^= 0x5A;
                    let got = unsafe { optimized_memcmp_unified(a.as_ptr(), s.as_ptr(), n) };
                    let expect = ref_memcmp(a, s);
                    assert_eq!(
                        got, expect,
                        "mid-diff failed a_off={a_off} b_off={b_off} n={n}"
                    );
                }
            }
        }
    }
}
