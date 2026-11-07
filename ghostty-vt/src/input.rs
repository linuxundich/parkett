//! Keyboard input: GDK reports evdev key codes (+8); libghostty-vt wants physical keys.
use crate::Terminal;
use ghostty_vt_sys as sys;
use std::ptr::{null, null_mut};

pub const MOD_SHIFT: u32 = sys::GHOSTTY_MODS_SHIFT;
pub const MOD_CTRL: u32 = sys::GHOSTTY_MODS_CTRL;
pub const MOD_ALT: u32 = sys::GHOSTTY_MODS_ALT;
pub const MOD_SUPER: u32 = sys::GHOSTTY_MODS_SUPER;
pub const MOD_CAPS_LOCK: u32 = sys::GHOSTTY_MODS_CAPS_LOCK;
pub const MOD_NUM_LOCK: u32 = sys::GHOSTTY_MODS_NUM_LOCK;

/// Physical key for a Linux evdev key code (`KEY_*` from `linux/input-event-codes.h`).
pub fn key_from_evdev(code: u32) -> Option<u32> {
    Some(match code {
        1 => sys::GhosttyKey_GHOSTTY_KEY_ESCAPE,
        2 => sys::GhosttyKey_GHOSTTY_KEY_DIGIT_1,
        3 => sys::GhosttyKey_GHOSTTY_KEY_DIGIT_2,
        4 => sys::GhosttyKey_GHOSTTY_KEY_DIGIT_3,
        5 => sys::GhosttyKey_GHOSTTY_KEY_DIGIT_4,
        6 => sys::GhosttyKey_GHOSTTY_KEY_DIGIT_5,
        7 => sys::GhosttyKey_GHOSTTY_KEY_DIGIT_6,
        8 => sys::GhosttyKey_GHOSTTY_KEY_DIGIT_7,
        9 => sys::GhosttyKey_GHOSTTY_KEY_DIGIT_8,
        10 => sys::GhosttyKey_GHOSTTY_KEY_DIGIT_9,
        11 => sys::GhosttyKey_GHOSTTY_KEY_DIGIT_0,
        12 => sys::GhosttyKey_GHOSTTY_KEY_MINUS,
        13 => sys::GhosttyKey_GHOSTTY_KEY_EQUAL,
        14 => sys::GhosttyKey_GHOSTTY_KEY_BACKSPACE,
        15 => sys::GhosttyKey_GHOSTTY_KEY_TAB,
        16 => sys::GhosttyKey_GHOSTTY_KEY_Q,
        17 => sys::GhosttyKey_GHOSTTY_KEY_W,
        18 => sys::GhosttyKey_GHOSTTY_KEY_E,
        19 => sys::GhosttyKey_GHOSTTY_KEY_R,
        20 => sys::GhosttyKey_GHOSTTY_KEY_T,
        21 => sys::GhosttyKey_GHOSTTY_KEY_Y,
        22 => sys::GhosttyKey_GHOSTTY_KEY_U,
        23 => sys::GhosttyKey_GHOSTTY_KEY_I,
        24 => sys::GhosttyKey_GHOSTTY_KEY_O,
        25 => sys::GhosttyKey_GHOSTTY_KEY_P,
        26 => sys::GhosttyKey_GHOSTTY_KEY_BRACKET_LEFT,
        27 => sys::GhosttyKey_GHOSTTY_KEY_BRACKET_RIGHT,
        28 => sys::GhosttyKey_GHOSTTY_KEY_ENTER,
        29 => sys::GhosttyKey_GHOSTTY_KEY_CONTROL_LEFT,
        30 => sys::GhosttyKey_GHOSTTY_KEY_A,
        31 => sys::GhosttyKey_GHOSTTY_KEY_S,
        32 => sys::GhosttyKey_GHOSTTY_KEY_D,
        33 => sys::GhosttyKey_GHOSTTY_KEY_F,
        34 => sys::GhosttyKey_GHOSTTY_KEY_G,
        35 => sys::GhosttyKey_GHOSTTY_KEY_H,
        36 => sys::GhosttyKey_GHOSTTY_KEY_J,
        37 => sys::GhosttyKey_GHOSTTY_KEY_K,
        38 => sys::GhosttyKey_GHOSTTY_KEY_L,
        39 => sys::GhosttyKey_GHOSTTY_KEY_SEMICOLON,
        40 => sys::GhosttyKey_GHOSTTY_KEY_QUOTE,
        41 => sys::GhosttyKey_GHOSTTY_KEY_BACKQUOTE,
        42 => sys::GhosttyKey_GHOSTTY_KEY_SHIFT_LEFT,
        43 => sys::GhosttyKey_GHOSTTY_KEY_BACKSLASH,
        44 => sys::GhosttyKey_GHOSTTY_KEY_Z,
        45 => sys::GhosttyKey_GHOSTTY_KEY_X,
        46 => sys::GhosttyKey_GHOSTTY_KEY_C,
        47 => sys::GhosttyKey_GHOSTTY_KEY_V,
        48 => sys::GhosttyKey_GHOSTTY_KEY_B,
        49 => sys::GhosttyKey_GHOSTTY_KEY_N,
        50 => sys::GhosttyKey_GHOSTTY_KEY_M,
        51 => sys::GhosttyKey_GHOSTTY_KEY_COMMA,
        52 => sys::GhosttyKey_GHOSTTY_KEY_PERIOD,
        53 => sys::GhosttyKey_GHOSTTY_KEY_SLASH,
        54 => sys::GhosttyKey_GHOSTTY_KEY_SHIFT_RIGHT,
        55 => sys::GhosttyKey_GHOSTTY_KEY_NUMPAD_MULTIPLY,
        56 => sys::GhosttyKey_GHOSTTY_KEY_ALT_LEFT,
        57 => sys::GhosttyKey_GHOSTTY_KEY_SPACE,
        58 => sys::GhosttyKey_GHOSTTY_KEY_CAPS_LOCK,
        59 => sys::GhosttyKey_GHOSTTY_KEY_F1,
        60 => sys::GhosttyKey_GHOSTTY_KEY_F2,
        61 => sys::GhosttyKey_GHOSTTY_KEY_F3,
        62 => sys::GhosttyKey_GHOSTTY_KEY_F4,
        63 => sys::GhosttyKey_GHOSTTY_KEY_F5,
        64 => sys::GhosttyKey_GHOSTTY_KEY_F6,
        65 => sys::GhosttyKey_GHOSTTY_KEY_F7,
        66 => sys::GhosttyKey_GHOSTTY_KEY_F8,
        67 => sys::GhosttyKey_GHOSTTY_KEY_F9,
        68 => sys::GhosttyKey_GHOSTTY_KEY_F10,
        69 => sys::GhosttyKey_GHOSTTY_KEY_NUM_LOCK,
        70 => sys::GhosttyKey_GHOSTTY_KEY_SCROLL_LOCK,
        71 => sys::GhosttyKey_GHOSTTY_KEY_NUMPAD_7,
        72 => sys::GhosttyKey_GHOSTTY_KEY_NUMPAD_8,
        73 => sys::GhosttyKey_GHOSTTY_KEY_NUMPAD_9,
        74 => sys::GhosttyKey_GHOSTTY_KEY_NUMPAD_SUBTRACT,
        75 => sys::GhosttyKey_GHOSTTY_KEY_NUMPAD_4,
        76 => sys::GhosttyKey_GHOSTTY_KEY_NUMPAD_5,
        77 => sys::GhosttyKey_GHOSTTY_KEY_NUMPAD_6,
        78 => sys::GhosttyKey_GHOSTTY_KEY_NUMPAD_ADD,
        79 => sys::GhosttyKey_GHOSTTY_KEY_NUMPAD_1,
        80 => sys::GhosttyKey_GHOSTTY_KEY_NUMPAD_2,
        81 => sys::GhosttyKey_GHOSTTY_KEY_NUMPAD_3,
        82 => sys::GhosttyKey_GHOSTTY_KEY_NUMPAD_0,
        83 => sys::GhosttyKey_GHOSTTY_KEY_NUMPAD_DECIMAL,
        86 => sys::GhosttyKey_GHOSTTY_KEY_INTL_BACKSLASH,
        87 => sys::GhosttyKey_GHOSTTY_KEY_F11,
        88 => sys::GhosttyKey_GHOSTTY_KEY_F12,
        96 => sys::GhosttyKey_GHOSTTY_KEY_NUMPAD_ENTER,
        97 => sys::GhosttyKey_GHOSTTY_KEY_CONTROL_RIGHT,
        98 => sys::GhosttyKey_GHOSTTY_KEY_NUMPAD_DIVIDE,
        99 => sys::GhosttyKey_GHOSTTY_KEY_PRINT_SCREEN,
        100 => sys::GhosttyKey_GHOSTTY_KEY_ALT_RIGHT,
        102 => sys::GhosttyKey_GHOSTTY_KEY_HOME,
        103 => sys::GhosttyKey_GHOSTTY_KEY_ARROW_UP,
        104 => sys::GhosttyKey_GHOSTTY_KEY_PAGE_UP,
        105 => sys::GhosttyKey_GHOSTTY_KEY_ARROW_LEFT,
        106 => sys::GhosttyKey_GHOSTTY_KEY_ARROW_RIGHT,
        107 => sys::GhosttyKey_GHOSTTY_KEY_END,
        108 => sys::GhosttyKey_GHOSTTY_KEY_ARROW_DOWN,
        109 => sys::GhosttyKey_GHOSTTY_KEY_PAGE_DOWN,
        110 => sys::GhosttyKey_GHOSTTY_KEY_INSERT,
        111 => sys::GhosttyKey_GHOSTTY_KEY_DELETE,
        119 => sys::GhosttyKey_GHOSTTY_KEY_PAUSE,
        125 => sys::GhosttyKey_GHOSTTY_KEY_META_LEFT,
        126 => sys::GhosttyKey_GHOSTTY_KEY_META_RIGHT,
        127 => sys::GhosttyKey_GHOSTTY_KEY_CONTEXT_MENU,
        _ => return None,
    })
}

