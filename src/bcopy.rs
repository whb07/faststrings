//! `bcopy` implementation.

/// Copy bytes from `src` to `dest`, allowing overlap.
///
/// Returns the number of bytes copied.
pub fn bcopy(src: &[u8], dest: &mut [u8]) -> usize {
    let n = src.len().min(dest.len());
    if n == 0 {
        return 0;
    }

    // SAFETY: Pointers are derived from valid slices with at least `n` bytes.
    // Optimized memmove handles overlap semantics required by bcopy.
    unsafe {
        crate::memmove::optimized_memmove_unified(dest.as_mut_ptr(), src.as_ptr(), n);
    }
    n
}

#[cfg(test)]
mod tests {
    use super::bcopy;

    #[test]
    fn test_bcopy_basic() {
        let src = *b"hello";
        let mut dst = [0u8; 5];
        let n = bcopy(&src, &mut dst);
        assert_eq!(n, 5);
        assert_eq!(&dst, b"hello");
    }

    #[test]
    fn test_bcopy_overlap() {
        let mut buf = *b"abcdef";
        // src = "abcd", dest starts at index 2
        let (src, dst) = unsafe {
            let src = core::slice::from_raw_parts(buf.as_ptr(), 4);
            let dst = core::slice::from_raw_parts_mut(buf.as_mut_ptr().add(2), 4);
            (src, dst)
        };
        bcopy(src, dst);
        assert_eq!(&buf, b"ababcd");
    }
}
