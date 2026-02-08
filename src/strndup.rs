//! `strndup` implementation.

use crate::str::strnlen;

/// Duplicate at most `n` bytes of a C-style string into an owned buffer.
///
/// The returned vector is always nul-terminated.
pub fn strndup(src: &[u8], n: usize) -> Vec<u8> {
    let copy_len = strnlen(src, n).min(src.len()).min(n);
    let mut out = Vec::with_capacity(copy_len + 1);
    out.extend_from_slice(&src[..copy_len]);
    out.push(0);
    out
}

#[cfg(test)]
mod tests {
    use super::strndup;

    #[test]
    fn test_strndup_stops_at_nul() {
        assert_eq!(strndup(b"ab\0cd", 5), b"ab\0");
    }

    #[test]
    fn test_strndup_respects_limit() {
        assert_eq!(strndup(b"abcdef\0", 3), b"abc\0");
    }
}
