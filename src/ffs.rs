//! `ffs` implementation.

/// Find first (least significant) set bit in `i`.
///
/// Returns 1-based position of the bit, or 0 when `i == 0`.
pub fn ffs(i: i32) -> i32 {
    if i == 0 {
        0
    } else {
        (i.trailing_zeros() as i32) + 1
    }
}

#[cfg(test)]
mod tests {
    use super::ffs;

    #[test]
    fn test_ffs_basic() {
        assert_eq!(ffs(0), 0);
        assert_eq!(ffs(1), 1);
        assert_eq!(ffs(2), 2);
        assert_eq!(ffs(12), 3);
    }

    #[test]
    fn test_ffs_negative() {
        assert_eq!(ffs(-1), 1);
        assert_eq!(ffs(i32::MIN), 32);
    }
}
