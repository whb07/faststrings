//! String searching functions
//!
//! Safe Rust implementations of string search functions.

use crate::str::strlen;

/// Locate character in null-terminated string
///
/// Returns the index of the first occurrence of `c` in `s` (up to the null terminator),
/// or `None` if not found.
///
/// # Examples
/// ```
/// use faststrings::search::strchr;
/// assert_eq!(strchr(b"hello\0world", b'l'), Some(2));
/// assert_eq!(strchr(b"hello\0world", b'w'), None); // 'w' is after null
/// assert_eq!(strchr(b"hello\0", b'\0'), Some(5)); // can find null
/// ```
pub fn strchr(s: &[u8], c: u8) -> Option<usize> {
    let len = if c == 0 {
        // If searching for null, search the whole slice
        s.len()
    } else {
        // Otherwise, search up to and including the null terminator
        strlen(s) + 1
    };

    let search_len = len.min(s.len());
    s[..search_len].iter().position(|&b| b == c)
}

/// Locate character in string (returns length if not found)
///
/// Like strchr, but returns the string length (position of null terminator)
/// if `c` is not found, rather than None.
pub fn strchrnul(s: &[u8], c: u8) -> usize {
    let len = strlen(s);
    let search_len = len.min(s.len());

    s[..search_len].iter().position(|&b| b == c).unwrap_or(len)
}

/// Locate character in string (from end)
///
/// Returns the index of the last occurrence of `c` in `s` (up to the null terminator),
/// or `None` if not found.
///
/// # Examples
/// ```
/// use faststrings::search::strrchr;
/// assert_eq!(strrchr(b"hello\0", b'l'), Some(3));
/// assert_eq!(strrchr(b"hello\0", b'x'), None);
/// ```
pub fn strrchr(s: &[u8], c: u8) -> Option<usize> {
    let len = strlen(s);
    let search_len = if c == 0 { len + 1 } else { len }.min(s.len());

    s[..search_len].iter().rposition(|&b| b == c)
}

/// Locate substring
///
/// Finds the first occurrence of the null-terminated string `needle` in
/// the null-terminated string `haystack`.
///
/// # Examples
/// ```
/// use faststrings::search::strstr;
/// assert_eq!(strstr(b"hello world\0", b"wor\0"), Some(6));
/// assert_eq!(strstr(b"hello world\0", b"xyz\0"), None);
/// assert_eq!(strstr(b"hello\0", b"\0"), Some(0)); // empty needle
/// ```
pub fn strstr(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    let h_len = strlen(haystack);
    let n_len = strlen(needle);

    if n_len == 0 {
        return Some(0);
    }

    if n_len > h_len {
        return None;
    }

    let haystack = &haystack[..h_len.min(haystack.len())];
    let needle = &needle[..n_len.min(needle.len())];

    let end = h_len - n_len + 1;

    (0..end).find(|&i| &haystack[i..i + n_len] == needle)
}

/// Locate substring (case-insensitive)
///
/// Like strstr, but ignores case when comparing.
pub fn strcasestr(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    let h_len = strlen(haystack);
    let n_len = strlen(needle);

    if n_len == 0 {
        return Some(0);
    }

    if n_len > h_len {
        return None;
    }

    let haystack = &haystack[..h_len.min(haystack.len())];
    let needle = &needle[..n_len.min(needle.len())];

    let end = h_len - n_len + 1;

    'outer: for i in 0..end {
        for j in 0..n_len {
            let hc = to_lower_ascii(haystack[i + j]);
            let nc = to_lower_ascii(needle[j]);
            if hc != nc {
                continue 'outer;
            }
        }
        return Some(i);
    }

    None
}

