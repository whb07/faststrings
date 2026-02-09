//! Memory manipulation functions
//!
//! Safe Rust implementations of memory functions operating on byte slices.

/// Copy bytes from source to destination (non-overlapping)
///
/// Copies bytes from `src` to `dest`. Returns the number of bytes copied,
/// which is `min(dest.len(), src.len())`.
///
/// Note: For overlapping regions, the behavior matches memcpy (undefined in C),
/// but in safe Rust this is well-defined. Use `memmove` for explicit overlap handling.
///
/// # Examples
/// ```
/// use faststrings::mem::memcpy;
/// let mut dest = [0u8; 5];
/// let src = b"hello";
/// assert_eq!(memcpy(&mut dest, src), 5);
/// assert_eq!(&dest, b"hello");
/// ```
pub fn memcpy(dest: &mut [u8], src: &[u8]) -> usize {
    let n = dest.len().min(src.len());
    dest[..n].copy_from_slice(&src[..n]);
    n
}

/// Copy bytes from source to destination (overlapping safe)
///
/// Copies bytes from `src` to `dest`, correctly handling overlapping regions.
/// Returns the number of bytes copied.
///
/// In safe Rust with slices, we cannot have overlapping mutable and immutable
/// references, so this behaves identically to `memcpy`.
pub fn memmove(dest: &mut [u8], src: &[u8]) -> usize {
    let n = dest.len().min(src.len());
    dest[..n].copy_from_slice(&src[..n]);
    n
}

/// Fill a byte slice with a constant value
///
/// Sets all bytes in `dest` to the value `c`. Returns the number of bytes set.
///
/// # Examples
/// ```
/// use faststrings::mem::memset;
/// let mut buf = [0u8; 5];
/// memset(&mut buf, b'x');
/// assert_eq!(&buf, b"xxxxx");
/// ```
pub fn memset(dest: &mut [u8], c: u8) -> usize {
    dest.fill(c);
    dest.len()
}

/// Compare two byte slices
///
/// Compares byte slices lexicographically.
///
/// Returns:
/// - < 0 if s1 < s2
/// - 0 if s1 == s2
/// - > 0 if s1 > s2
///
/// Comparison is performed up to `min(s1.len(), s2.len())` bytes.
/// If all compared bytes are equal but lengths differ, the shorter one is "less".
///
/// # Examples
/// ```
/// use faststrings::mem::memcmp;
/// assert!(memcmp(b"abc", b"abd") < 0);
/// assert!(memcmp(b"abc", b"abc") == 0);
/// assert!(memcmp(b"abd", b"abc") > 0);
/// ```
pub fn memcmp(s1: &[u8], s2: &[u8]) -> i32 {
    let n = s1.len().min(s2.len());

    let cmp = unsafe { crate::memcmp::optimized_memcmp_unified(s1.as_ptr(), s2.as_ptr(), n) };
    if cmp != 0 {
        return cmp;
    }

    // If all bytes equal, compare by length
    match s1.len().cmp(&s2.len()) {
        core::cmp::Ordering::Less => -1,
        core::cmp::Ordering::Equal => 0,
        core::cmp::Ordering::Greater => 1,
    }
}

/// Compare exactly n bytes of two slices
///
/// Like memcmp, but only compares up to `n` bytes, regardless of slice lengths.
/// If either slice is shorter than `n`, comparison stops at the shorter length.
pub fn memcmp_n(s1: &[u8], s2: &[u8], n: usize) -> i32 {
    let len = s1.len().min(s2.len()).min(n);
    unsafe { crate::memcmp::optimized_memcmp_unified(s1.as_ptr(), s2.as_ptr(), len) }
}

/// Scan a byte slice for a character
///
/// Returns the index of the first occurrence of `c` in `s`, or `None` if not found.
///
/// # Examples
/// ```
/// use faststrings::mem::memchr;
/// assert_eq!(memchr(b"hello", b'l'), Some(2));
/// assert_eq!(memchr(b"hello", b'x'), None);
/// ```
pub fn memchr(s: &[u8], c: u8) -> Option<usize> {
    unsafe { crate::memchr::optimized_memchr_unified(s.as_ptr(), s.len(), c) }
}

/// Scan a byte slice backward for a character
///
/// Returns the index of the last occurrence of `c` in `s`, or `None` if not found.
///
/// # Examples
/// ```
/// use faststrings::mem::memrchr;
/// assert_eq!(memrchr(b"hello", b'l'), Some(3));
/// assert_eq!(memrchr(b"hello", b'x'), None);
/// ```
pub fn memrchr(s: &[u8], c: u8) -> Option<usize> {
    unsafe { crate::memchr::optimized_memrchr_unified(s.as_ptr(), s.len(), c) }
}