/// One key press or release.
pub struct KeyEvent<'a> {
    /// From [`key_from_evdev`].
    pub key: u32,
    pub mods: u32,
    /// Text the key produces with the current layout (empty for non-text keys).
    pub utf8: &'a str,
    /// Codepoint of the key without Shift (for the Kitty protocol), 0 if none.
    pub unshifted: u32,
    pub press: bool,
    /// The modifiers that were used up to produce `utf8` (usually Shift).
    pub consumed_mods: u32,
}

pub struct KeyEncoder {
    enc: sys::GhosttyKeyEncoder,
    ev: sys::GhosttyKeyEvent,
}

impl KeyEncoder {
    pub fn new() -> Result<Self, i32> {
        let (mut enc, mut ev) = (null_mut(), null_mut());
        // SAFETY: valid out pointers, default allocator.
        unsafe {
            let r = sys::ghostty_key_encoder_new(null(), &mut enc);
            if r != 0 {
                return Err(r);
            }
            let r = sys::ghostty_key_event_new(null(), &mut ev);
            if r != 0 {
                sys::ghostty_key_encoder_free(enc);
                return Err(r);
            }
        }
        Ok(Self { enc, ev })
    }

    /// Bytes to send to the program for `e`, honouring the terminal's current modes (application
    /// cursor keys, Kitty keyboard flags, …). Empty when the key sends nothing.
    pub fn encode(&mut self, term: &Terminal, e: &KeyEvent) -> Vec<u8> {
        let mut buf = [0u8; 128];
        let mut n = 0usize;
        // SAFETY: handles are live; the utf8 string only has to outlive the encode call.
        unsafe {
            sys::ghostty_key_encoder_set_from_terminal(self.enc, term.raw());
            sys::ghostty_key_event_set_action(
                self.ev,
                if e.press {
                    sys::GhosttyKeyAction_GHOSTTY_KEY_ACTION_PRESS
                } else {
                    sys::GhosttyKeyAction_GHOSTTY_KEY_ACTION_RELEASE
                },
            );
            sys::ghostty_key_event_set_key(self.ev, e.key);
            sys::ghostty_key_event_set_mods(self.ev, e.mods as _);
            sys::ghostty_key_event_set_consumed_mods(self.ev, e.consumed_mods as _);
            sys::ghostty_key_event_set_composing(self.ev, false);
            sys::ghostty_key_event_set_unshifted_codepoint(self.ev, e.unshifted);
            sys::ghostty_key_event_set_utf8(self.ev, sys::GhosttyString { ptr: e.utf8.as_ptr(), len: e.utf8.len() });
            if sys::ghostty_key_encoder_encode(self.enc, self.ev, buf.as_mut_ptr(), buf.len(), &mut n) != 0 {
                return Vec::new();
            }
        }
        buf[..n.min(buf.len())].to_vec()
    }
}