/// Get length of prefix consisting of accepted characters
///
/// Returns the length of the initial segment of `s` which consists
/// entirely of characters in `accept`.
///
/// # Examples
/// ```
/// use faststrings::search::strspn;
/// assert_eq!(strspn(b"hello\0", b"ehlo\0"), 5);
/// assert_eq!(strspn(b"hello\0", b"xyz\0"), 0);
/// ```
pub fn strspn(s: &[u8], accept: &[u8]) -> usize {
    let s_len = strlen(s);
    let accept_len = strlen(accept);
    let s = &s[..s_len.min(s.len())];
    let accept = &accept[..accept_len.min(accept.len())];

    for (i, &c) in s.iter().enumerate() {
        if !accept.contains(&c) {
            return i;
        }
    }

    s.len()
}

/// Get length of prefix not containing rejected characters
///
/// Returns the length of the initial segment of `s` which does not
/// contain any of the characters in `reject`.
///
/// # Examples
/// ```
/// use faststrings::search::strcspn;
/// assert_eq!(strcspn(b"hello\0", b"lo\0"), 2); // "he" before 'l'
/// assert_eq!(strcspn(b"hello\0", b"xyz\0"), 5);
/// ```
pub fn strcspn(s: &[u8], reject: &[u8]) -> usize {
    let s_len = strlen(s);
    let reject_len = strlen(reject);
    let s = &s[..s_len.min(s.len())];
    let reject = &reject[..reject_len.min(reject.len())];

    for (i, &c) in s.iter().enumerate() {
        if reject.contains(&c) {
            return i;
        }
    }

    s.len()
}

/// Search string for any of a set of characters
///
/// Locates the first occurrence in `s` of any character in `accept`.
/// Returns the index, or `None` if not found.
///
/// # Examples
/// ```
/// use faststrings::search::strpbrk;
/// assert_eq!(strpbrk(b"hello\0", b"lo\0"), Some(2)); // 'l' at index 2
/// assert_eq!(strpbrk(b"hello\0", b"xyz\0"), None);
/// ```
pub fn strpbrk(s: &[u8], accept: &[u8]) -> Option<usize> {
    let s_len = strlen(s);
    let accept_len = strlen(accept);
    let s = &s[..s_len.min(s.len())];
    let accept = &accept[..accept_len.min(accept.len())];

    for (i, &c) in s.iter().enumerate() {
        if accept.contains(&c) {
            return Some(i);
        }
    }

    None
}

/// Locate byte in string (BSD alias for strchr)
pub fn index(s: &[u8], c: u8) -> Option<usize> {
    strchr(s, c)
}

/// Locate byte in string from end (BSD alias for strrchr)
pub fn rindex(s: &[u8], c: u8) -> Option<usize> {
    strrchr(s, c)
}

// Helper for case-insensitive comparison
fn to_lower_ascii(c: u8) -> u8 {
    c.to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strchr_and_strchrnul() {
        assert_eq!(strchr(b"hello\0", b'l'), Some(2));
        assert_eq!(strchr(b"hello\0", b'w'), None);
        assert_eq!(strchr(b"hello\0", b'\0'), Some(5));
        assert_eq!(strchrnul(b"hello\0", b'l'), 2);
        assert_eq!(strchrnul(b"hello\0", b'x'), 5);
    }

    #[test]
    fn test_strrchr_and_aliases() {
        assert_eq!(strrchr(b"hello\0", b'l'), Some(3));
        assert_eq!(index(b"hello\0", b'h'), Some(0));
        assert_eq!(rindex(b"hello\0", b'h'), Some(0));
    }

    #[test]
    fn test_strstr_and_strcasestr() {
        assert_eq!(strstr(b"hello world\0", b"wor\0"), Some(6));
        assert_eq!(strstr(b"hello\0", b"\0"), Some(0));
        assert_eq!(strcasestr(b"Hello\0", b"hEl\0"), Some(0));
    }

    #[test]
    fn test_strspn_strcspn_strpbrk() {
        assert_eq!(strspn(b"hello\0", b"ehlo\0"), 5);
        assert_eq!(strcspn(b"hello\0", b"lo\0"), 2);
        assert_eq!(strpbrk(b"hello\0", b"xyz\0"), None);
        assert_eq!(strpbrk(b"hello\0", b"lo\0"), Some(2));
    }
}