/// Copy bytes until a character is found
///
/// Copies bytes from `src` to `dest` until the character `c` is found or
/// all bytes are copied. Returns `Some(index + 1)` if `c` was found (pointing
/// past the copied `c`), or `None` if not found.
///
/// # Examples
/// ```
/// use faststrings::mem::memccpy;
/// let mut dest = [0u8; 10];
/// let src = b"hello world";
/// assert_eq!(memccpy(&mut dest, src, b' '), Some(6)); // copied "hello "
/// ```
pub fn memccpy(dest: &mut [u8], src: &[u8], c: u8) -> Option<usize> {
    let n = dest.len().min(src.len());
    if n == 0 {
        return None;
    }

    let stop = memchr(&src[..n], c);
    let copy_len = stop.map_or(n, |idx| idx + 1);

    // SAFETY: Both pointers come from valid slices and `copy_len <= n <= len`.
    unsafe {
        crate::memcpy::optimized_memcpy_unified(dest.as_mut_ptr(), src.as_ptr(), copy_len);
    }

    stop.map(|idx| idx + 1)
}

/// Find a substring in a byte slice
///
/// Returns the starting index of the first occurrence of `needle` in `haystack`,
/// or `None` if not found.
///
/// # Examples
/// ```
/// use faststrings::mem::memmem;
/// assert_eq!(memmem(b"hello world", b"wor"), Some(6));
/// assert_eq!(memmem(b"hello world", b"xyz"), None);
/// assert_eq!(memmem(b"hello", b""), Some(0)); // empty needle
/// ```
pub fn memmem(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }

    if needle.len() > haystack.len() {
        return None;
    }

    if needle.len() == 1 {
        return memchr(haystack, needle[0]);
    }

    let needle_len = needle.len();
    let first = needle[0];
    let last = needle[needle_len - 1];
    let max_start = haystack.len() - needle_len;
    let mut search_start = 0usize;

    while search_start <= max_start {
        let rel = match memchr(&haystack[search_start..max_start + 1], first) {
            Some(pos) => pos,
            None => return None,
        };
        let idx = search_start + rel;

        if haystack[idx + needle_len - 1] == last
            && memcmp_n(&haystack[idx..idx + needle_len], needle, needle_len) == 0
        {
            return Some(idx);
        }

        search_start = idx + 1;
    }

    None
}

/// Set memory to zero (secure-ish in safe Rust)
///
/// Zeros out the byte slice. In safe Rust, we cannot guarantee this won't
/// be optimized away, but the optimizer is less aggressive with slice operations.
///
/// For truly secure zeroing, the unsafe FFI layer can use `write_volatile`.
pub fn explicit_bzero(s: &mut [u8]) {
    s.fill(0);
    // In safe Rust, we can't use write_volatile or compiler_fence
    // The FFI layer will need to handle secure zeroing
}

/// Zero bytes (BSD)
///
/// Writes zeroed bytes to the slice. Equivalent to `memset(s, 0)`.
pub fn bzero(s: &mut [u8]) {
    s.fill(0);
}

/// Compare bytes (BSD)
///
/// Compares two byte slices. Equivalent to `memcmp`.
pub fn bcmp(s1: &[u8], s2: &[u8]) -> i32 {
    memcmp(s1, s2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memcpy_memmove() {
        let mut dest = [0u8; 5];
        let src = *b"hello";
        assert_eq!(memcpy(&mut dest, &src), 5);
        assert_eq!(dest, src);

        let mut dest2 = [0u8; 3];
        assert_eq!(memmove(&mut dest2, &src), 3);
        assert_eq!(&dest2, b"hel");
    }

    #[test]
    fn test_memcmp_variants() {
        assert_eq!(memcmp(b"abc", b"abc"), 0);
        assert!(memcmp(b"abc", b"abd") < 0);
        assert!(memcmp(b"abe", b"abd") > 0);
        assert_eq!(memcmp_n(b"abc", b"abd", 2), 0);
    }

    #[test]
    fn test_memchr_memrchr() {
        assert_eq!(memchr(b"hello", b'l'), Some(2));
        assert_eq!(memrchr(b"hello", b'l'), Some(3));
        assert_eq!(memchr(b"hello", b'x'), None);
    }

    #[test]
    fn test_memccpy_memmem() {
        let mut dest = [0u8; 10];
        assert_eq!(memccpy(&mut dest, b"hello world", b' '), Some(6));
        assert_eq!(&dest[..6], b"hello ");

        assert_eq!(memmem(b"hello world", b"wor"), Some(6));
        assert_eq!(memmem(b"hello world", b"xyz"), None);
        assert_eq!(memmem(b"hello", b""), Some(0));
    }

    #[test]
    fn test_bzero_and_bcmp() {
        let mut buf = [1u8, 2, 3];
        bzero(&mut buf);
        assert_eq!(buf, [0u8; 3]);
        assert_eq!(bcmp(b"abc", b"abc"), 0);
    }

    #[test]
    fn test_explicit_bzero() {
        let mut buf = [5u8, 6, 7];
        explicit_bzero(&mut buf);
        assert_eq!(buf, [0u8; 3]);
    }
}
