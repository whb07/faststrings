//! String manipulation functions
//!
//! Safe Rust implementations of C string functions. These operate on byte slices
//! and treat 0 (null byte) as the string terminator.

/// Calculate the length of a null-terminated string
///
/// Returns the number of bytes before the first null byte (0).
/// If no null byte is found, returns the length of the slice.
///
/// # Examples
/// ```
/// use faststrings::str::strlen;
/// assert_eq!(strlen(b"hello\0world"), 5);
/// assert_eq!(strlen(b"\0"), 0);
/// assert_eq!(strlen(b"hello"), 5); // no null terminator
/// ```
pub fn strlen(s: &[u8]) -> usize {
    crate::mem::memchr(s, 0).unwrap_or(s.len())
}

/// Calculate bounded length of a null-terminated string
///
/// Returns the number of bytes before the first null byte, but at most `maxlen`.
///
/// # Examples
/// ```
/// use faststrings::str::strnlen;
/// assert_eq!(strnlen(b"hello\0world", 10), 5);
/// assert_eq!(strnlen(b"hello", 3), 3);
/// ```
pub fn strnlen(s: &[u8], maxlen: usize) -> usize {
    let limit = s.len().min(maxlen);
    crate::mem::memchr(&s[..limit], 0).unwrap_or(limit)
}

/// Version-aware string comparison (musl-compatible).
///
/// Treats digit sequences as numbers for ordering, following musl's
/// strverscmp semantics.
pub fn strverscmp(s1: &[u8], s2: &[u8]) -> i32 {
    fn is_digit(c: u8) -> bool {
        c.wrapping_sub(b'0') < 10
    }

    fn byte_at(s: &[u8], idx: usize) -> u8 {
        s.get(idx).copied().unwrap_or(0)
    }

    let mut i = 0usize;
    let mut dp = 0usize;
    let mut zeros = true;

    loop {
        let c1 = byte_at(s1, i);
        let c2 = byte_at(s2, i);
        if c1 != c2 {
            break;
        }
        if c1 == 0 {
            return 0;
        }
        if !is_digit(c1) {
            dp = i + 1;
            zeros = true;
        } else if c1 != b'0' {
            zeros = false;
        }
        i += 1;
    }

    let dp_c1 = byte_at(s1, dp);
    let dp_c2 = byte_at(s2, dp);

    if (dp_c1.wrapping_sub(b'1') < 9) && (dp_c2.wrapping_sub(b'1') < 9) {
        let mut j = i;
        loop {
            let lj = byte_at(s1, j);
            let rj = byte_at(s2, j);
            if !is_digit(lj) {
                break;
            }
            if !is_digit(rj) {
                return 1;
            }
            j += 1;
        }
        if is_digit(byte_at(s2, j)) {
            return -1;
        }
    } else if zeros && dp < i {
        let c1 = byte_at(s1, i);
        let c2 = byte_at(s2, i);
        if is_digit(c1) || is_digit(c2) {
            let l = c1.wrapping_sub(b'0') as i32;
            let r = c2.wrapping_sub(b'0') as i32;
            return l - r;
        }
    }

    byte_at(s1, i) as i32 - byte_at(s2, i) as i32
}

/// Copy a null-terminated string
///
/// Copies bytes from `src` to `dest` up to and including the first null byte.
/// Returns the number of bytes copied (including the null).
/// If `src` has no null byte, copies until end of `src` or `dest` is full.
///
/// # Examples
/// ```
/// use faststrings::str::strcpy;
/// let mut dest = [0u8; 10];
/// let n = strcpy(&mut dest, b"hello\0");
/// assert_eq!(n, 6);
/// assert_eq!(&dest[..6], b"hello\0");
/// ```
pub fn strcpy(dest: &mut [u8], src: &[u8]) -> usize {
    let src_len = strlen(src);
    let copy_len = (src_len + 1).min(dest.len()).min(src.len());

    // Copy the string content
    let content_len = copy_len.min(src_len);
    dest[..content_len].copy_from_slice(&src[..content_len]);

    // Add null terminator if there's room
    if copy_len > src_len && copy_len <= dest.len() {
        dest[src_len] = 0;
        return src_len + 1;
    }

    content_len
}

