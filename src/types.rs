//! C-compatible type aliases used by this crate.

#![allow(non_camel_case_types)]

/// C `wchar_t` equivalent.
///
/// Windows uses 16-bit UTF-16 code units, while Unix-like targets typically
/// use 32-bit wide characters.
#[cfg(target_os = "windows")]
pub type wchar_t = u16;

/// C `wchar_t` equivalent on non-Windows targets.
#[cfg(not(target_os = "windows"))]
pub type wchar_t = i32;
