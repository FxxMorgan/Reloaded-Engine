//! Android EGL platform implementation for gm8emulator OpenGL ES 3.0 renderer.

use crate::ramen::{connection::Connection, window::Window};
use std::{
    ffi::{c_char, c_void},
    ptr,
};

#[allow(dead_code)]
#[link(name = "EGL")]
extern "C" {
    fn eglGetDisplay(display_id: *mut c_void) -> *mut c_void;
    fn eglGetCurrentDisplay() -> *mut c_void;
    fn eglGetCurrentContext() -> *mut c_void;
    fn eglGetCurrentSurface(readdraw: i32) -> *mut c_void;
    fn eglGetProcAddress(procname: *const c_char) -> *mut c_void;
    fn eglSwapBuffers(dpy: *mut c_void, surface: *mut c_void) -> u32;
    fn eglSwapInterval(dpy: *mut c_void, interval: i32) -> u32;
    fn eglQuerySurface(dpy: *mut c_void, surface: *mut c_void, attribute: i32, value: *mut i32) -> u32;
}

const EGL_DRAW: i32 = 0x3059;
const EGL_WIDTH: i32 = 0x3057;
const EGL_HEIGHT: i32 = 0x3056;

pub struct PlatformImpl {
    display: *mut c_void,
    surface: *mut c_void,
    window_size: (u16, u16),
}

impl PlatformImpl {
    pub unsafe fn new(_connection: &Connection, window: &Window) -> Result<Self, String> {
        let mut display = eglGetCurrentDisplay();
        if display.is_null() {
            display = eglGetDisplay(ptr::null_mut());
        }
        let surface = eglGetCurrentSurface(EGL_DRAW);
        Ok(PlatformImpl {
            display,
            surface,
            window_size: window.size,
        })
    }

    pub unsafe fn get_function_loader() -> Result<Box<dyn FnMut(&'static str) -> *const c_void>, String> {
        let mut libgles = libc::dlopen(b"libGLESv3.so\0".as_ptr().cast(), libc::RTLD_GLOBAL | libc::RTLD_NOW);
        if libgles.is_null() {
            libgles = libc::dlopen(b"libGLESv2.so\0".as_ptr().cast(), libc::RTLD_GLOBAL | libc::RTLD_NOW);
        }
        let libegl = libc::dlopen(b"libEGL.so\0".as_ptr().cast(), libc::RTLD_GLOBAL | libc::RTLD_NOW);
        let mut buf = Vec::new();
        Ok(Box::new(move |name: &'static str| unsafe {
            buf.clear();
            buf.extend_from_slice(name.as_bytes());
            buf.push(0);
            let c_str = buf.as_ptr().cast();
            let mut p = eglGetProcAddress(c_str);
            if p.is_null() && !libgles.is_null() {
                p = libc::dlsym(libgles, c_str);
            }
            if p.is_null() && !libegl.is_null() {
                p = libc::dlsym(libegl, c_str);
            }
            p as *const c_void
        }))
    }

    pub unsafe fn clean_function_loader() {}

    pub unsafe fn swap_buffers(&self) {
        let cur_dpy = eglGetCurrentDisplay();
        let dpy = if !cur_dpy.is_null() { cur_dpy } else { self.display };
        let cur_surf = eglGetCurrentSurface(EGL_DRAW);
        let surf = if !cur_surf.is_null() { cur_surf } else { self.surface };
        if !dpy.is_null() && !surf.is_null() {
            eglSwapBuffers(dpy, surf);
        }
    }

    pub unsafe fn set_swap_interval(&self, n: u32) -> bool {
        let cur_dpy = eglGetCurrentDisplay();
        let dpy = if !cur_dpy.is_null() { cur_dpy } else { self.display };
        if !dpy.is_null() {
            eglSwapInterval(dpy, n as i32) != 0
        } else {
            false
        }
    }

    pub unsafe fn get_swap_interval(&self) -> u32 {
        0
    }

    pub unsafe fn wait_vsync(&self) {}

    pub fn current_surface_size(&self) -> Option<(f32, f32)> {
        unsafe {
            let cur_dpy = eglGetCurrentDisplay();
            let dpy = if !cur_dpy.is_null() { cur_dpy } else { self.display };
            let cur_surf = eglGetCurrentSurface(EGL_DRAW);
            let surf = if !cur_surf.is_null() { cur_surf } else { self.surface };
            if !dpy.is_null() && !surf.is_null() {
                let mut w: i32 = 0;
                let mut h: i32 = 0;
                if eglQuerySurface(dpy, surf, EGL_WIDTH, &mut w) != 0
                    && eglQuerySurface(dpy, surf, EGL_HEIGHT, &mut h) != 0
                    && w > 0
                    && h > 0
                {
                    return Some((w as f32, h as f32));
                }
            }
        }
        if self.window_size.0 > 0 && self.window_size.1 > 0 {
            Some((self.window_size.0 as f32, self.window_size.1 as f32))
        } else {
            None
        }
    }
}
