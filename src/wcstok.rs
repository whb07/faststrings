//! `wcstok` implementation.

use crate::types::wchar_t;
use crate::wide::wcslen;

/// Reentrant tokenizer over a nul-terminated wide string.
///
/// `saveptr` stores scan position between calls.
/// Initialize `*saveptr = 0` before the first call.
pub fn wcstok<'a>(
    s: &'a [wchar_t],
    delim: &[wchar_t],
    saveptr: &mut usize,
) -> Option<&'a [wchar_t]> {
    let len = wcslen(s).min(s.len());
    let delim_len = wcslen(delim).min(delim.len());
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
    use super::wcstok;
    use crate::types::wchar_t;

    #[test]
    fn test_wcstok_sequence() {
        let s = [
            b'a' as wchar_t,
            b',' as wchar_t,
            b'b' as wchar_t,
            b',' as wchar_t,
            b'c' as wchar_t,
            0,
        ];
        let delim = [b',' as wchar_t, 0];
        let mut save = 0usize;

        assert_eq!(wcstok(&s, &delim, &mut save), Some(&[b'a' as wchar_t][..]));
        assert_eq!(wcstok(&s, &delim, &mut save), Some(&[b'b' as wchar_t][..]));
        assert_eq!(wcstok(&s, &delim, &mut save), Some(&[b'c' as wchar_t][..]));
        assert_eq!(wcstok(&s, &delim, &mut save), None);
    }
}
