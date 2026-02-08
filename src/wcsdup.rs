//! `wcsdup` implementation.

use crate::types::wchar_t;
use crate::wide::wcslen;

/// Duplicate a wide C-style string into an owned buffer.
///
/// The returned vector is always nul-terminated.
pub fn wcsdup(src: &[wchar_t]) -> Vec<wchar_t> {
    let src_len = wcslen(src).min(src.len());
    let mut out = Vec::with_capacity(src_len + 1);
    out.extend_from_slice(&src[..src_len]);
    out.push(0);
    out
}

#[cfg(test)]
mod tests {
    use super::wcsdup;
    use crate::types::wchar_t;

    #[test]
    fn test_wcsdup_basic() {
        let src = [b'a' as wchar_t, b'b' as wchar_t, 0];
        assert_eq!(wcsdup(&src), src);
    }

    #[test]
    fn test_wcsdup_adds_terminator_when_missing() {
        let src = [b'a' as wchar_t, b'b' as wchar_t];
        assert_eq!(wcsdup(&src), vec![b'a' as wchar_t, b'b' as wchar_t, 0]);
    }
}
