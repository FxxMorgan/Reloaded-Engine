//! Lightweight embedded replacement for `ramen` without X11 or Win32 dependencies.

pub mod platform {
    pub mod linux {
        pub type Display = std::ffi::c_void;
    }
}

pub mod connection {
    pub struct Connection;

    impl Connection {
        pub fn new() -> Result<Self, String> {
            Ok(Self)
        }

        pub fn builder(&self) -> super::window::WindowBuilder {
            super::window::WindowBuilder::new()
        }
    }
}

pub mod window {
    use super::event::Event;

    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub enum Cursor {
        Arrow,
        Cross,
        IBeam,
        ResizeNESW,
        ResizeNS,
        ResizeNWSE,
        ResizeWE,
        Wait,
        Unavailable,
        Progress,
        Help,
        Hand,
        ResizeAll,
        Blank,
    }

    #[derive(Copy, Clone, Debug, Default)]
    pub struct Controls;

    impl Controls {
        pub const fn new() -> Self {
            Self
        }

        pub const fn minimise(self, _: bool) -> Self {
            self
        }

        pub const fn maximise(self, _: bool) -> Self {
            self
        }
    }

    #[derive(Debug, Default)]
    pub struct WindowBuilder {
        pub size: (u16, u16),
        pub title: String,
        pub visible: bool,
        pub borderless: bool,
        pub resizable: bool,
        pub controls: Option<Controls>,
    }

    impl WindowBuilder {
        pub fn new() -> Self {
            Self {
                size: (800, 600),
                title: String::new(),
                visible: true,
                borderless: false,
                resizable: true,
                controls: None,
            }
        }

        pub fn class_name(self, _: &str) -> Self {
            self
        }

        pub fn visible(mut self, visible: bool) -> Self {
            self.visible = visible;
            self
        }

        pub fn size(mut self, size: (u16, u16)) -> Self {
            self.size = size;
            self
        }

        pub fn borderless(mut self, borderless: bool) -> Self {
            self.borderless = borderless;
            self
        }

        pub fn title(mut self, title: impl Into<String>) -> Self {
            self.title = title.into();
            self
        }

        pub fn resizable(mut self, resizable: bool) -> Self {
            self.resizable = resizable;
            self
        }

        pub fn controls(mut self, controls: Option<Controls>) -> Self {
            self.controls = controls;
            self
        }

        pub fn depth(self, _: i32) -> Self {
            self
        }

        pub fn visual(self, _: *mut std::ffi::c_void) -> Self {
            self
        }

        pub fn build(self) -> Result<Window, String> {
            Ok(Window {
                events: Vec::new(),
                size: self.size,
                title: self.title,
                visible: self.visible,
                borderless: self.borderless,
                resizable: self.resizable,
            })
        }
    }

    pub struct Window {
        pub events: Vec<Event>,
        pub size: (u16, u16),
        pub title: String,
        pub visible: bool,
        pub borderless: bool,
        pub resizable: bool,
    }

    impl Window {
        pub fn set_visible(&mut self, visible: bool) {
            self.visible = visible;
        }

        pub fn set_size(&mut self, size: (u16, u16)) {
            self.size = size;
        }

        pub fn set_title(&mut self, title: &str) {
            self.title = title.to_string();
        }

        pub fn set_borderless(&mut self, borderless: bool) {
            self.borderless = borderless;
        }

        pub fn set_resizable(&mut self, resizable: bool) {
            self.resizable = resizable;
        }

        pub fn set_cursor(&mut self, _: Cursor) {}

        pub fn set_maximised(&mut self, _: bool) {}

        pub fn poll_events(&mut self) {}

        pub fn events(&self) -> &[Event] {
            &self.events
        }

        pub fn xid(&self) -> u32 {
            0
        }

        pub fn hwnd(&self) -> usize {
            0
        }
    }
}

pub mod event {
    use super::input::{Key, MouseButton};

    #[derive(Copy, Clone, Debug, PartialEq)]
    pub enum Event {
        KeyboardDown(Key),
        KeyboardUp(Key),
        MouseMove((i16, i16)),
        MouseDown(MouseButton),
        MouseUp(MouseButton),
        ScrollUp,
        ScrollDown,
        Resize((u16, u16)),
        CloseRequest,
        FocusLost,
        Minimise,
    }
}

pub mod input {
    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub enum Key {
        A,
        Alpha0,
        Alpha1,
        Alpha2,
        Alpha3,
        Alpha4,
        Alpha5,
        Alpha6,
        Alpha7,
        Alpha8,
        Alpha9,
        Applications,
        Attn,
        B,
        Backspace,
        BrowserBack,
        BrowserFavourites,
        BrowserForward,
        BrowserHome,
        BrowserRefresh,
        BrowserSearch,
        BrowserStop,
        C,
        CapsLock,
        Clear,
        Comma,
        CrSel,
        D,
        Delete,
        DownArrow,
        E,
        End,
        Enter,
        EraseEof,
        Escape,
        Execute,
        ExSel,
        F,
        F1,
        F10,
        F11,
        F12,
        F13,
        F14,
        F15,
        F16,
        F17,
        F18,
        F19,
        F2,
        F20,
        F21,
        F22,
        F23,
        F24,
        F3,
        F4,
        F5,
        F6,
        F7,
        F8,
        F9,
        G,
        H,
        Help,
        Home,
        I,
        ImeAccept,
        ImeConvert,
        ImeFinal,
        ImeHanjaOrKanji,
        ImeJunja,
        ImeKanaOrHangul,
        ImeModeChangeRequest,
        ImeNonConvert,
        ImeOff,
        ImeOn,
        ImeProcess,
        Insert,
        J,
        K,
        Keypad0,
        Keypad1,
        Keypad2,
        Keypad3,
        Keypad4,
        Keypad5,
        Keypad6,
        Keypad7,
        Keypad8,
        Keypad9,
        KeypadAdd,
        KeypadDecimal,
        KeypadDivide,
        KeypadMultiply,
        KeypadSeparator,
        KeypadSubtract,
        L,
        LaunchApp1,
        LaunchApp2,
        LaunchApplication1,
        LaunchApplication2,
        LaunchMail,
        LaunchMedia,
        LaunchMediaSelect,
        LeftAlt,
        LeftArrow,
        LeftControl,
        LeftCtrl,
        LeftShift,
        LeftSuper,
        M,
        MediaNextTrack,
        MediaPlay,
        MediaPlayPause,
        MediaPreviousTrack,
        MediaStop,
        MediaVolumeDown,
        MediaVolumeMute,
        MediaVolumeUp,
        Menu,
        Minus,
        N,
        NumLock,
        O,
        Oem102,
        OemClear,
        P,
        Pa1,
        PageDown,
        PageUp,
        Pause,
        Period,
        Play,
        Plus,
        Print,
        PrintScreen,
        Q,
        R,
        Return,
        RightAlt,
        RightArrow,
        RightControl,
        RightCtrl,
        RightShift,
        RightSuper,
        S,
        ScrollLock,
        Select,
        Slash,
        Sleep,
        Space,
        T,
        Tab,
        U,
        UpArrow,
        V,
        W,
        X,
        Y,
        Z,
        Zoom,
    }

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub enum MouseButton {
        Left,
        Right,
        Middle,
    }
}
