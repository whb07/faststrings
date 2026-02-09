//! POSIX-style `strerror_r` implementation.

use crate::strerror::{lookup_error_message, UNKNOWN_ERROR_MESSAGE};

/// POSIX `EINVAL`.
pub const EINVAL: i32 = 22;
/// POSIX `ERANGE`.
pub const ERANGE: i32 = 34;

/// Writes a nul-terminated message for `errnum` into `buf`.
///
/// Returns:
/// - `0` on success
/// - `EINVAL` when `errnum` is unknown
/// - `ERANGE` when `buf` is too small (message is truncated but still nul-terminated)
pub fn strerror_r(errnum: i32, buf: &mut [u8]) -> i32 {
    if buf.is_empty() {
        return ERANGE;
    }

    let (msg, status) = match lookup_error_message(errnum) {
        Some(msg) => (msg, 0),
        None => (UNKNOWN_ERROR_MESSAGE, EINVAL),
    };

    if msg.len() <= buf.len() {
        buf[..msg.len()].copy_from_slice(msg);
        return status;
    }

    let copy_len = buf.len() - 1;
    if copy_len > 0 {
        buf[..copy_len].copy_from_slice(&msg[..copy_len]);
    }
    buf[copy_len] = 0;
    ERANGE
}

#[cfg(test)]
mod tests {
    use super::{strerror_r, EINVAL, ERANGE};

    #[test]
    fn test_strerror_r_success() {
        let mut buf = [0u8; 64];
        let rc = strerror_r(22, &mut buf);
        assert_eq!(rc, 0);
        assert_eq!(&buf[..17], b"Invalid argument\0");
    }

    #[test]
    fn test_strerror_r_unknown_sets_einval() {
        let mut buf = [0u8; 32];
        let rc = strerror_r(99_999, &mut buf);
        assert_eq!(rc, EINVAL);
        assert_eq!(&buf[..14], b"Unknown error\0");
    }

    #[test]
    fn test_strerror_r_truncates_with_erange() {
        let mut buf = [0xAAu8; 8];
        let rc = strerror_r(2, &mut buf);
        assert_eq!(rc, ERANGE);
        assert_eq!(&buf, b"No such\0");
    }

    #[test]
    fn test_strerror_r_empty_buffer() {
        let mut buf = [];
        let rc = strerror_r(2, &mut buf);
        assert_eq!(rc, ERANGE);
    }
}