/// Copy a string with length limit
///
/// Copies at most `n` bytes from `src` to `dest`. If `src` is shorter than `n`,
/// the remainder of `dest` is padded with null bytes.
///
/// # Examples
/// ```
/// use faststrings::str::strncpy;
/// let mut dest = [0xFFu8; 10];
/// strncpy(&mut dest, b"hi\0", 5);
/// assert_eq!(&dest[..5], b"hi\0\0\0");
/// ```
pub fn strncpy(dest: &mut [u8], src: &[u8], n: usize) -> usize {
    let limit = dest.len().min(n);
    let src_len = strlen(src);
    let copy_len = src_len.min(limit).min(src.len());

    // Copy source bytes
    dest[..copy_len].copy_from_slice(&src[..copy_len]);

    // Pad with nulls
    if copy_len < limit {
        dest[copy_len..limit].fill(0);
    }

    limit
}

/// Copy a string, returning bytes written
///
/// Like strcpy, but returns the position where the null terminator was written
/// (or would have been written).
pub fn stpcpy(dest: &mut [u8], src: &[u8]) -> usize {
    let src_len = strlen(src);
    let copy_len = src_len.min(dest.len()).min(src.len());

    dest[..copy_len].copy_from_slice(&src[..copy_len]);

    // Add null terminator if there's room
    if copy_len < dest.len() && src_len < src.len() {
        dest[copy_len] = 0;
    }

    copy_len
}

/// Concatenate two strings
///
/// Appends `src` to the end of the null-terminated string in `dest`.
/// Returns the new total length (excluding null terminator).
///
/// # Examples
/// ```
/// use faststrings::str::strcat;
/// let mut dest = [0u8; 20];
/// dest[..6].copy_from_slice(b"hello\0");
/// let len = strcat(&mut dest, b" world\0");
/// assert_eq!(len, 11);
/// assert_eq!(&dest[..12], b"hello world\0");
/// ```
pub fn strcat(dest: &mut [u8], src: &[u8]) -> usize {
    let dest_len = strlen(dest);
    let src_len = strlen(src);

    if dest_len >= dest.len() {
        return dest_len;
    }

    let remaining = dest.len() - dest_len;
    let copy_len = src_len.min(remaining - 1).min(src.len());

    dest[dest_len..dest_len + copy_len].copy_from_slice(&src[..copy_len]);

    // Always null-terminate if possible
    if dest_len + copy_len < dest.len() {
        dest[dest_len + copy_len] = 0;
    }

    dest_len + copy_len
}

/// Concatenate strings with length limit
///
/// Appends at most `n` bytes from `src` to `dest`, always null-terminating.
pub fn strncat(dest: &mut [u8], src: &[u8], n: usize) -> usize {
    let dest_len = strlen(dest);
    let src_len = strnlen(src, n);

    if dest_len >= dest.len() {
        return dest_len;
    }

    let remaining = dest.len() - dest_len;
    let copy_len = src_len.min(remaining - 1).min(src.len());

    dest[dest_len..dest_len + copy_len].copy_from_slice(&src[..copy_len]);

    if dest_len + copy_len < dest.len() {
        dest[dest_len + copy_len] = 0;
    }

    dest_len + copy_len
}

/// Compare two null-terminated strings
///
/// Compares strings lexicographically up to the first null byte.
///
/// Returns:
/// - < 0 if s1 < s2
/// - 0 if s1 == s2
/// - > 0 if s1 > s2
///
/// # Examples
/// ```
/// use faststrings::str::strcmp;
/// assert!(strcmp(b"abc\0", b"abd\0") < 0);
/// assert!(strcmp(b"abc\0", b"abc\0") == 0);
/// assert!(strcmp(b"abd\0", b"abc\0") > 0);
/// ```
pub fn strcmp(s1: &[u8], s2: &[u8]) -> i32 {
    let len1 = strlen(s1);
    let len2 = strlen(s2);
    let min_len = len1.min(len2);

    for i in 0..min_len {
        if s1[i] != s2[i] {
            return (s1[i] as i32) - (s2[i] as i32);
        }
    }

    // If all bytes equal, shorter string is "less"
    match len1.cmp(&len2) {
        core::cmp::Ordering::Less => -1,
        core::cmp::Ordering::Equal => 0,
        core::cmp::Ordering::Greater => 1,
    }
}

/// Compare strings with length limit
///
/// Like strcmp, but compares at most `n` bytes.
pub fn strncmp(s1: &[u8], s2: &[u8], n: usize) -> i32 {
    if n == 0 {
        return 0;
    }

    let len1 = strnlen(s1, n);
    let len2 = strnlen(s2, n);
    let min_len = len1.min(len2);

    for i in 0..min_len {
        if s1[i] != s2[i] {
            return (s1[i] as i32) - (s2[i] as i32);
        }
    }

    if len1 < n && len2 < n {
        match len1.cmp(&len2) {
            core::cmp::Ordering::Less => -1,
            core::cmp::Ordering::Equal => 0,
            core::cmp::Ordering::Greater => 1,
        }
    } else {
        0
    }
}