impl Drop for KeyEncoder {
    fn drop(&mut self) {
        // SAFETY: created by the matching *_new calls, freed once.
        unsafe {
            sys::ghostty_key_event_free(self.ev);
            sys::ghostty_key_encoder_free(self.enc);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseAction {
    Press,
    Release,
    Motion,
}

/// Mouse buttons as the library numbers them (wheel up/down are buttons 4 and 5).
pub const BUTTON_LEFT: u32 = sys::GhosttyMouseButton_GHOSTTY_MOUSE_BUTTON_LEFT;
pub const BUTTON_RIGHT: u32 = sys::GhosttyMouseButton_GHOSTTY_MOUSE_BUTTON_RIGHT;
pub const BUTTON_MIDDLE: u32 = sys::GhosttyMouseButton_GHOSTTY_MOUSE_BUTTON_MIDDLE;
pub const BUTTON_WHEEL_UP: u32 = sys::GhosttyMouseButton_GHOSTTY_MOUSE_BUTTON_FOUR;
pub const BUTTON_WHEEL_DOWN: u32 = sys::GhosttyMouseButton_GHOSTTY_MOUSE_BUTTON_FIVE;

pub struct MouseEvent {
    pub action: MouseAction,
    /// `None` for motion without a button.
    pub button: Option<u32>,
    pub mods: u32,
    /// Position in pixels inside the terminal area.
    pub x: f32,
    pub y: f32,
}

/// Size of the drawing area and its cells, so pixel positions become cell coordinates.
#[derive(Clone, Copy, Debug)]
pub struct MouseGeometry {
    pub width: u32,
    pub height: u32,
    pub cell_w: u32,
    pub cell_h: u32,
}

pub struct MouseEncoder {
    enc: sys::GhosttyMouseEncoder,
    ev: sys::GhosttyMouseEvent,
}

impl MouseEncoder {
    pub fn new() -> Result<Self, i32> {
        let (mut enc, mut ev) = (null_mut(), null_mut());
        // SAFETY: valid out pointers, default allocator.
        unsafe {
            let r = sys::ghostty_mouse_encoder_new(null(), &mut enc);
            if r != 0 {
                return Err(r);
            }
            let r = sys::ghostty_mouse_event_new(null(), &mut ev);
            if r != 0 {
                sys::ghostty_mouse_encoder_free(enc);
                return Err(r);
            }
            let yes = true;
            sys::ghostty_mouse_encoder_set(
                enc,
                sys::GhosttyMouseEncoderOption_GHOSTTY_MOUSE_ENCODER_OPT_TRACK_LAST_CELL,
                &yes as *const _ as *const _,
            );
        }
        Ok(Self { enc, ev })
    }

    /// Bytes for the program, empty when the current tracking mode does not report this event
    /// (including motion inside the same cell).
    pub fn encode(&mut self, term: &Terminal, g: MouseGeometry, any_pressed: bool, e: &MouseEvent) -> Vec<u8> {
        let size = sys::GhosttyMouseEncoderSize {
            size: std::mem::size_of::<sys::GhosttyMouseEncoderSize>(),
            screen_width: g.width,
            screen_height: g.height,
            cell_width: g.cell_w.max(1),
            cell_height: g.cell_h.max(1),
            padding_top: 0,
            padding_bottom: 0,
            padding_right: 0,
            padding_left: 0,
        };
        let mut buf = [0u8; 64];
        let mut n = 0usize;
        // SAFETY: handles live, option values valid for the calls.
        unsafe {
            sys::ghostty_mouse_encoder_set_from_terminal(self.enc, term.raw());
            sys::ghostty_mouse_encoder_set(
                self.enc,
                sys::GhosttyMouseEncoderOption_GHOSTTY_MOUSE_ENCODER_OPT_SIZE,
                &size as *const _ as *const _,
            );
            sys::ghostty_mouse_encoder_set(
                self.enc,
                sys::GhosttyMouseEncoderOption_GHOSTTY_MOUSE_ENCODER_OPT_ANY_BUTTON_PRESSED,
                &any_pressed as *const _ as *const _,
            );
            sys::ghostty_mouse_event_set_action(
                self.ev,
                match e.action {
                    MouseAction::Press => sys::GhosttyMouseAction_GHOSTTY_MOUSE_ACTION_PRESS,
                    MouseAction::Release => sys::GhosttyMouseAction_GHOSTTY_MOUSE_ACTION_RELEASE,
                    MouseAction::Motion => sys::GhosttyMouseAction_GHOSTTY_MOUSE_ACTION_MOTION,
                },
            );
            match e.button {
                Some(b) => sys::ghostty_mouse_event_set_button(self.ev, b),
                None => sys::ghostty_mouse_event_clear_button(self.ev),
            };
            sys::ghostty_mouse_event_set_mods(self.ev, e.mods as _);
            sys::ghostty_mouse_event_set_position(self.ev, sys::GhosttyMousePosition { x: e.x, y: e.y });
            if sys::ghostty_mouse_encoder_encode(self.enc, self.ev, buf.as_mut_ptr(), buf.len(), &mut n) != 0 {
                return Vec::new();
            }
        }
        buf[..n.min(buf.len())].to_vec()
    }
}

impl Drop for MouseEncoder {
    fn drop(&mut self) {
        // SAFETY: created by the matching *_new calls, freed once.
        unsafe {
            sys::ghostty_mouse_event_free(self.ev);
            sys::ghostty_mouse_encoder_free(self.enc);
        }
    }
}

impl Terminal {
    /// A program asked for mouse events (any tracking mode).
    pub fn mouse_tracking(&self) -> bool {
        let mut on = false;
        // SAFETY: bool out value as documented.
        unsafe {
            sys::ghostty_terminal_get(
                self.raw(),
                sys::GhosttyTerminalData_GHOSTTY_TERMINAL_DATA_MOUSE_TRACKING,
                &mut on as *mut _ as *mut _,
            );
        }
        on
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(key: u32, utf8: &str, mods: u32) -> KeyEvent<'_> {
        KeyEvent {
            key,
            mods,
            utf8,
            unshifted: utf8.chars().next().map_or(0, |c| c as u32),
            press: true,
            consumed_mods: 0,
        }
    }

    #[test]
    fn mouse_reports_only_when_asked() {
        let mut t = Terminal::new(80, 24).unwrap();
        let mut m = MouseEncoder::new().unwrap();
        let g = MouseGeometry { width: 800, height: 480, cell_w: 10, cell_h: 20 };
        let press = MouseEvent { action: MouseAction::Press, button: Some(BUTTON_LEFT), mods: 0, x: 25.0, y: 45.0 };
        assert!(!t.mouse_tracking());
        assert!(m.encode(&t, g, false, &press).is_empty());
        t.write(b"\x1b[?1000h\x1b[?1006h"); // normal tracking, SGR format
        assert!(t.mouse_tracking());
        assert_eq!(m.encode(&t, g, false, &press), b"\x1b[<0;3;3M");
        let rel = MouseEvent { action: MouseAction::Release, ..press };
        assert_eq!(m.encode(&t, g, true, &rel), b"\x1b[<0;3;3m");
    }

    #[test]
    fn text_arrows_and_application_mode() {
        let mut t = Terminal::new(20, 3).unwrap();
        let mut k = KeyEncoder::new().unwrap();
        let a = key_from_evdev(30).unwrap();
        assert_eq!(k.encode(&t, &press(a, "a", 0)), b"a");
        assert_eq!(k.encode(&t, &press(a, "a", MOD_CTRL)), b"\x01");
        let up = key_from_evdev(103).unwrap();
        assert_eq!(k.encode(&t, &press(up, "", 0)), b"\x1b[A");
        t.write(b"\x1b[?1h"); // application cursor keys
        assert_eq!(k.encode(&t, &press(up, "", 0)), b"\x1bOA");
        assert_eq!(k.encode(&t, &press(key_from_evdev(28).unwrap(), "", 0)), b"\r");
        assert_eq!(k.encode(&t, &press(a, "a", MOD_ALT)), b"\x1ba");
    }
}
