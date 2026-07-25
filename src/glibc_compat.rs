//! Forward-compatibility shim for glibc >= 2.38 symbols.
//!
//! `ort` links against a prebuilt ONNX Runtime that was compiled on a system
//! with glibc >= 2.38. There, `<stdlib.h>` redirects the C23 string-to-integer
//! functions to `__isoc23_*` variants, so the archive carries undefined
//! references to them.
//!
//! The release runner is ubuntu-22.04 (glibc 2.35), which predates those
//! symbols, and the link fails with:
//!
//! ```text
//! rust-lld: error: undefined symbol: __isoc23_strtoll
//!   >>> referenced by parser.cc in libort_sys-....rlib
//! ```
//!
//! The only difference between `__isoc23_strtoX` and plain `strtoX` is C23
//! handling of the `0b` binary prefix when base is 0 or 2 - irrelevant to how
//! ONNX Runtime parses model metadata. So we define the three missing symbols
//! and forward them to the classic ones.
//!
//! These live in the binary crate's own object files rather than in a static
//! archive, so the linker always sees them - no archive-ordering games needed.
//!
//! Remove this module once the release workflow moves to ubuntu-24.04.

#![allow(non_snake_case)]

#[cfg(all(target_os = "linux", target_env = "gnu"))]
mod linux_gnu {
    use std::os::raw::{c_char, c_int, c_long, c_longlong, c_ulonglong};

    extern "C" {
        fn strtol(s: *const c_char, endp: *mut *mut c_char, base: c_int) -> c_long;
        fn strtoll(s: *const c_char, endp: *mut *mut c_char, base: c_int) -> c_longlong;
        fn strtoull(s: *const c_char, endp: *mut *mut c_char, base: c_int) -> c_ulonglong;
    }

    #[no_mangle]
    pub unsafe extern "C" fn __isoc23_strtol(
        s: *const c_char,
        endp: *mut *mut c_char,
        base: c_int,
    ) -> c_long {
        strtol(s, endp, base)
    }

    #[no_mangle]
    pub unsafe extern "C" fn __isoc23_strtoll(
        s: *const c_char,
        endp: *mut *mut c_char,
        base: c_int,
    ) -> c_longlong {
        strtoll(s, endp, base)
    }

    #[no_mangle]
    pub unsafe extern "C" fn __isoc23_strtoull(
        s: *const c_char,
        endp: *mut *mut c_char,
        base: c_int,
    ) -> c_ulonglong {
        strtoull(s, endp, base)
    }
}
