//! Wide memory manipulation functions
//!
//! Safe Rust implementations of wide memory functions.
//! Wide characters are represented as wchar_t slices.

#![allow(unsafe_code)]

use crate::types::wchar_t;

/// Copy wide character array
///
/// Copies `n` wide characters from `src` to `dest`.
/// Returns the number of wide characters copied.
pub fn wmemcpy(dest: &mut [wchar_t], src: &[wchar_t]) -> usize {
    let n = dest.len().min(src.len());
    dest[..n].copy_from_slice(&src[..n]);
    n
}

/// Copy wide character array and return end index (wmempcpy).
///
/// Equivalent to wmemcpy but returns the index one past the last element copied.
pub fn wmempcpy(dest: &mut [wchar_t], src: &[wchar_t]) -> usize {
    let n = dest.len().min(src.len());
    if n > 0 {
        dest[..n].copy_from_slice(&src[..n]);
    }
    n
}

/// Copy wide character array (overlapping safe)
///
/// In safe Rust, this behaves identically to wmemcpy.
pub fn wmemmove(dest: &mut [wchar_t], src: &[wchar_t]) -> usize {
    let n = dest.len().min(src.len());
    if n == 0 {
        return 0;
    }
    // SAFETY: dest/src are valid for n elements, and ptr::copy supports overlap.
    unsafe { core::ptr::copy(src.as_ptr(), dest.as_mut_ptr(), n) };
    n
}

/// Fill wide character array with a constant
///
/// Sets all wide characters in `dest` to `c`. Returns the count.
pub fn wmemset(dest: &mut [wchar_t], c: wchar_t) -> usize {
    dest.fill(c);
    dest.len()
}

/// Compare wide character arrays
///
/// Compares slices element by element.
///
/// Returns:
/// - < 0 if s1 < s2
/// - 0 if s1 == s2
/// - > 0 if s1 > s2
pub fn wmemcmp(s1: &[wchar_t], s2: &[wchar_t]) -> i32 {
    let n = s1.len().min(s2.len());

    for i in 0..n {
        if s1[i] != s2[i] {
            return if s1[i] < s2[i] { -1 } else { 1 };
        }
    }

    match s1.len().cmp(&s2.len()) {
        core::cmp::Ordering::Less => -1,
        core::cmp::Ordering::Equal => 0,
        core::cmp::Ordering::Greater => 1,
    }
}

/// Scan wide character array for a wide character
///
/// Returns the index of the first occurrence of `c`, or `None` if not found.
pub fn wmemchr(s: &[wchar_t], c: wchar_t) -> Option<usize> {
    s.iter().position(|&ch| ch == c)
}

/// Scan wide character array backward for a wide character
pub fn wmemrchr(s: &[wchar_t], c: wchar_t) -> Option<usize> {
    s.iter().rposition(|&ch| ch == c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wmemmove_overlap_forward() {
        let mut buf = [1 as wchar_t, 2, 3, 4, 5];
        let (src_slice, dest_slice) = unsafe {
            // SAFETY: buf is valid for 5 elements; we create overlapping slices of length 4.
            let src = buf.as_ptr();
            let dest = buf.as_mut_ptr().add(1);
            (
                core::slice::from_raw_parts(src, 4),
                core::slice::from_raw_parts_mut(dest, 4),
            )
        };
        wmemmove(dest_slice, src_slice);
        assert_eq!(buf, [1, 1, 2, 3, 4]);
    }

    #[test]
    fn test_wmemmove_overlap_backward() {
        let mut buf = [1 as wchar_t, 2, 3, 4, 5];
        let (src_slice, dest_slice) = unsafe {
            // SAFETY: buf is valid for 5 elements; we create overlapping slices of length 4.
            let src = buf.as_ptr().add(1);
            let dest = buf.as_mut_ptr();
            (
                core::slice::from_raw_parts(src, 4),
                core::slice::from_raw_parts_mut(dest, 4),
            )
        };
        wmemmove(dest_slice, src_slice);
        assert_eq!(buf, [2, 3, 4, 5, 5]);
    }

    #[test]
    fn test_wmempcpy_basic() {
        let src = [1 as wchar_t, 2, 3];
        let mut dest = [0 as wchar_t; 4];
        let end = wmempcpy(&mut dest[..3], &src);
        assert_eq!(end, 3);
        assert_eq!(dest[..3], src);
    }
}
