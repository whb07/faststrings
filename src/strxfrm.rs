//! `strxfrm` implementation.

use crate::str::strlen;

/// Transform a string for collation and return required transformed length.
///
/// In the C/POSIX locale this is equivalent to byte-preserving copy semantics.
/// The returned value is the length of transformed `src` excluding trailing nul.
pub fn strxfrm(dest: &mut [u8], src: &[u8]) -> usize {
    let src_len = strlen(src).min(src.len());

    if dest.is_empty() {
        return src_len;
    }

    let copy_len = src_len.min(dest.len().saturating_sub(1));
    if copy_len > 0 {
        dest[..copy_len].copy_from_slice(&src[..copy_len]);
    }
    dest[copy_len] = 0;

    src_len
}

#[cfg(test)]
mod tests {
    use super::strxfrm;

    #[test]
    fn test_strxfrm_copy_and_len() {
        let mut dest = [0u8; 8];
        let n = strxfrm(&mut dest, b"abc\0");
        assert_eq!(n, 3);
        assert_eq!(&dest[..4], b"abc\0");
    }

    #[test]
    fn test_strxfrm_truncation_reports_full_len() {
        let mut dest = [0u8; 3];
        let n = strxfrm(&mut dest, b"hello\0");
        assert_eq!(n, 5);
        assert_eq!(&dest, b"he\0");
    }
}
