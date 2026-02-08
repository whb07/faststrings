//! Wide string manipulation functions
//!
//! Safe Rust implementations of wide string functions.
//! Wide characters are represented as wchar_t.

use crate::types::wchar_t;

/// Calculate the length of a null-terminated wide string
///
/// Returns the number of wide characters before the first null (0).
///
/// # Examples
/// ```
/// use faststrings::wide::wcslen;
/// use faststrings::types::wchar_t;
/// let s = [b'h' as wchar_t, b'i' as wchar_t, 0, b'!' as wchar_t];
/// assert_eq!(wcslen(&s), 2);
/// ```
pub fn wcslen(s: &[wchar_t]) -> usize {
    s.iter().position(|&c| c == 0).unwrap_or(s.len())
}

/// Calculate bounded length of a wide string
pub fn wcsnlen(s: &[wchar_t], maxlen: usize) -> usize {
    let limit = s.len().min(maxlen);
    s[..limit].iter().position(|&c| c == 0).unwrap_or(limit)
}

/// Copy a wide string
///
/// Copies wide characters from `src` to `dest` up to and including the null.
/// Returns the number of wide characters copied.
pub fn wcscpy(dest: &mut [wchar_t], src: &[wchar_t]) -> usize {
    let src_len = wcslen(src);
    let copy_len = (src_len + 1).min(dest.len()).min(src.len());

    let content_len = copy_len.min(src_len);
    dest[..content_len].copy_from_slice(&src[..content_len]);

    if copy_len > src_len && copy_len <= dest.len() {
        dest[src_len] = 0;
        return src_len + 1;
    }

    content_len
}

/// Copy a wide string with length limit
pub fn wcsncpy(dest: &mut [wchar_t], src: &[wchar_t], n: usize) -> usize {
    let limit = dest.len().min(n);
    let src_len = wcslen(src);
    let copy_len = src_len.min(limit).min(src.len());

    dest[..copy_len].copy_from_slice(&src[..copy_len]);

    if copy_len < limit {
        dest[copy_len..limit].fill(0);
    }

    limit
}

/// Copy a wide string and return index of null terminator (wcpcpy).
///
/// Caller must provide enough space for the full source string plus null.
pub fn wcpcpy(dest: &mut [wchar_t], src: &[wchar_t]) -> usize {
    let src_len = wcslen(src);
    let copy_len = src_len.min(dest.len().saturating_sub(1));

    if copy_len > 0 {
        dest[..copy_len].copy_from_slice(&src[..copy_len]);
    }
    if copy_len < dest.len() {
        dest[copy_len] = 0;
    }

    copy_len
}

