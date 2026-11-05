use std::sync::atomic::{AtomicU32, Ordering};

pub const BTN_UP: u32 = 1 << 0;
pub const BTN_DOWN: u32 = 1 << 1;
pub const BTN_LEFT: u32 = 1 << 2;
pub const BTN_RIGHT: u32 = 1 << 3;
pub const BTN_Z: u32 = 1 << 4;
pub const BTN_X: u32 = 1 << 5;
pub const BTN_C: u32 = 1 << 6;
pub const BTN_A: u32 = 1 << 7;
pub const BTN_S: u32 = 1 << 8;
pub const BTN_D: u32 = 1 << 9;
pub const BTN_ENTER: u32 = 1 << 10;
pub const BTN_ESCAPE: u32 = 1 << 11;
pub const BTN_SPACE: u32 = 1 << 12;
pub const BTN_F1: u32 = 1 << 13;
pub const BTN_F2: u32 = 1 << 14;
pub const BTN_F3: u32 = 1 << 15;
pub const BTN_F4: u32 = 1 << 16;
pub const BTN_F5: u32 = 1 << 17;
pub const BTN_F6: u32 = 1 << 18;
pub const BTN_F7: u32 = 1 << 19;
pub const BTN_F8: u32 = 1 << 20;
pub const BTN_F9: u32 = 1 << 21;
pub const BTN_F10: u32 = 1 << 22;
pub const BTN_F11: u32 = 1 << 23;
pub const BTN_F12: u32 = 1 << 24;

/// Immutable snapshot of input state comparing the current frame against the previous frame.
/// Enables GameMaker 8 style input handling:
/// - `pressed`: key was just pressed this frame (`keyboard_check_pressed`)
/// - `held`: key was already down and remains down (`keyboard_check` for continuous holds)
/// - `released`: key was released this frame (`keyboard_check_released`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InputSnapshot {
    pub current: u32,
    pub previous: u32,
    pub pressed: u32,
    pub held: u32,
    pub released: u32,
}

impl InputSnapshot {
    /// Returns true if the given button was pressed this frame (transitioned from 0 to 1).
    #[inline]
    pub fn is_pressed(&self, bit: u32) -> bool {
        (self.pressed & bit) != 0
    }

    /// Returns true if the button was down in both the previous and current frame.
    #[inline]
    pub fn is_held(&self, bit: u32) -> bool {
        (self.held & bit) != 0
    }

    /// Returns true if the button was released this frame (transitioned from 1 to 0).
    #[inline]
    pub fn is_released(&self, bit: u32) -> bool {
        (self.released & bit) != 0
    }

    /// Returns true if the button is currently down in this frame (`keyboard_check`).
    #[inline]
    pub fn is_down(&self, bit: u32) -> bool {
        (self.current & bit) != 0
    }
}

/// Thread-safe, lock-free input state using `AtomicU32`.
#[derive(Debug, Default)]
pub struct AtomicInputState {
    current: AtomicU32,
    previous: AtomicU32,
}

impl AtomicInputState {
    /// Creates a new empty `AtomicInputState`.
    pub fn new() -> Self {
        Self {
            current: AtomicU32::new(0),
            previous: AtomicU32::new(0),
        }
    }

    /// Overwrites the full input bitmask.
    #[inline]
    pub fn set_mask(&self, mask: u32) {
        self.current.store(mask, Ordering::Relaxed);
    }

    /// Sets or clears a specific button bit.
    #[inline]
    pub fn set_button(&self, bit: u32, pressed: bool) {
        if pressed {
            self.current.fetch_or(bit, Ordering::Relaxed);
        } else {
            self.current.fetch_and(!bit, Ordering::Relaxed);
        }
    }

    /// Gets the current active bitmask.
    #[inline]
    pub fn get_mask(&self) -> u32 {
        self.current.load(Ordering::Relaxed)
    }

    /// Checks if a button bit is currently set.
    #[inline]
    pub fn is_pressed(&self, bit: u32) -> bool {
        (self.get_mask() & bit) != 0
    }

    /// Advances the frame state and takes a snapshot of `current`, `previous`, `pressed`, `held`, and `released`.
    pub fn snapshot(&self) -> InputSnapshot {
        let curr = self.current.load(Ordering::Relaxed);
        let prev = self.previous.swap(curr, Ordering::Relaxed);

        InputSnapshot {
            current: curr,
            previous: prev,
            pressed: curr & !prev,
            held: curr & prev,
            released: !curr & prev,
        }
    }

    /// Resets all inputs to unpressed.
    pub fn reset(&self) {
        self.current.store(0, Ordering::Relaxed);
        self.previous.store(0, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atomic_input_transitions() {
        let input = AtomicInputState::new();
        assert_eq!(input.get_mask(), 0);

        // Frame 1: Press Z and UP
        input.set_button(BTN_Z, true);
        input.set_button(BTN_UP, true);
        assert!(input.is_pressed(BTN_Z));
        assert!(input.is_pressed(BTN_UP));
        assert!(!input.is_pressed(BTN_X));

        let snap1 = input.snapshot();
        assert!(snap1.is_pressed(BTN_Z));
        assert!(snap1.is_pressed(BTN_UP));
        assert!(!snap1.is_held(BTN_Z));
        assert!(!snap1.is_released(BTN_Z));
        assert!(snap1.is_down(BTN_Z));
        assert!(snap1.is_down(BTN_UP));

        // Frame 2: Hold Z, release UP, press X
        input.set_button(BTN_UP, false);
        input.set_button(BTN_X, true);

        let snap2 = input.snapshot();
        // Z was held
        assert!(!snap2.is_pressed(BTN_Z));
        assert!(snap2.is_held(BTN_Z));
        assert!(!snap2.is_released(BTN_Z));
        assert!(snap2.is_down(BTN_Z));

        // UP was released
        assert!(!snap2.is_pressed(BTN_UP));
        assert!(!snap2.is_held(BTN_UP));
        assert!(snap2.is_released(BTN_UP));
        assert!(!snap2.is_down(BTN_UP));

        // X was pressed
        assert!(snap2.is_pressed(BTN_X));
        assert!(!snap2.is_held(BTN_X));
        assert!(!snap2.is_released(BTN_X));
        assert!(snap2.is_down(BTN_X));

        // Frame 3: No changes
        let snap3 = input.snapshot();
        assert!(!snap3.is_pressed(BTN_Z));
        assert!(snap3.is_held(BTN_Z));
        assert!(!snap3.is_pressed(BTN_X));
        assert!(snap3.is_held(BTN_X));
        assert!(!snap3.is_released(BTN_UP));

        // Frame 4: Release all via set_mask
        input.set_mask(0);
        let snap4 = input.snapshot();
        assert!(snap4.is_released(BTN_Z));
        assert!(snap4.is_released(BTN_X));
        assert_eq!(snap4.current, 0);

        // Frame 5: Idle
        let snap5 = input.snapshot();
        assert_eq!(snap5.current, 0);
        assert_eq!(snap5.pressed, 0);
        assert_eq!(snap5.held, 0);
        assert_eq!(snap5.released, 0);
    }

    #[test]
    fn test_f_keys_and_buttons_constants() {
        assert_eq!(BTN_UP, 1);
        assert_eq!(BTN_DOWN, 2);
        assert_eq!(BTN_LEFT, 4);
        assert_eq!(BTN_RIGHT, 8);
        assert_eq!(BTN_F1, 1 << 13);
        assert_eq!(BTN_F12, 1 << 24);
    }
}
