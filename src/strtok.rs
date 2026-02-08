//! `strtok` implementation.

use crate::strtok_r::strtok_r;

/// Tokenize a nul-terminated byte string using caller-managed state.
///
/// This safe API mirrors `strtok` behavior without hidden global state.
/// Callers should initialize `*state = 0` before the first call.
pub fn strtok<'a>(s: &'a [u8], delim: &[u8], state: &mut usize) -> Option<&'a [u8]> {
    strtok_r(s, delim, state)
}

#[cfg(test)]
mod tests {
    use super::strtok;

    #[test]
    fn test_strtok_sequence() {
        let s = b"one:two:three\0";
        let mut state = 0usize;
        assert_eq!(strtok(s, b":\0", &mut state), Some(&b"one"[..]));
        assert_eq!(strtok(s, b":\0", &mut state), Some(&b"two"[..]));
        assert_eq!(strtok(s, b":\0", &mut state), Some(&b"three"[..]));
        assert_eq!(strtok(s, b":\0", &mut state), None);
    }
}
