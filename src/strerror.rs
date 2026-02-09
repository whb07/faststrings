//! `strerror` implementation.
//!
//! Returns static, nul-terminated English error descriptions for common Linux
//! errno values. Unknown values map to a generic fallback string.

/// Generic fallback used when `errnum` is unknown.
pub const UNKNOWN_ERROR_MESSAGE: &[u8] = b"Unknown error\0";

/// Returns a static, nul-terminated error message for `errnum`.
pub fn strerror(errnum: i32) -> &'static [u8] {
    lookup_error_message(errnum).unwrap_or(UNKNOWN_ERROR_MESSAGE)
}

/// Internal lookup used by both `strerror` and `strerror_r`.
pub(crate) fn lookup_error_message(errnum: i32) -> Option<&'static [u8]> {
    match errnum {
        0 => Some(b"Success\0"),
        1 => Some(b"Operation not permitted\0"),
        2 => Some(b"No such file or directory\0"),
        3 => Some(b"No such process\0"),
        4 => Some(b"Interrupted system call\0"),
        5 => Some(b"Input/output error\0"),
        6 => Some(b"No such device or address\0"),
        7 => Some(b"Argument list too long\0"),
        8 => Some(b"Exec format error\0"),
        9 => Some(b"Bad file descriptor\0"),
        10 => Some(b"No child processes\0"),
        11 => Some(b"Resource temporarily unavailable\0"),
        12 => Some(b"Cannot allocate memory\0"),
        13 => Some(b"Permission denied\0"),
        14 => Some(b"Bad address\0"),
        15 => Some(b"Block device required\0"),
        16 => Some(b"Device or resource busy\0"),
        17 => Some(b"File exists\0"),
        18 => Some(b"Invalid cross-device link\0"),
        19 => Some(b"No such device\0"),
        20 => Some(b"Not a directory\0"),
        21 => Some(b"Is a directory\0"),
        22 => Some(b"Invalid argument\0"),
        23 => Some(b"Too many open files in system\0"),
        24 => Some(b"Too many open files\0"),
        25 => Some(b"Inappropriate ioctl for device\0"),
        26 => Some(b"Text file busy\0"),
        27 => Some(b"File too large\0"),
        28 => Some(b"No space left on device\0"),
        29 => Some(b"Illegal seek\0"),
        30 => Some(b"Read-only file system\0"),
        31 => Some(b"Too many links\0"),
        32 => Some(b"Broken pipe\0"),
        33 => Some(b"Numerical argument out of domain\0"),
        34 => Some(b"Numerical result out of range\0"),
        35 => Some(b"Resource deadlock avoided\0"),
        36 => Some(b"File name too long\0"),
        37 => Some(b"No locks available\0"),
        38 => Some(b"Function not implemented\0"),
        39 => Some(b"Directory not empty\0"),
        40 => Some(b"Too many levels of symbolic links\0"),
        42 => Some(b"No message of desired type\0"),
        43 => Some(b"Identifier removed\0"),
        44 => Some(b"Channel number out of range\0"),
        45 => Some(b"Level 2 not synchronized\0"),
        46 => Some(b"Level 3 halted\0"),
        47 => Some(b"Level 3 reset\0"),
        48 => Some(b"Link number out of range\0"),
        49 => Some(b"Protocol driver not attached\0"),
        50 => Some(b"No CSI structure available\0"),
        51 => Some(b"Level 2 halted\0"),
        52 => Some(b"Invalid exchange\0"),
        53 => Some(b"Invalid request descriptor\0"),
        54 => Some(b"Exchange full\0"),
        55 => Some(b"No anode\0"),
        56 => Some(b"Invalid request code\0"),
        57 => Some(b"Invalid slot\0"),
        59 => Some(b"Bad font file format\0"),
        60 => Some(b"Device not a stream\0"),
        61 => Some(b"No data available\0"),
        62 => Some(b"Timer expired\0"),
        63 => Some(b"Out of streams resources\0"),
        64 => Some(b"Machine is not on the network\0"),
        65 => Some(b"Package not installed\0"),
        66 => Some(b"Object is remote\0"),
        67 => Some(b"Link has been severed\0"),
        68 => Some(b"Advertise error\0"),
        69 => Some(b"Srmount error\0"),
        70 => Some(b"Communication error on send\0"),
        71 => Some(b"Protocol error\0"),
        72 => Some(b"Multihop attempted\0"),
        73 => Some(b"RFS specific error\0"),
        74 => Some(b"Bad message\0"),
        75 => Some(b"Value too large for defined data type\0"),
        76 => Some(b"Name not unique on network\0"),
        77 => Some(b"File descriptor in bad state\0"),
        78 => Some(b"Remote address changed\0"),
        79 => Some(b"Can not access a needed shared library\0"),
        80 => Some(b"Accessing a corrupted shared library\0"),
        81 => Some(b".lib section in a.out corrupted\0"),
        82 => Some(b"Attempting to link in too many shared libraries\0"),
        83 => Some(b"Cannot exec a shared library directly\0"),
        84 => Some(b"Invalid or incomplete multibyte or wide character\0"),
        85 => Some(b"Interrupted system call should be restarted\0"),
        86 => Some(b"Streams pipe error\0"),
        87 => Some(b"Too many users\0"),
        88 => Some(b"Socket operation on non-socket\0"),
        89 => Some(b"Destination address required\0"),
        90 => Some(b"Message too long\0"),
        91 => Some(b"Protocol wrong type for socket\0"),
        92 => Some(b"Protocol not available\0"),
        93 => Some(b"Protocol not supported\0"),
        94 => Some(b"Socket type not supported\0"),
        95 => Some(b"Operation not supported\0"),
        96 => Some(b"Protocol family not supported\0"),
        97 => Some(b"Address family not supported by protocol\0"),
        98 => Some(b"Address already in use\0"),
        99 => Some(b"Cannot assign requested address\0"),
        100 => Some(b"Network is down\0"),
        101 => Some(b"Network is unreachable\0"),
        102 => Some(b"Network dropped connection on reset\0"),
        103 => Some(b"Software caused connection abort\0"),
        104 => Some(b"Connection reset by peer\0"),
        105 => Some(b"No buffer space available\0"),
        106 => Some(b"Transport endpoint is already connected\0"),
        107 => Some(b"Transport endpoint is not connected\0"),
        108 => Some(b"Cannot send after transport endpoint shutdown\0"),
        109 => Some(b"Too many references: cannot splice\0"),
        110 => Some(b"Connection timed out\0"),
        111 => Some(b"Connection refused\0"),
        112 => Some(b"Host is down\0"),
        113 => Some(b"No route to host\0"),
        114 => Some(b"Operation already in progress\0"),
        115 => Some(b"Operation now in progress\0"),
        116 => Some(b"Stale file handle\0"),
        117 => Some(b"Structure needs cleaning\0"),
        118 => Some(b"Not a XENIX named type file\0"),
        119 => Some(b"No XENIX semaphores available\0"),
        120 => Some(b"Is a named type file\0"),
        121 => Some(b"Remote I/O error\0"),
        122 => Some(b"Disk quota exceeded\0"),
        123 => Some(b"No medium found\0"),
        124 => Some(b"Wrong medium type\0"),
        125 => Some(b"Operation canceled\0"),
        126 => Some(b"Required key not available\0"),
        127 => Some(b"Key has expired\0"),
        128 => Some(b"Key has been revoked\0"),
        129 => Some(b"Key was rejected by service\0"),
        130 => Some(b"Owner died\0"),
        131 => Some(b"State not recoverable\0"),
        132 => Some(b"Operation not possible due to RF-kill\0"),
        133 => Some(b"Memory page has hardware error\0"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{strerror, UNKNOWN_ERROR_MESSAGE};

    #[test]
    fn test_strerror_known_values() {
        assert_eq!(strerror(0), b"Success\0");
        assert_eq!(strerror(2), b"No such file or directory\0");
        assert_eq!(strerror(22), b"Invalid argument\0");
        assert_eq!(strerror(110), b"Connection timed out\0");
    }

    #[test]
    fn test_strerror_unknown_value() {
        assert_eq!(strerror(10_000), UNKNOWN_ERROR_MESSAGE);
        assert_eq!(strerror(-1), UNKNOWN_ERROR_MESSAGE);
    }
}
