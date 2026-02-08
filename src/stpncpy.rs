//! `stpncpy` implementation.

use crate::str::strlen;

/// Copy up to `n` bytes and return the index of the first nul written.
///
/// This follows C `stpncpy` behavior in slice form:
/// - copies at most `n` bytes from `src`
/// - pads destination with nul bytes when source is shorter than `n`
/// - returns `n` when no nul was written
pub fn stpncpy(dest: &mut [u8], src: &[u8], n: usize) -> usize {
    let limit = dest.len().min(n);
    let src_len = strlen(src).min(src.len());
    let copy_len = src_len.min(limit);

    if copy_len > 0 {
        dest[..copy_len].copy_from_slice(&src[..copy_len]);
    }

    if copy_len < limit {
        dest[copy_len..limit].fill(0);
        copy_len
    } else {
        limit
    }
}

#[cfg(test)]
mod tests {
    use super::stpncpy;

    #[test]
    fn test_stpncpy_short_source() {
        let mut dest = [0xAAu8; 8];
        let end = stpncpy(&mut dest, b"hi\0", 6);
        assert_eq!(end, 2);
        assert_eq!(&dest[..6], b"hi\0\0\0\0");
    }

    #[test]
    fn test_stpncpy_no_padding() {
        let mut dest = [0u8; 4];
        let end = stpncpy(&mut dest, b"abcdef\0", 4);
        assert_eq!(end, 4);
        assert_eq!(&dest, b"abcd");
    }
}