/// Copy a wide string with length limit and return end index (wcpncpy).
///
/// Returns the index of the null terminator if written, or n if none.
pub fn wcpncpy(dest: &mut [wchar_t], src: &[wchar_t], n: usize) -> usize {
    let limit = dest.len().min(n);
    let src_len = wcslen(src);
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

/// Concatenate two wide strings
pub fn wcscat(dest: &mut [wchar_t], src: &[wchar_t]) -> usize {
    let dest_len = wcslen(dest);
    let src_len = wcslen(src);

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

/// Concatenate wide strings with length limit
pub fn wcsncat(dest: &mut [wchar_t], src: &[wchar_t], n: usize) -> usize {
    let dest_len = wcslen(dest);
    let src_len = wcsnlen(src, n);

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

/// Compare two wide strings
pub fn wcscmp(s1: &[wchar_t], s2: &[wchar_t]) -> i32 {
    let len1 = wcslen(s1);
    let len2 = wcslen(s2);
    let min_len = len1.min(len2);

    for i in 0..min_len {
        if s1[i] != s2[i] {
            return if s1[i] < s2[i] { -1 } else { 1 };
        }
    }

    match len1.cmp(&len2) {
        core::cmp::Ordering::Less => -1,
        core::cmp::Ordering::Equal => 0,
        core::cmp::Ordering::Greater => 1,
    }
}

/// Compare wide strings with length limit
pub fn wcsncmp(s1: &[wchar_t], s2: &[wchar_t], n: usize) -> i32 {
    if n == 0 {
        return 0;
    }

    let len1 = wcsnlen(s1, n);
    let len2 = wcsnlen(s2, n);
    let min_len = len1.min(len2);

    for i in 0..min_len {
        if s1[i] != s2[i] {
            return if s1[i] < s2[i] { -1 } else { 1 };
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

/// Compare wide strings using locale collation
pub fn wcscoll(s1: &[wchar_t], s2: &[wchar_t]) -> i32 {
    wcscmp(s1, s2)
}

/// Locate wide character in wide string
pub fn wcschr(s: &[wchar_t], c: wchar_t) -> Option<usize> {
    let len = if c == 0 { s.len() } else { wcslen(s) + 1 };
    let search_len = len.min(s.len());
    s[..search_len].iter().position(|&ch| ch == c)
}

/// Locate wide character from end
pub fn wcsrchr(s: &[wchar_t], c: wchar_t) -> Option<usize> {
    let len = wcslen(s);
    let search_len = if c == 0 { len + 1 } else { len }.min(s.len());
    s[..search_len].iter().rposition(|&ch| ch == c)
}

/// Locate wide substring
pub fn wcsstr(haystack: &[wchar_t], needle: &[wchar_t]) -> Option<usize> {
    let h_len = wcslen(haystack);
    let n_len = wcslen(needle);

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

/// Get length of wide prefix of accepted characters
pub fn wcsspn(s: &[wchar_t], accept: &[wchar_t]) -> usize {
    let s_len = wcslen(s);
    let accept_len = wcslen(accept);
    let s = &s[..s_len.min(s.len())];
    let accept = &accept[..accept_len.min(accept.len())];

    for (i, &c) in s.iter().enumerate() {
        if !accept.contains(&c) {
            return i;
        }
    }

    s.len()
}

/// Get length of wide prefix not containing rejected characters
pub fn wcscspn(s: &[wchar_t], reject: &[wchar_t]) -> usize {
    let s_len = wcslen(s);
    let reject_len = wcslen(reject);
    let s = &s[..s_len.min(s.len())];
    let reject = &reject[..reject_len.min(reject.len())];

    for (i, &c) in s.iter().enumerate() {
        if reject.contains(&c) {
            return i;
        }
    }

    s.len()
}

/// Search wide string for any character in accept set
pub fn wcspbrk(s: &[wchar_t], accept: &[wchar_t]) -> Option<usize> {
    let s_len = wcslen(s);
    let accept_len = wcslen(accept);
    let s = &s[..s_len.min(s.len())];
    let accept = &accept[..accept_len.min(accept.len())];

    for (i, &c) in s.iter().enumerate() {
        if accept.contains(&c) {
            return Some(i);
        }
    }

    None
}

/// Compare wide strings ignoring case (ASCII only)
pub fn wcscasecmp(s1: &[wchar_t], s2: &[wchar_t]) -> i32 {
    let len1 = wcslen(s1);
    let len2 = wcslen(s2);
    let min_len = len1.min(len2);

    for i in 0..min_len {
        let c1 = to_lower_wide(s1[i]);
        let c2 = to_lower_wide(s2[i]);

        if c1 != c2 {
            return if c1 < c2 { -1 } else { 1 };
        }
    }

    match len1.cmp(&len2) {
        core::cmp::Ordering::Less => -1,
        core::cmp::Ordering::Equal => 0,
        core::cmp::Ordering::Greater => 1,
    }
}

/// Compare wide strings ignoring case with length limit
pub fn wcsncasecmp(s1: &[wchar_t], s2: &[wchar_t], n: usize) -> i32 {
    if n == 0 {
        return 0;
    }

    let len1 = wcsnlen(s1, n);
    let len2 = wcsnlen(s2, n);
    let min_len = len1.min(len2);

    for i in 0..min_len {
        let c1 = to_lower_wide(s1[i]);
        let c2 = to_lower_wide(s2[i]);

        if c1 != c2 {
            return if c1 < c2 { -1 } else { 1 };
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

/// Locate wide character in wide string, or return end if not found.
pub fn wcschrnul(s: &[wchar_t], c: wchar_t) -> usize {
    let len = wcslen(s);
    if c == 0 {
        return len;
    }
    s[..len.min(s.len())]
        .iter()
        .position(|&ch| ch == c)
        .unwrap_or(len)
}

/// Copy wide string with size limit (safe version).
///
/// Copies up to `size - 1` wide chars from `src` to `dest`, always null-terminating.
/// Returns the length of `src` (for truncation detection).
pub fn wcslcpy(dest: &mut [wchar_t], src: &[wchar_t]) -> usize {
    let src_len = wcslen(src);

    if dest.is_empty() {
        return src_len;
    }

    let copy_len = src_len.min(dest.len() - 1).min(src.len());
    dest[..copy_len].copy_from_slice(&src[..copy_len]);
    dest[copy_len] = 0;

    src_len
}

/// Concatenate wide strings with size limit (safe version).
///
/// Appends `src` to `dest`, ensuring null-termination and not exceeding `size`.
/// Returns the total length that would have been created without truncation.
pub fn wcslcat(dest: &mut [wchar_t], src: &[wchar_t]) -> usize {
    let size = dest.len();
    let dest_len = wcsnlen(dest, size);
    let src_len = wcslen(src);

    if dest_len >= size {
        return size + src_len;
    }

    let remaining = size - dest_len;
    let copy_len = src_len.min(remaining - 1).min(src.len());

    dest[dest_len..dest_len + copy_len].copy_from_slice(&src[..copy_len]);
    dest[dest_len + copy_len] = 0;

    dest_len + src_len
}

// Simple ASCII case folding for wide characters
fn to_lower_wide(c: wchar_t) -> wchar_t {
    if c >= 'A' as wchar_t && c <= 'Z' as wchar_t {
        c + 32
    } else {
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wcslen_wcsnlen() {
        let s = [b'h' as wchar_t, b'i' as wchar_t, 0, b'!' as wchar_t];
        assert_eq!(wcslen(&s), 2);
        assert_eq!(wcsnlen(&s, 0), 0);
        assert_eq!(wcsnlen(&s, 1), 1);
        assert_eq!(wcsnlen(&s, 4), 2);
    }

    #[test]
    fn test_wcscpy_wcsncpy() {
        let src = [b'a' as wchar_t, b'b' as wchar_t, 0];
        let mut dest = [0 as wchar_t; 5];
        wcscpy(&mut dest, &src);
        assert_eq!(dest[0], b'a' as wchar_t);
        assert_eq!(dest[1], b'b' as wchar_t);
        assert_eq!(dest[2], 0);

        let mut dest2 = [b'x' as wchar_t; 5];
        wcsncpy(&mut dest2, &src, 4);
        assert_eq!(dest2[0], b'a' as wchar_t);
        assert_eq!(dest2[1], b'b' as wchar_t);
        assert_eq!(dest2[2], 0);
        assert_eq!(dest2[3], 0);
    }

    #[test]
    fn test_wcscat_wcsncat() {
        let mut dest = [b'h' as wchar_t, b'i' as wchar_t, 0, 0, 0];
        let src = [b'!' as wchar_t, 0];
        wcscat(&mut dest, &src);
        assert_eq!(dest[0], b'h' as wchar_t);
        assert_eq!(dest[1], b'i' as wchar_t);
        assert_eq!(dest[2], b'!' as wchar_t);
        assert_eq!(dest[3], 0);

        let mut dest2 = [b'a' as wchar_t, 0, 0, 0, 0];
        let src2 = [b'b' as wchar_t, b'c' as wchar_t, 0];
        wcsncat(&mut dest2, &src2, 1);
        assert_eq!(dest2[0], b'a' as wchar_t);
        assert_eq!(dest2[1], b'b' as wchar_t);
        assert_eq!(dest2[2], 0);
    }

    #[test]
    fn test_wcscmp_wcsncmp() {
        let a = [b'a' as wchar_t, 0];
        let b = [b'b' as wchar_t, 0];
        assert!(wcscmp(&a, &b) < 0);
        assert!(wcscmp(&b, &a) > 0);
        assert_eq!(wcsncmp(&a, &b, 0), 0);
        assert!(wcsncmp(&a, &b, 1) < 0);
    }

    #[test]
    fn test_wcschr_wcsrchr_wcsstr() {
        let s = [b'a' as wchar_t, b'b' as wchar_t, b'a' as wchar_t, 0];
        assert_eq!(wcschr(&s, b'a' as wchar_t), Some(0));
        assert_eq!(wcsrchr(&s, b'a' as wchar_t), Some(2));

        let hay = [b'a' as wchar_t, b'b' as wchar_t, b'c' as wchar_t, 0];
        let needle = [b'b' as wchar_t, b'c' as wchar_t, 0];
        assert_eq!(wcsstr(&hay, &needle), Some(1));
    }

    #[test]
    fn test_wcsspn_wcscspn_wcspbrk() {
        let s = [b'a' as wchar_t, b'b' as wchar_t, b'c' as wchar_t, 0];
        let accept = [b'a' as wchar_t, b'b' as wchar_t, 0];
        let reject = [b'c' as wchar_t, 0];
        assert_eq!(wcsspn(&s, &accept), 2);
        assert_eq!(wcscspn(&s, &reject), 2);
        assert_eq!(wcspbrk(&s, &accept), Some(0));
    }

    #[test]
    fn test_wcscasecmp_wcsncasecmp() {
        let s1 = [b'A' as wchar_t, b'b' as wchar_t, 0];
        let s2 = [b'a' as wchar_t, b'B' as wchar_t, 0];
        assert_eq!(wcscasecmp(&s1, &s2), 0);
        assert_eq!(wcsncasecmp(&s1, &s2, 1), 0);
        assert_eq!(wcsncasecmp(&s1, &s2, 2), 0);
    }

    #[test]
    fn test_wcpcpy_wcpncpy() {
        let src = [b'h' as wchar_t, b'i' as wchar_t, 0];
        let mut dest = [0 as wchar_t; 4];
        let end = wcpcpy(&mut dest, &src);
        assert_eq!(end, 2);
        assert_eq!(dest[..3], [b'h' as wchar_t, b'i' as wchar_t, 0]);

        let mut dest2 = [b'x' as wchar_t; 4];
        let end2 = wcpncpy(&mut dest2, &src, 2);
        assert_eq!(end2, 2);
        assert_eq!(dest2[..2], [b'h' as wchar_t, b'i' as wchar_t]);

        let mut dest3 = [b'x' as wchar_t; 4];
        let end3 = wcpncpy(&mut dest3, &src, 3);
        assert_eq!(end3, 2);
        assert_eq!(dest3[..3], [b'h' as wchar_t, b'i' as wchar_t, 0]);
    }

    #[test]
    fn test_wcschrnul() {
        let s = [b'a' as wchar_t, b'b' as wchar_t, 0];
        assert_eq!(wcschrnul(&s, b'a' as wchar_t), 0);
        assert_eq!(wcschrnul(&s, b'b' as wchar_t), 1);
        assert_eq!(wcschrnul(&s, b'c' as wchar_t), 2);
        assert_eq!(wcschrnul(&s, 0), 2);
    }

    #[test]
    fn test_wcslcpy_wcslcat() {
        let src = [b'h' as wchar_t, b'i' as wchar_t, 0];
        let mut dest = [0 as wchar_t; 3];
        let len = wcslcpy(&mut dest, &src);
        assert_eq!(len, 2);
        assert_eq!(dest, [b'h' as wchar_t, b'i' as wchar_t, 0]);

        let mut dest2 = [b'a' as wchar_t, 0, 0, 0];
        let src2 = [b'b' as wchar_t, b'c' as wchar_t, 0];
        let total = wcslcat(&mut dest2, &src2);
        assert_eq!(total, 3);
        assert_eq!(
            dest2[..3],
            [b'a' as wchar_t, b'b' as wchar_t, b'c' as wchar_t]
        );
    }
}
