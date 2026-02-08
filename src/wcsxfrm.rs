//! `wcsxfrm` implementation.

use crate::types::wchar_t;
use crate::wide::wcslen;

/// Transform a wide string for collation and return required transformed length.
///
/// In the C/POSIX locale this is equivalent to wide-character preserving copy.
/// The returned value is the transformed length excluding trailing nul.
pub fn wcsxfrm(dest: &mut [wchar_t], src: &[wchar_t]) -> usize {
    let src_len = wcslen(src).min(src.len());

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
    use super::wcsxfrm;
    use crate::types::wchar_t;

    #[test]
    fn test_wcsxfrm_copy_and_len() {
        let mut dest = [0 as wchar_t; 8];
        let src = [b'a' as wchar_t, b'b' as wchar_t, 0];
        let n = wcsxfrm(&mut dest, &src);
        assert_eq!(n, 2);
        assert_eq!(&dest[..3], &[b'a' as wchar_t, b'b' as wchar_t, 0]);
    }

    #[test]
    fn test_wcsxfrm_truncation() {
        let mut dest = [0 as wchar_t; 2];
        let src = [b'h' as wchar_t, b'i' as wchar_t, b'!' as wchar_t, 0];
        let n = wcsxfrm(&mut dest, &src);
        assert_eq!(n, 3);
        assert_eq!(&dest, &[b'h' as wchar_t, 0]);
    }
}
