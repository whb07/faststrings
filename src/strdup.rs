//! `strdup` implementation.

use crate::str::strlen;

/// Duplicate a C-style string into an owned buffer.
///
/// The returned vector is always nul-terminated.
pub fn strdup(src: &[u8]) -> Vec<u8> {
    let src_len = strlen(src).min(src.len());
    let mut out = Vec::with_capacity(src_len + 1);
    out.extend_from_slice(&src[..src_len]);
    out.push(0);
    out
}

#[cfg(test)]
mod tests {
    use super::strdup;

    #[test]
    fn test_strdup_basic() {
        assert_eq!(strdup(b"abc\0"), b"abc\0");
    }

    #[test]
    fn test_strdup_adds_terminator_when_missing() {
        assert_eq!(strdup(b"abc"), b"abc\0");
    }
}
