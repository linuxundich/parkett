//! Raw bindings to libghostty-vt 1.0.0, generated with bindgen from the 1.0.0 headers:
//! `bindgen include/ghostty/vt.h --use-core --no-layout-tests --no-doc-comments
//!  --allowlist-function 'ghostty_.*' --allowlist-type 'Ghostty.*' --allowlist-var 'GHOSTTY_.*'
//!  --default-enum-style consts`
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, dead_code, clippy::all)]

mod bindings;
pub use bindings::*;

#[cfg(test)]
mod tests {
    use super::*;

    unsafe fn text(t: GhosttyTerminal) -> String {
        let mut f: GhosttyFormatter = core::ptr::null_mut();
        assert_eq!(ghostty_formatter_terminal_new(core::ptr::null(), &mut f, t), 0);
        let (mut p, mut n) = (core::ptr::null_mut(), 0usize);
        assert_eq!(ghostty_formatter_format_alloc(f, core::ptr::null(), &mut p, &mut n), 0);
        let s = String::from_utf8_lossy(core::slice::from_raw_parts(p, n)).into_owned();
        ghostty_free(core::ptr::null(), p, n);
        ghostty_formatter_free(f);
        s
    }

    #[test]
    fn hello_title_and_pwd() {
        unsafe {
            let mut t: GhosttyTerminal = core::ptr::null_mut();
            assert_eq!(ghostty_terminal_new(core::ptr::null(), &mut t, 80, 24), 0);
            let data = b"Hello, \x1b[1;32mworld\x1b[0m!\r\n\x1b]2;my title\x07\x1b]7;file://h/tmp/x\x07";
            ghostty_terminal_vt_write(t, data.as_ptr(), data.len());
            assert!(text(t).starts_with("Hello, world!"));
            let mut s = GhosttyString { ptr: core::ptr::null(), len: 0 };
            let r =
                ghostty_terminal_get(t, GhosttyTerminalData_GHOSTTY_TERMINAL_DATA_TITLE, &mut s as *mut _ as *mut _);
            assert_eq!(r, 0);
            assert_eq!(core::slice::from_raw_parts(s.ptr, s.len), b"my title");
            let r = ghostty_terminal_get(t, GhosttyTerminalData_GHOSTTY_TERMINAL_DATA_PWD, &mut s as *mut _ as *mut _);
            assert_eq!(r, 0);
            assert!(core::slice::from_raw_parts(s.ptr, s.len).ends_with(b"/tmp/x"));
            ghostty_terminal_free(t);
        }
    }
}
