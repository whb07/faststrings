//! `strtok_r` implementation.

use crate::str::strlen;

/// Reentrant tokenizer over a nul-terminated byte string.
///
/// `saveptr` stores scan position between calls.
/// Initialize `*saveptr = 0` before the first call.
pub fn strtok_r<'a>(s: &'a [u8], delim: &[u8], saveptr: &mut usize) -> Option<&'a [u8]> {
    let len = strlen(s).min(s.len());
    let delim_len = strlen(delim).min(delim.len());
    let delim = &delim[..delim_len];

    let mut pos = (*saveptr).min(len + 1);
    if pos > len {
        return None;
    }

    while pos < len && delim.contains(&s[pos]) {
        pos += 1;
    }

    if pos >= len {
        *saveptr = len + 1;
        return None;
    }

    let start = pos;
    while pos < len && !delim.contains(&s[pos]) {
        pos += 1;
    }

    *saveptr = if pos < len { pos + 1 } else { len + 1 };
    Some(&s[start..pos])
}

#[cfg(test)]
mod tests {
    use super::strtok_r;

    #[test]
    fn test_strtok_r_sequence() {
        let s = b"aa,bb,,cc\0";
        let mut save = 0usize;

        assert_eq!(strtok_r(s, b",\0", &mut save), Some(&b"aa"[..]));
        assert_eq!(strtok_r(s, b",\0", &mut save), Some(&b"bb"[..]));
        assert_eq!(strtok_r(s, b",\0", &mut save), Some(&b"cc"[..]));
        assert_eq!(strtok_r(s, b",\0", &mut save), None);
    }

    #[test]
    fn test_strtok_r_all_delims() {
        let s = b",,,\0";
        let mut save = 0usize;
        assert_eq!(strtok_r(s, b",\0", &mut save), None);
    }
}
