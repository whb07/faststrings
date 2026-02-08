//! String tokenization functions
//!
//! Safe Rust implementations of string tokenization using iterators.

use crate::str::strlen;

/// Iterator-based string tokenizer
///
/// A safe, iterator-based replacement for strtok/strtok_r.
///
/// # Examples
/// ```
/// use faststrings::token::Tokenizer;
///
/// let s = b"hello,world,foo\0";
/// let delim = b",\0";
/// let mut tok = Tokenizer::new(s, delim);
///
/// assert_eq!(tok.next(), Some(&b"hello"[..]));
/// assert_eq!(tok.next(), Some(&b"world"[..]));
/// assert_eq!(tok.next(), Some(&b"foo"[..]));
/// assert_eq!(tok.next(), None);
/// ```
pub struct Tokenizer<'a> {
    data: &'a [u8],
    delimiters: &'a [u8],
    position: usize,
    str_len: usize,
    delim_len: usize,
}

impl<'a> Tokenizer<'a> {
    /// Create a new tokenizer
    ///
    /// Both `data` and `delimiters` are treated as null-terminated strings.
    pub fn new(data: &'a [u8], delimiters: &'a [u8]) -> Self {
        let str_len = strlen(data);
        let delim_len = strlen(delimiters);

        Self {
            data,
            delimiters,
            position: 0,
            str_len,
            delim_len,
        }
    }

    /// Create a tokenizer from a byte slice (not null-terminated)
    pub fn from_slice(data: &'a [u8], delimiters: &'a [u8]) -> Self {
        Self {
            data,
            delimiters,
            position: 0,
            str_len: data.len(),
            delim_len: delimiters.len(),
        }
    }

    /// Check if a byte is a delimiter
    fn is_delimiter(&self, c: u8) -> bool {
        let delim = &self.delimiters[..self.delim_len.min(self.delimiters.len())];
        delim.contains(&c)
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        let data = &self.data[..self.str_len.min(self.data.len())];

        // Skip leading delimiters
        while self.position < data.len() && self.is_delimiter(data[self.position]) {
            self.position += 1;
        }

        if self.position >= data.len() {
            return None;
        }

        let start = self.position;

        // Find end of token
        while self.position < data.len() && !self.is_delimiter(data[self.position]) {
            self.position += 1;
        }

        Some(&data[start..self.position])
    }
}

/// Split iterator that preserves empty fields
///
/// Unlike Tokenizer (which mimics strtok), this iterator preserves empty
/// fields between consecutive delimiters, similar to strsep.
///
/// # Examples
/// ```
/// use faststrings::token::Splitter;
///
/// let s = b"a,,b,\0";
/// let delim = b",\0";
/// let mut split = Splitter::new(s, delim);
///
/// assert_eq!(split.next(), Some(&b"a"[..]));
/// assert_eq!(split.next(), Some(&b""[..]));  // empty field
/// assert_eq!(split.next(), Some(&b"b"[..]));
/// assert_eq!(split.next(), Some(&b""[..]));  // trailing empty
/// assert_eq!(split.next(), None);
/// ```
pub struct Splitter<'a> {
    data: &'a [u8],
    delimiters: &'a [u8],
    position: usize,
    str_len: usize,
    delim_len: usize,
    done: bool,
}

impl<'a> Splitter<'a> {
    /// Create a new splitter
    pub fn new(data: &'a [u8], delimiters: &'a [u8]) -> Self {
        let str_len = strlen(data);
        let delim_len = strlen(delimiters);

        Self {
            data,
            delimiters,
            position: 0,
            str_len,
            delim_len,
            done: false,
        }
    }

    /// Create a splitter from a byte slice (not null-terminated)
    pub fn from_slice(data: &'a [u8], delimiters: &'a [u8]) -> Self {
        Self {
            data,
            delimiters,
            position: 0,
            str_len: data.len(),
            delim_len: delimiters.len(),
            done: false,
        }
    }

    fn is_delimiter(&self, c: u8) -> bool {
        let delim = &self.delimiters[..self.delim_len.min(self.delimiters.len())];
        delim.contains(&c)
    }
}

impl<'a> Iterator for Splitter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let data = &self.data[..self.str_len.min(self.data.len())];

        if self.position > data.len() {
            self.done = true;
            return None;
        }

        let start = self.position;

        // Find next delimiter
        while self.position < data.len() && !self.is_delimiter(data[self.position]) {
            self.position += 1;
        }

        let end = self.position;

        if self.position < data.len() {
            self.position += 1; // Skip delimiter
        } else {
            self.done = true;
        }

        Some(&data[start..end])
    }
}

/// Tokenize a string (functional style)
///
/// Returns a Tokenizer iterator. This is the safe Rust replacement for strtok.
///
/// # Examples
/// ```
/// use faststrings::token::strtok_iter;
///
/// let tokens: Vec<_> = strtok_iter(b"a,b,c\0", b",\0").collect();
/// assert_eq!(tokens, vec![&b"a"[..], &b"b"[..], &b"c"[..]]);
/// ```
pub fn strtok_iter<'a>(s: &'a [u8], delim: &'a [u8]) -> Tokenizer<'a> {
    Tokenizer::new(s, delim)
}

/// Split a string preserving empty fields (functional style)
///
/// Returns a Splitter iterator. This is the safe Rust replacement for strsep.
pub fn strsep_iter<'a>(s: &'a [u8], delim: &'a [u8]) -> Splitter<'a> {
    Splitter::new(s, delim)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenizer_basic() {
        let s = b"hello,world,foo\0";
        let delim = b",\0";
        let mut tok = Tokenizer::new(s, delim);
        assert_eq!(tok.next(), Some(&b"hello"[..]));
        assert_eq!(tok.next(), Some(&b"world"[..]));
        assert_eq!(tok.next(), Some(&b"foo"[..]));
        assert_eq!(tok.next(), None);
    }

    #[test]
    fn test_splitter_preserves_empty() {
        let s = b"a,,b,\0";
        let delim = b",\0";
        let mut split = Splitter::new(s, delim);
        assert_eq!(split.next(), Some(&b"a"[..]));
        assert_eq!(split.next(), Some(&b""[..]));
        assert_eq!(split.next(), Some(&b"b"[..]));
        assert_eq!(split.next(), Some(&b""[..]));
        assert_eq!(split.next(), None);
    }

    #[test]
    fn test_iter_helpers() {
        let mut tokens = strtok_iter(b"a,b,c\0", b",\0");
        assert_eq!(tokens.next(), Some(&b"a"[..]));
        assert_eq!(tokens.next(), Some(&b"b"[..]));
        assert_eq!(tokens.next(), Some(&b"c"[..]));
        assert_eq!(tokens.next(), None);

        let mut tokens = strsep_iter(b"a,,\0", b",\0");
        assert_eq!(tokens.next(), Some(&b"a"[..]));
        assert_eq!(tokens.next(), Some(&b""[..]));
        assert_eq!(tokens.next(), Some(&b""[..]));
        assert_eq!(tokens.next(), None);
    }
}
