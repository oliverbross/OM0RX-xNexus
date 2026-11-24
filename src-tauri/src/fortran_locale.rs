// SPDX-License-Identifier: GPL-3.0-only
//! `setlocale` for the statically linked GNU Fortran runtime, on Windows (OM0RX-xNexus).
//!
//! Around every formatted READ or WRITE, internal ones included, libgfortran sets the numeric locale
//! to `"C"` and then back: it keeps the pointer `setlocale(LC_NUMERIC, NULL)` returned, without
//! copying the text, and hands it to `setlocale` again afterwards. In the Universal C runtime each
//! `setlocale` call replaces and frees that text, so what is handed back is always freed memory.
//! The decoders write that way on every decode thread, thousands of times a period. Now and then
//! another thread has taken the memory meanwhile, and the runtime ends the process on an invalid
//! argument (0xC0000409 in ucrtbase.dll; live run 2026-11-24 of 1.25.0-xlog.28: `chkcrc14a` in the
//! JTDX-style FT8 decoder, after five hours). A test program with two writing threads handed back
//! garbage 7,880 times in 20 seconds.
//!
//! The executable is linked with `--wrap=setlocale` (build.rs), so every caller comes here. The
//! numeric locale is set to `"C"` once and stays there, which is what the runtime wants for its
//! formatting and what it would set each time, and its switching does nothing. Every other category
//! goes to the C runtime.

use std::ffi::{c_char, c_int, CStr};
use std::sync::Once;

const LC_ALL: c_int = 0;
const LC_NUMERIC: c_int = 4;
const C_LOCALE: &CStr = c"C";
static NUMERIC_C: Once = Once::new();

extern "C" {
    /// The C runtime's own `setlocale` (`ld --wrap`).
    fn __real_setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
}

/// # Safety
/// `setlocale`'s own: `locale` is null or a NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn __wrap_setlocale(category: c_int, locale: *const c_char) -> *mut c_char {
    if category == LC_NUMERIC {
        // A static answer: whoever keeps it may hand it back at any time.
        NUMERIC_C.call_once(|| unsafe {
            __real_setlocale(LC_NUMERIC, C_LOCALE.as_ptr());
        });
        return C_LOCALE.as_ptr().cast_mut();
    }
    let was = unsafe { __real_setlocale(category, locale) };
    if category == LC_ALL && !locale.is_null() {
        // It set the numeric locale too.
        unsafe { __real_setlocale(LC_NUMERIC, C_LOCALE.as_ptr()) };
    }
    was
}

/// Keeps the symbol in the executable: only the Fortran runtime's objects name it.
#[used]
static KEEP: unsafe extern "C" fn(c_int, *const c_char) -> *mut c_char = __wrap_setlocale;