/// Compare strings using locale collation
///
/// In the C/POSIX locale, this is equivalent to strcmp.
pub fn strcoll(s1: &[u8], s2: &[u8]) -> i32 {
    strcmp(s1, s2)
}

/// Compare strings ignoring case
///
/// Like strcmp but treats uppercase and lowercase ASCII letters as equal.
///
/// # Examples
/// ```
/// use faststrings::str::strcasecmp;
/// assert_eq!(strcasecmp(b"Hello\0", b"hello\0"), 0);
/// assert!(strcasecmp(b"ABC\0", b"abd\0") < 0);
/// ```
pub fn strcasecmp(s1: &[u8], s2: &[u8]) -> i32 {
    let len1 = strlen(s1);
    let len2 = strlen(s2);
    let min_len = len1.min(len2);

    for i in 0..min_len {
        let c1 = to_lower_ascii(s1[i]);
        let c2 = to_lower_ascii(s2[i]);

        if c1 != c2 {
            return (c1 as i32) - (c2 as i32);
        }
    }

    match len1.cmp(&len2) {
        core::cmp::Ordering::Less => -1,
        core::cmp::Ordering::Equal => 0,
        core::cmp::Ordering::Greater => 1,
    }
}

/// Compare strings ignoring case with length limit
pub fn strncasecmp(s1: &[u8], s2: &[u8], n: usize) -> i32 {
    if n == 0 {
        return 0;
    }

    let len1 = strnlen(s1, n);
    let len2 = strnlen(s2, n);
    let min_len = len1.min(len2);

    for i in 0..min_len {
        let c1 = to_lower_ascii(s1[i]);
        let c2 = to_lower_ascii(s2[i]);

        if c1 != c2 {
            return (c1 as i32) - (c2 as i32);
        }
    }

    if len1 < n && len2 < n {
        match len1.cmp(&len2) {
            core::cmp::Ordering::Less => -1,
            core::cmp::Ordering::Equal => 0,
            core::cmp::Ordering::Greater => 1,
        }
    } else {
        0
    }
}

/// Copy string with size limit (safe version)
///
/// Copies up to `size - 1` bytes from `src` to `dest`, always null-terminating.
/// Returns the length of `src` (for truncation detection).
///
/// # Examples
/// ```
/// use faststrings::str::strlcpy;
/// let mut dest = [0u8; 5];
/// let len = strlcpy(&mut dest, b"hello world\0");
/// assert_eq!(len, 11); // src length
/// assert_eq!(&dest, b"hell\0"); // truncated + null
/// ```
pub fn strlcpy(dest: &mut [u8], src: &[u8]) -> usize {
    let src_len = strlen(src);

    if dest.is_empty() {
        return src_len;
    }

    let copy_len = src_len.min(dest.len() - 1).min(src.len());
    dest[..copy_len].copy_from_slice(&src[..copy_len]);
    dest[copy_len] = 0;

    src_len
}

/// Concatenate strings with size limit (safe version)
///
/// Appends `src` to `dest`, ensuring null-termination and not exceeding `size`.
/// Returns the total length that would have been created without truncation.
pub fn strlcat(dest: &mut [u8], src: &[u8]) -> usize {
    let size = dest.len();
    let dest_len = strnlen(dest, size);
    let src_len = strlen(src);

    if dest_len >= size {
        return size + src_len;
    }

    let remaining = size - dest_len;
    let copy_len = src_len.min(remaining - 1).min(src.len());

    dest[dest_len..dest_len + copy_len].copy_from_slice(&src[..copy_len]);
    dest[dest_len + copy_len] = 0;

    dest_len + src_len
}

// Helper function for case-insensitive comparison
fn to_lower_ascii(c: u8) -> u8 {
    c.to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::strverscmp;

    #[test]
    fn test_strverscmp_numeric_ordering() {
        assert!(strverscmp(b"a1\0", b"a2\0") < 0);
        assert!(strverscmp(b"a2\0", b"a10\0") < 0);
        assert!(strverscmp(b"a10\0", b"a2\0") > 0);
    }

    #[test]
    fn test_strverscmp_leading_zeros() {
        assert!(strverscmp(b"a01\0", b"a1\0") < 0);
        assert!(strverscmp(b"v000\0", b"v0\0") < 0);
    }
}
