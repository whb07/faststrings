//! String searching functions
//!
//! Safe Rust implementations of string search functions.

use crate::str::strlen;

#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::*;

#[inline(always)]
fn build_byte_bitmap(set: &[u8]) -> [u64; 4] {
    let mut bitmap = [0u64; 4];
    for &byte in set {
        let slot = (byte >> 6) as usize;
        let bit = 1u64 << (byte & 63);
        bitmap[slot] |= bit;
    }
    bitmap
}

#[inline(always)]
fn bitmap_contains(bitmap: &[u64; 4], byte: u8) -> bool {
    let slot = (byte >> 6) as usize;
    let bit = 1u64 << (byte & 63);
    (bitmap[slot] & bit) != 0
}

#[inline(always)]
fn contains_small_set(set: &[u8], c: u8) -> bool {
    match set {
        [a] => c == *a,
        [a, b] => c == *a || c == *b,
        [a, b, d] => c == *a || c == *b || c == *d,
        [a, b, d, e] => c == *a || c == *b || c == *d || c == *e,
        _ => false,
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn strspn_small_set_avx2(s: &[u8], accept: &[u8]) -> usize {
    let len = s.len();
    let mut i = 0usize;

    let a0 = _mm256_set1_epi8(accept[0] as i8);
    let a1 = if accept.len() >= 2 {
        _mm256_set1_epi8(accept[1] as i8)
    } else {
        _mm256_setzero_si256()
    };
    let a2 = if accept.len() >= 3 {
        _mm256_set1_epi8(accept[2] as i8)
    } else {
        _mm256_setzero_si256()
    };
    let a3 = if accept.len() >= 4 {
        _mm256_set1_epi8(accept[3] as i8)
    } else {
        _mm256_setzero_si256()
    };

    while i + 32 <= len {
        let chunk = _mm256_loadu_si256(s.as_ptr().add(i) as *const __m256i);

        let mut eq = _mm256_cmpeq_epi8(chunk, a0);
        if accept.len() >= 2 {
            eq = _mm256_or_si256(eq, _mm256_cmpeq_epi8(chunk, a1));
        }
        if accept.len() >= 3 {
            eq = _mm256_or_si256(eq, _mm256_cmpeq_epi8(chunk, a2));
        }
        if accept.len() >= 4 {
            eq = _mm256_or_si256(eq, _mm256_cmpeq_epi8(chunk, a3));
        }

        let mask = _mm256_movemask_epi8(eq) as u32;
        if mask != u32::MAX {
            let first_bad = (!mask).trailing_zeros() as usize;
            return i + first_bad;
        }
        i += 32;
    }

    while i < len {
        if !contains_small_set(accept, s[i]) {
            return i;
        }
        i += 1;
    }

    len
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn find_first_any_small_set_avx2(s: &[u8], set: &[u8]) -> Option<usize> {
    let len = s.len();
    let mut i = 0usize;

    let a0 = _mm256_set1_epi8(set[0] as i8);
    let a1 = if set.len() >= 2 {
        _mm256_set1_epi8(set[1] as i8)
    } else {
        _mm256_setzero_si256()
    };
    let a2 = if set.len() >= 3 {
        _mm256_set1_epi8(set[2] as i8)
    } else {
        _mm256_setzero_si256()
    };
    let a3 = if set.len() >= 4 {
        _mm256_set1_epi8(set[3] as i8)
    } else {
        _mm256_setzero_si256()
    };

    while i + 32 <= len {
        let chunk = _mm256_loadu_si256(s.as_ptr().add(i) as *const __m256i);

        let mut eq = _mm256_cmpeq_epi8(chunk, a0);
        if set.len() >= 2 {
            eq = _mm256_or_si256(eq, _mm256_cmpeq_epi8(chunk, a1));
        }
        if set.len() >= 3 {
            eq = _mm256_or_si256(eq, _mm256_cmpeq_epi8(chunk, a2));
        }
        if set.len() >= 4 {
            eq = _mm256_or_si256(eq, _mm256_cmpeq_epi8(chunk, a3));
        }

        let mask = _mm256_movemask_epi8(eq) as u32;
        if mask != 0 {
            return Some(i + mask.trailing_zeros() as usize);
        }
        i += 32;
    }

    while i < len {
        if contains_small_set(set, s[i]) {
            return Some(i);
        }
        i += 1;
    }

    None
}

/// Locate character in null-terminated string
///
/// Returns the index of the first occurrence of `c` in `s` (up to the null terminator),
/// or `None` if not found.
///
/// # Examples
/// ```
/// use faststrings::search::strchr;
/// assert_eq!(strchr(b"hello\0world", b'l'), Some(2));
/// assert_eq!(strchr(b"hello\0world", b'w'), None); // 'w' is after null
/// assert_eq!(strchr(b"hello\0", b'\0'), Some(5)); // can find null
/// ```
pub fn strchr(s: &[u8], c: u8) -> Option<usize> {
    let len = if c == 0 {
        // If searching for null, search the whole slice
        s.len()
    } else {
        // Otherwise, search up to and including the null terminator
        strlen(s) + 1
    };

    let search_len = len.min(s.len());
    crate::mem::memchr(&s[..search_len], c)
}

/// Locate character in string (returns length if not found)
///
/// Like strchr, but returns the string length (position of null terminator)
/// if `c` is not found, rather than None.
pub fn strchrnul(s: &[u8], c: u8) -> usize {
    let len = strlen(s);
    let search_len = len.min(s.len());

    crate::mem::memchr(&s[..search_len], c).unwrap_or(len)
}

/// Locate character in string (from end)
///
/// Returns the index of the last occurrence of `c` in `s` (up to the null terminator),
/// or `None` if not found.
///
/// # Examples
/// ```
/// use faststrings::search::strrchr;
/// assert_eq!(strrchr(b"hello\0", b'l'), Some(3));
/// assert_eq!(strrchr(b"hello\0", b'x'), None);
/// ```
pub fn strrchr(s: &[u8], c: u8) -> Option<usize> {
    let len = strlen(s);
    let search_len = if c == 0 { len + 1 } else { len }.min(s.len());

    crate::mem::memrchr(&s[..search_len], c)
}

/// Locate substring
///
/// Finds the first occurrence of the null-terminated string `needle` in
/// the null-terminated string `haystack`.
///
/// # Examples
/// ```
/// use faststrings::search::strstr;
/// assert_eq!(strstr(b"hello world\0", b"wor\0"), Some(6));
/// assert_eq!(strstr(b"hello world\0", b"xyz\0"), None);
/// assert_eq!(strstr(b"hello\0", b"\0"), Some(0)); // empty needle
/// ```
pub fn strstr(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    let h_len = strlen(haystack);
    let n_len = strlen(needle);

    if n_len == 0 {
        return Some(0);
    }

    if n_len > h_len {
        return None;
    }

    let haystack = &haystack[..h_len.min(haystack.len())];
    let needle = &needle[..n_len.min(needle.len())];

    let end = h_len - n_len + 1;

    (0..end).find(|&i| &haystack[i..i + n_len] == needle)
}

/// Locate substring (case-insensitive)
///
/// Like strstr, but ignores case when comparing.
pub fn strcasestr(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    let h_len = strlen(haystack);
    let n_len = strlen(needle);

    if n_len == 0 {
        return Some(0);
    }

    if n_len > h_len {
        return None;
    }

    let haystack = &haystack[..h_len.min(haystack.len())];
    let needle = &needle[..n_len.min(needle.len())];

    let end = h_len - n_len + 1;

    'outer: for i in 0..end {
        for j in 0..n_len {
            let hc = to_lower_ascii(haystack[i + j]);
            let nc = to_lower_ascii(needle[j]);
            if hc != nc {
                continue 'outer;
            }
        }
        return Some(i);
    }

    None
}

/// Get length of prefix consisting of accepted characters
///
/// Returns the length of the initial segment of `s` which consists
/// entirely of characters in `accept`.
///
/// # Examples
/// ```
/// use faststrings::search::strspn;
/// assert_eq!(strspn(b"hello\0", b"ehlo\0"), 5);
/// assert_eq!(strspn(b"hello\0", b"xyz\0"), 0);
/// ```
pub fn strspn(s: &[u8], accept: &[u8]) -> usize {
    let s_len = strlen(s);
    let accept_len = strlen(accept);
    let s = &s[..s_len.min(s.len())];
    let accept = &accept[..accept_len.min(accept.len())];

    if accept.is_empty() {
        return 0;
    }

    if accept.len() <= 4 {
        #[cfg(target_arch = "x86_64")]
        if s.len() >= 32 {
            // SAFETY: AVX2 is baseline for this project.
            return unsafe { strspn_small_set_avx2(s, accept) };
        }

        for (i, &c) in s.iter().enumerate() {
            if !contains_small_set(accept, c) {
                return i;
            }
        }
        return s.len();
    }

    let bitmap = build_byte_bitmap(accept);
    for (i, &c) in s.iter().enumerate() {
        if !bitmap_contains(&bitmap, c) {
            return i;
        }
    }

    s.len()
}

/// Get length of prefix not containing rejected characters
///
/// Returns the length of the initial segment of `s` which does not
/// contain any of the characters in `reject`.
///
/// # Examples
/// ```
/// use faststrings::search::strcspn;
/// assert_eq!(strcspn(b"hello\0", b"lo\0"), 2); // "he" before 'l'
/// assert_eq!(strcspn(b"hello\0", b"xyz\0"), 5);
/// ```
pub fn strcspn(s: &[u8], reject: &[u8]) -> usize {
    let s_len = strlen(s);
    let reject_len = strlen(reject);
    let s = &s[..s_len.min(s.len())];
    let reject = &reject[..reject_len.min(reject.len())];

    if reject.is_empty() {
        return s.len();
    }

    if reject.len() == 1 {
        return crate::mem::memchr(s, reject[0]).unwrap_or(s.len());
    }

    if reject.len() <= 4 {
        #[cfg(target_arch = "x86_64")]
        if s.len() >= 64 {
            // SAFETY: AVX2 is baseline for this project.
            return unsafe { find_first_any_small_set_avx2(s, reject) }.unwrap_or(s.len());
        }

        for (i, &c) in s.iter().enumerate() {
            if contains_small_set(reject, c) {
                return i;
            }
        }
        return s.len();
    }

    let bitmap = build_byte_bitmap(reject);
    for (i, &c) in s.iter().enumerate() {
        if bitmap_contains(&bitmap, c) {
            return i;
        }
    }

    s.len()
}

/// Search string for any of a set of characters
///
/// Locates the first occurrence in `s` of any character in `accept`.
/// Returns the index, or `None` if not found.
///
/// # Examples
/// ```
/// use faststrings::search::strpbrk;
/// assert_eq!(strpbrk(b"hello\0", b"lo\0"), Some(2)); // 'l' at index 2
/// assert_eq!(strpbrk(b"hello\0", b"xyz\0"), None);
/// ```
pub fn strpbrk(s: &[u8], accept: &[u8]) -> Option<usize> {
    let s_len = strlen(s);
    let accept_len = strlen(accept);
    let s = &s[..s_len.min(s.len())];
    let accept = &accept[..accept_len.min(accept.len())];

    if accept.is_empty() {
        return None;
    }

    if accept.len() == 1 {
        return crate::mem::memchr(s, accept[0]);
    }

    if accept.len() <= 4 {
        #[cfg(target_arch = "x86_64")]
        if s.len() >= 64 {
            // SAFETY: AVX2 is baseline for this project.
            return unsafe { find_first_any_small_set_avx2(s, accept) };
        }

        for (i, &c) in s.iter().enumerate() {
            if contains_small_set(accept, c) {
                return Some(i);
            }
        }
        return None;
    }

    let bitmap = build_byte_bitmap(accept);
    for (i, &c) in s.iter().enumerate() {
        if bitmap_contains(&bitmap, c) {
            return Some(i);
        }
    }

    None
}

/// Locate byte in string (BSD alias for strchr)
pub fn index(s: &[u8], c: u8) -> Option<usize> {
    strchr(s, c)
}

/// Locate byte in string from end (BSD alias for strrchr)
pub fn rindex(s: &[u8], c: u8) -> Option<usize> {
    strrchr(s, c)
}

// Helper for case-insensitive comparison
fn to_lower_ascii(c: u8) -> u8 {
    c.to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strchr_and_strchrnul() {
        assert_eq!(strchr(b"hello\0", b'l'), Some(2));
        assert_eq!(strchr(b"hello\0", b'w'), None);
        assert_eq!(strchr(b"hello\0", b'\0'), Some(5));
        assert_eq!(strchrnul(b"hello\0", b'l'), 2);
        assert_eq!(strchrnul(b"hello\0", b'x'), 5);
    }

    #[test]
    fn test_strrchr_and_aliases() {
        assert_eq!(strrchr(b"hello\0", b'l'), Some(3));
        assert_eq!(index(b"hello\0", b'h'), Some(0));
        assert_eq!(rindex(b"hello\0", b'h'), Some(0));
    }

    #[test]
    fn test_strstr_and_strcasestr() {
        assert_eq!(strstr(b"hello world\0", b"wor\0"), Some(6));
        assert_eq!(strstr(b"hello\0", b"\0"), Some(0));
        assert_eq!(strcasestr(b"Hello\0", b"hEl\0"), Some(0));
    }

    #[test]
    fn test_strspn_strcspn_strpbrk() {
        assert_eq!(strspn(b"hello\0", b"ehlo\0"), 5);
        assert_eq!(strcspn(b"hello\0", b"lo\0"), 2);
        assert_eq!(strpbrk(b"hello\0", b"xyz\0"), None);
        assert_eq!(strpbrk(b"hello\0", b"lo\0"), Some(2));
    }
}
