use reloaded_core::renderer::Gles3BatchRenderer;

#[cfg(target_os = "android")]
mod imp {
    use super::*;
    use reloaded_core::renderer::{
        SpriteVertex, Std140UniformBlock, FRAGMENT_SHADER_ES30, VERTEX_SHADER_ES30,
    };
    use std::ffi::{c_char, c_void, CString};

    #[link(name = "android")]
    #[link(name = "log")]
    #[link(name = "EGL")]
    #[link(name = "GLESv3")]
    extern "C" {
        pub fn __android_log_write(
            prio: i32,
            tag: *const c_char,
            text: *const c_char,
        ) -> i32;

        pub fn ANativeWindow_fromSurface(
            env: *mut jni::sys::JNIEnv,
            surface: jni::sys::jobject,
        ) -> *mut c_void;
        pub fn ANativeWindow_release(window: *mut c_void);

        pub fn eglGetDisplay(display_id: *mut c_void) -> *mut c_void;
        pub fn eglInitialize(dpy: *mut c_void, major: *mut i32, minor: *mut i32) -> u32;
        pub fn eglChooseConfig(
            dpy: *mut c_void,
            attrib_list: *const i32,
            configs: *mut *mut c_void,
            config_size: i32,
            num_config: *mut i32,
        ) -> u32;
        pub fn eglCreateWindowSurface(
            dpy: *mut c_void,
            config: *mut c_void,
            win: *mut c_void,
            attrib_list: *const i32,
        ) -> *mut c_void;
        pub fn eglCreateContext(
            dpy: *mut c_void,
            config: *mut c_void,
            share_context: *mut c_void,
            attrib_list: *const i32,
        ) -> *mut c_void;
        pub fn eglMakeCurrent(
            dpy: *mut c_void,
            draw: *mut c_void,
            read: *mut c_void,
            ctx: *mut c_void,
        ) -> u32;
        pub fn eglSwapBuffers(dpy: *mut c_void, surface: *mut c_void) -> u32;
        pub fn eglDestroySurface(dpy: *mut c_void, surface: *mut c_void) -> u32;
        pub fn eglDestroyContext(dpy: *mut c_void, ctx: *mut c_void) -> u32;
        pub fn eglTerminate(dpy: *mut c_void) -> u32;
        pub fn eglGetError() -> i32;

        pub fn glViewport(x: i32, y: i32, width: i32, height: i32);
        pub fn glClearColor(red: f32, green: f32, blue: f32, alpha: f32);
        pub fn glClear(mask: u32);
        pub fn glEnable(cap: u32);
        pub fn glDisable(cap: u32);
        pub fn glBlendFunc(sfactor: u32, dfactor: u32);
        pub fn glCreateShader(shader_type: u32) -> u32;
        pub fn glShaderSource(
            shader: u32,
            count: i32,
            string: *const *const c_char,
            length: *const i32,
        );
        pub fn glCompileShader(shader: u32);
        pub fn glGetShaderiv(shader: u32, pname: u32, params: *mut i32);
        pub fn glCreateProgram() -> u32;
        pub fn glAttachShader(program: u32, shader: u32);
        pub fn glLinkProgram(program: u32);
        pub fn glGetProgramiv(program: u32, pname: u32, params: *mut i32);
        pub fn glUseProgram(program: u32);
        pub fn glGenVertexArrays(n: i32, arrays: *mut u32);
        pub fn glBindVertexArray(array: u32);
        pub fn glGenBuffers(n: i32, buffers: *mut u32);
        pub fn glBindBuffer(target: u32, buffer: u32);
        pub fn glBufferData(target: u32, size: isize, data: *const c_void, usage: u32);
        pub fn glEnableVertexAttribArray(index: u32);
        pub fn glVertexAttribPointer(
            index: u32,
            size: i32,
            type_: u32,
            normalized: u8,
            stride: i32,
            pointer: *const c_void,
        );
        pub fn glGetUniformLocation(program: u32, name: *const c_char) -> i32;
        pub fn glUniform1i(location: i32, v0: i32);
        pub fn glGetUniformBlockIndex(program: u32, uniformBlockName: *const c_char) -> u32;
        pub fn glUniformBlockBinding(
            program: u32,
            uniformBlockIndex: u32,
            uniformBlockBinding: u32,
        );
        pub fn glBindBufferBase(target: u32, index: u32, buffer: u32);
        pub fn glActiveTexture(texture: u32);
        pub fn glGenTextures(n: i32, textures: *mut u32);
        pub fn glBindTexture(target: u32, texture: u32);
        pub fn glTexImage2D(
            target: u32,
            level: i32,
            internalformat: i32,
            width: i32,
            height: i32,
            border: i32,
            format: u32,
            type_: u32,
            data: *const c_void,
        );
        pub fn glTexParameteri(target: u32, pname: u32, param: i32);
        pub fn glDrawArrays(mode: u32, first: i32, count: i32);
        pub fn glDeleteBuffers(n: i32, buffers: *const u32);
        pub fn glDeleteVertexArrays(n: i32, arrays: *const u32);
        pub fn glDeleteProgram(program: u32);
        pub fn glDeleteTextures(n: i32, textures: *const u32);
    }

    pub fn alog(msg: &str) {
        if let Ok(c_msg) = CString::new(msg) {
            unsafe {
                __android_log_write(4, b"ReloadedEGL\0".as_ptr() as *const c_char, c_msg.as_ptr());
            }
        }
    }

    const EGL_DEFAULT_DISPLAY: *mut c_void = std::ptr::null_mut();
    const EGL_NO_CONTEXT: *mut c_void = std::ptr::null_mut();
    const EGL_NO_SURFACE: *mut c_void = std::ptr::null_mut();
    const EGL_NO_DISPLAY: *mut c_void = std::ptr::null_mut();
    const EGL_SURFACE_TYPE: i32 = 0x3033;
    const EGL_WINDOW_BIT: i32 = 0x0004;
    const EGL_RENDERABLE_TYPE: i32 = 0x3040;
    const EGL_OPENGL_ES2_BIT: i32 = 0x0004;
    const EGL_OPENGL_ES3_BIT: i32 = 0x0040;
    const EGL_RED_SIZE: i32 = 0x3024;
    const EGL_GREEN_SIZE: i32 = 0x3023;
    const EGL_BLUE_SIZE: i32 = 0x3022;
    const EGL_ALPHA_SIZE: i32 = 0x3021;
    const EGL_DEPTH_SIZE: i32 = 0x3025;
    const EGL_NONE: i32 = 0x3038;
    const EGL_CONTEXT_CLIENT_VERSION: i32 = 0x3098;

    const GL_COLOR_BUFFER_BIT: u32 = 0x00004000;
    const GL_DEPTH_BUFFER_BIT: u32 = 0x00000100;
    const GL_TRIANGLES: u32 = 0x0004;
    const GL_FLOAT: u32 = 0x1406;
    const GL_FALSE: u8 = 0;
    const GL_ARRAY_BUFFER: u32 = 0x8892;
    const GL_UNIFORM_BUFFER: u32 = 0x8A11;
    const GL_DYNAMIC_DRAW: u32 = 0x88E8;
    const GL_FRAGMENT_SHADER: u32 = 0x8B30;
    const GL_VERTEX_SHADER: u32 = 0x8B31;
    const GL_COMPILE_STATUS: u32 = 0x8B81;
    const GL_LINK_STATUS: u32 = 0x8B82;
    const GL_BLEND: u32 = 0x0BE2;
    const GL_SRC_ALPHA: u32 = 0x0302;
    const GL_ONE_MINUS_SRC_ALPHA: u32 = 0x0303;
    const GL_TEXTURE_2D: u32 = 0x0DE1;
    const GL_TEXTURE0: u32 = 0x84C0;
    const GL_DEPTH_TEST: u32 = 0x0B71;
    const GL_CULL_FACE: u32 = 0x0B44;
    const GL_RGBA: u32 = 0x1908;
    const GL_UNSIGNED_BYTE: u32 = 0x1401;
    const GL_TEXTURE_MIN_FILTER: u32 = 0x2801;
    const GL_TEXTURE_MAG_FILTER: u32 = 0x2800;
    const GL_NEAREST: i32 = 0x2600;
    const GL_TEXTURE_WRAP_S: u32 = 0x2802;
    const GL_TEXTURE_WRAP_T: u32 = 0x2803;
    const GL_CLAMP_TO_EDGE: i32 = 0x812F;

    pub struct HardwareEglRenderer {
        display: *mut c_void,
        surface: *mut c_void,
        context: *mut c_void,
        config: *mut c_void,
        native_window: *mut c_void,
        program: u32,
        vao: u32,
        vbo: u32,
        ubo: u32,
        default_texture: u32,
        entity_texture: u32,
        width: u32,
        height: u32,
        initialized: bool,
        frame_counter: u64,
    }

    unsafe impl Send for HardwareEglRenderer {}
    unsafe impl Sync for HardwareEglRenderer {}

    impl Default for HardwareEglRenderer {
        fn default() -> Self {
            Self::new()
        }
    }

    impl HardwareEglRenderer {
        pub fn new() -> Self {
            Self {
                display: EGL_NO_DISPLAY,
                surface: EGL_NO_SURFACE,
                context: EGL_NO_CONTEXT,
                config: std::ptr::null_mut(),
                native_window: std::ptr::null_mut(),
                program: 0,
                vao: 0,
                vbo: 0,
                ubo: 0,
                default_texture: 0,
                entity_texture: 0,
                width: 0,
                height: 0,
                initialized: false,
                frame_counter: 0,
            }
        }

        pub fn attach_surface(
            &mut self,
            raw_env: *mut jni::sys::JNIEnv,
            surface_obj: jni::sys::jobject,
            width: u32,
            height: u32,
        ) -> bool {
            alog(&format!("attach_surface: {}x{}", width, height));

            if self.surface != EGL_NO_SURFACE || !self.native_window.is_null() {
                self.destroy_surface();
            }

            unsafe {
                let win = ANativeWindow_fromSurface(raw_env, surface_obj);
                if win.is_null() {
                    alog("ERROR: ANativeWindow_fromSurface returned NULL");
                    return false;
                }
                self.native_window = win;

                // 1. Initialize EGL Display if not already initialized
                if self.display == EGL_NO_DISPLAY {
                    let dpy = eglGetDisplay(EGL_DEFAULT_DISPLAY);
                    if dpy == EGL_NO_DISPLAY {
                        alog("ERROR: eglGetDisplay failed");
                        return false;
                    }
                    let mut major = 0;
                    let mut minor = 0;
                    if eglInitialize(dpy, &mut major, &mut minor) == 0 {
                        alog("ERROR: eglInitialize failed");
                        return false;
                    }
                    alog(&format!("EGL initialized {}.{}", major, minor));
                    self.display = dpy;
                }

                // 2. Choose Config
                if self.config.is_null() {
                    let attribs = [
                        EGL_SURFACE_TYPE,
                        EGL_WINDOW_BIT,
                        EGL_RENDERABLE_TYPE,
                        EGL_OPENGL_ES3_BIT | EGL_OPENGL_ES2_BIT,
                        EGL_RED_SIZE,
                        8,
                        EGL_GREEN_SIZE,
                        8,
                        EGL_BLUE_SIZE,
                        8,
                        EGL_ALPHA_SIZE,
                        8,
                        EGL_DEPTH_SIZE,
                        16,
                        EGL_NONE,
                    ];
                    let mut cfg: *mut c_void = std::ptr::null_mut();
                    let mut num_configs = 0;
                    if eglChooseConfig(self.display, attribs.as_ptr(), &mut cfg, 1, &mut num_configs) == 0
                        || num_configs < 1
                    {
                        let fallback_attribs = [
                            EGL_SURFACE_TYPE,
                            EGL_WINDOW_BIT,
                            EGL_RENDERABLE_TYPE,
                            EGL_OPENGL_ES2_BIT,
                            EGL_RED_SIZE,
                            8,
                            EGL_GREEN_SIZE,
                            8,
                            EGL_BLUE_SIZE,
                            8,
                            EGL_ALPHA_SIZE,
                            8,
                            EGL_DEPTH_SIZE,
                            16,
                            EGL_NONE,
                        ];
                        if eglChooseConfig(
                            self.display,
                            fallback_attribs.as_ptr(),
                            &mut cfg,
                            1,
                            &mut num_configs,
                        ) == 0
                            || num_configs < 1
                        {
                            alog("ERROR: eglChooseConfig failed");
                            return false;
                        }
                    }
                    self.config = cfg;
                }

                // 3. Create EGL Window Surface
                let surf = eglCreateWindowSurface(
                    self.display,
                    self.config,
                    self.native_window,
                    std::ptr::null(),
                );
                if surf == EGL_NO_SURFACE {
                    let err = eglGetError();
                    alog(&format!("ERROR: eglCreateWindowSurface failed: 0x{:X}", err));
                    return false;
                }
                self.surface = surf;

                // 4. Create fresh EGL Context if not created
                if self.context == EGL_NO_CONTEXT {
                    let es3_attribs = [EGL_CONTEXT_CLIENT_VERSION, 3, EGL_NONE];
                    let mut ctx = eglCreateContext(
                        self.display,
                        self.config,
                        EGL_NO_CONTEXT,
                        es3_attribs.as_ptr(),
                    );
                    if ctx == EGL_NO_CONTEXT {
                        let es2_attribs = [EGL_CONTEXT_CLIENT_VERSION, 2, EGL_NONE];
                        ctx = eglCreateContext(
                            self.display,
                            self.config,
                            EGL_NO_CONTEXT,
                            es2_attribs.as_ptr(),
                        );
                    }
                    if ctx == EGL_NO_CONTEXT {
                        let err = eglGetError();
                        alog(&format!("ERROR: eglCreateContext failed: 0x{:X}", err));
                        return false;
                    }
                    self.context = ctx;
                }

                // 5. Make Current
                if eglMakeCurrent(self.display, self.surface, self.surface, self.context) == 0 {
                    let err = eglGetError();
                    alog(&format!("ERROR: eglMakeCurrent failed: 0x{:X}", err));
                    return false;
                }

                // 6. Setup GL Pipeline
                if !self.initialized {
                    if !self.setup_gl_pipeline() {
                        alog("ERROR: setup_gl_pipeline failed");
                        return false;
                    }
                    self.initialized = true;
                }

                self.resize(width, height);
                glViewport(0, 0, width as i32, height as i32);
                glClearColor(0.06, 0.08, 0.12, 1.0);
                glClear(GL_COLOR_BUFFER_BIT);
                eglSwapBuffers(self.display, self.surface);

                // Release EGL context from the calling (Android UI) thread so the dedicated game thread
                // can safely bind it via make_current() without EGL_BAD_ACCESS.
                eglMakeCurrent(
                    self.display,
                    EGL_NO_SURFACE,
                    EGL_NO_SURFACE,
                    EGL_NO_CONTEXT,
                );

                alog(&format!("attach_surface READY ({}x{}) - context released for game thread", width, height));
                true
            }
        }

        unsafe fn setup_gl_pipeline(&mut self) -> bool {
            glDisable(GL_DEPTH_TEST);
            glDisable(GL_CULL_FACE);
            glEnable(GL_BLEND);
            glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);

            let vert = match Self::compile_shader(GL_VERTEX_SHADER, VERTEX_SHADER_ES30) {
                Ok(s) => s,
                Err(e) => {
                    alog(&format!("ERROR: vertex shader: {e}"));
                    return false;
                }
            };

            let frag = match Self::compile_shader(GL_FRAGMENT_SHADER, FRAGMENT_SHADER_ES30) {
                Ok(s) => s,
                Err(e) => {
                    alog(&format!("ERROR: fragment shader: {e}"));
                    return false;
                }
            };

            let prog = glCreateProgram();
            glAttachShader(prog, vert);
            glAttachShader(prog, frag);
            glLinkProgram(prog);

            let mut link_ok = 0;
            glGetProgramiv(prog, GL_LINK_STATUS, &mut link_ok);
            if link_ok == 0 {
                alog("ERROR: program linking failed");
                return false;
            }
            self.program = prog;
            glUseProgram(self.program);

            // Bind sampler u_texture to unit 0
            let tex_loc = glGetUniformLocation(self.program, b"u_texture\0".as_ptr() as *const c_char);
            if tex_loc >= 0 {
                glUniform1i(tex_loc, 0);
            }

            // VAO & VBO
            let mut vao = 0;
            glGenVertexArrays(1, &mut vao);
            glBindVertexArray(vao);
            self.vao = vao;

            let mut vbo = 0;
            glGenBuffers(1, &mut vbo);
            glBindBuffer(GL_ARRAY_BUFFER, vbo);
            self.vbo = vbo;

            let stride = std::mem::size_of::<SpriteVertex>() as i32;
            glEnableVertexAttribArray(0);
            glVertexAttribPointer(0, 2, GL_FLOAT, GL_FALSE, stride, 0 as *const c_void);
            glEnableVertexAttribArray(1);
            glVertexAttribPointer(1, 2, GL_FLOAT, GL_FALSE, stride, 8 as *const c_void);
            glEnableVertexAttribArray(2);
            glVertexAttribPointer(2, 4, GL_FLOAT, GL_FALSE, stride, 16 as *const c_void);

            // UBO for Std140UniformBlock
            let mut ubo = 0;
            glGenBuffers(1, &mut ubo);
            glBindBuffer(GL_UNIFORM_BUFFER, ubo);
            glBufferData(
                GL_UNIFORM_BUFFER,
                std::mem::size_of::<Std140UniformBlock>() as isize,
                std::ptr::null(),
                GL_DYNAMIC_DRAW,
            );
            self.ubo = ubo;

            let block_name = b"FrameUniforms\0";
            let block_idx = glGetUniformBlockIndex(self.program, block_name.as_ptr() as *const c_char);
            if block_idx != 0xFFFFFFFF {
                glUniformBlockBinding(self.program, block_idx, 0);
            }
            glBindBufferBase(GL_UNIFORM_BUFFER, 0, self.ubo);

            // Generate 16x16 Ground Tile Texture (Atlas 0: subtle grass tufts + border)
            let mut tile_tex = 0;
            glGenTextures(1, &mut tile_tex);
            glBindTexture(GL_TEXTURE_2D, tile_tex);
            let mut tile_pixels = [0xFFu8; 16 * 16 * 4];
            for y in 0..16usize {
                for x in 0..16usize {
                    let idx = (y * 16 + x) * 4;
                    let is_border = x == 0 || y == 0;
                    let is_blade = (x == 4 && (y == 5 || y == 6))
                        || (x == 5 && y == 6)
                        || (x == 11 && (y == 11 || y == 12))
                        || (x == 12 && y == 12);
                    if is_blade {
                        tile_pixels[idx] = 225;
                        tile_pixels[idx + 1] = 242;
                        tile_pixels[idx + 2] = 225;
                        tile_pixels[idx + 3] = 255;
                    } else if is_border {
                        tile_pixels[idx] = 242;
                        tile_pixels[idx + 1] = 247;
                        tile_pixels[idx + 2] = 242;
                        tile_pixels[idx + 3] = 255;
                    }
                }
            }
            glTexImage2D(
                GL_TEXTURE_2D,
                0,
                GL_RGBA as i32,
                16,
                16,
                0,
                GL_RGBA,
                GL_UNSIGNED_BYTE,
                tile_pixels.as_ptr() as *const c_void,
            );
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
            self.default_texture = tile_tex;

            // Generate 16x16 Entity Sprite Texture (Atlas 1: crisp pixel border + eye/badge detail)
            let mut ent_tex = 0;
            glGenTextures(1, &mut ent_tex);
            glBindTexture(GL_TEXTURE_2D, ent_tex);
            let mut ent_pixels = [0xFFu8; 16 * 16 * 4];
            for y in 0..16usize {
                for x in 0..16usize {
                    let idx = (y * 16 + x) * 4;
                    let is_outer_edge = x == 0 || x == 15 || y == 0 || y == 15;
                    let is_eye = (y == 5 || y == 6) && (x == 4 || x == 5 || x == 10 || x == 11);
                    if is_outer_edge || is_eye {
                        ent_pixels[idx] = 45;
                        ent_pixels[idx + 1] = 55;
                        ent_pixels[idx + 2] = 72;
                        ent_pixels[idx + 3] = 255;
                    }
                }
            }
            glTexImage2D(
                GL_TEXTURE_2D,
                0,
                GL_RGBA as i32,
                16,
                16,
                0,
                GL_RGBA,
                GL_UNSIGNED_BYTE,
                ent_pixels.as_ptr() as *const c_void,
            );
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_NEAREST);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_NEAREST);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
            glTexParameteri(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
            self.entity_texture = ent_tex;

            alog("GL pipeline initialized successfully");
            true
        }

        unsafe fn compile_shader(shader_type: u32, source: &str) -> Result<u32, String> {
            let shader = glCreateShader(shader_type);
            let src_c = CString::new(source).map_err(|e| e.to_string())?;
            let ptrs = [src_c.as_ptr()];
            glShaderSource(shader, 1, ptrs.as_ptr(), std::ptr::null());
            glCompileShader(shader);

            let mut status = 0;
            glGetShaderiv(shader, GL_COMPILE_STATUS, &mut status);
            if status == 0 {
                return Err("Shader compilation failed".to_string());
            }
            Ok(shader)
        }

        pub fn resize(&mut self, width: u32, height: u32) {
            self.width = width;
            self.height = height;
        }

        pub fn render_and_swap(&mut self, renderer: &Gles3BatchRenderer) {
            if self.surface == EGL_NO_SURFACE
                || self.display == EGL_NO_DISPLAY
                || self.context == EGL_NO_CONTEXT
            {
                return;
            }

            unsafe {
                if eglMakeCurrent(self.display, self.surface, self.surface, self.context) == 0 {
                    return;
                }

                let w = self.width.max(renderer.surface_width).max(1) as i32;
                let h = self.height.max(renderer.surface_height).max(1) as i32;
                glViewport(0, 0, w, h);

                // Clear with soft sage green overworld tone (never pitch black!)
                glClearColor(0.84, 0.90, 0.84, 1.0);
                glClear(GL_COLOR_BUFFER_BIT | GL_DEPTH_BUFFER_BIT);

                glDisable(GL_DEPTH_TEST);
                glDisable(GL_CULL_FACE);
                glEnable(GL_BLEND);
                glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA);

                glUseProgram(self.program);
                glBindVertexArray(self.vao);

                // Upload uniform block and explicitly re-bind base for TBDR GPUs (Mali/PowerVR)
                glBindBuffer(GL_UNIFORM_BUFFER, self.ubo);
                glBufferData(
                    GL_UNIFORM_BUFFER,
                    std::mem::size_of::<Std140UniformBlock>() as isize,
                    &renderer.uniforms as *const _ as *const c_void,
                    GL_DYNAMIC_DRAW,
                );
                glBindBufferBase(GL_UNIFORM_BUFFER, 0, self.ubo);

                // Upload vertex buffer and explicitly bind attribute pointers per pass
                glBindBuffer(GL_ARRAY_BUFFER, self.vbo);
                if !renderer.vertex_buffer.is_empty() {
                    glBufferData(
                        GL_ARRAY_BUFFER,
                        (renderer.vertex_buffer.len() * std::mem::size_of::<SpriteVertex>()) as isize,
                        renderer.vertex_buffer.as_ptr() as *const c_void,
                        GL_DYNAMIC_DRAW,
                    );
                }

                let stride = std::mem::size_of::<SpriteVertex>() as i32;
                glEnableVertexAttribArray(0);
                glVertexAttribPointer(0, 2, GL_FLOAT, GL_FALSE, stride, 0 as *const c_void);
                glEnableVertexAttribArray(1);
                glVertexAttribPointer(1, 2, GL_FLOAT, GL_FALSE, stride, 8 as *const c_void);
                glEnableVertexAttribArray(2);
                glVertexAttribPointer(2, 4, GL_FLOAT, GL_FALSE, stride, 16 as *const c_void);

                glActiveTexture(GL_TEXTURE0);

                // Draw batches with per-atlas texture selection
                if !renderer.batches.is_empty() {
                    for batch in &renderer.batches {
                        let tex = if batch.atlas_id == 0 {
                            self.default_texture
                        } else {
                            self.entity_texture
                        };
                        glBindTexture(GL_TEXTURE_2D, tex);
                        glDrawArrays(
                            GL_TRIANGLES,
                            batch.vertex_offset as i32,
                            batch.vertex_count as i32,
                        );
                    }
                } else if !renderer.vertex_buffer.is_empty() {
                    glBindTexture(GL_TEXTURE_2D, self.default_texture);
                    glDrawArrays(GL_TRIANGLES, 0, renderer.vertex_buffer.len() as i32);
                }

                eglSwapBuffers(self.display, self.surface);
                self.frame_counter += 1;
                if self.frame_counter == 1 || self.frame_counter % 300 == 0 {
                    alog(&format!(
                        "render_and_swap frame={} verts={} batches={}",
                        self.frame_counter,
                        renderer.vertex_buffer.len(),
                        renderer.batches.len()
                    ));
                }
            }
        }

        pub fn make_current(&self) -> bool {
            if self.display == EGL_NO_DISPLAY
                || self.surface == EGL_NO_SURFACE
                || self.context == EGL_NO_CONTEXT
            {
                return false;
            }
            unsafe {
                eglMakeCurrent(self.display, self.surface, self.surface, self.context) != 0
            }
        }

        pub fn destroy_surface(&mut self) {
            alog("destroy_surface: releasing EGL surface and native window, keeping context alive");
            unsafe {
                if self.display != EGL_NO_DISPLAY {
                    eglMakeCurrent(
                        self.display,
                        EGL_NO_SURFACE,
                        EGL_NO_SURFACE,
                        EGL_NO_CONTEXT,
                    );

                    if self.surface != EGL_NO_SURFACE {
                        eglDestroySurface(self.display, self.surface);
                        self.surface = EGL_NO_SURFACE;
                    }
                }
                if !self.native_window.is_null() {
                    ANativeWindow_release(self.native_window);
                    self.native_window = std::ptr::null_mut();
                }
            }
        }

        pub fn is_active(&self) -> bool {
            self.surface != EGL_NO_SURFACE
        }
    }

    impl Drop for HardwareEglRenderer {
        fn drop(&mut self) {
            self.destroy_surface();
            unsafe {
                if self.display != EGL_NO_DISPLAY {
                    if self.context != EGL_NO_CONTEXT {
                        eglDestroyContext(self.display, self.context);
                        self.context = EGL_NO_CONTEXT;
                    }
                    if self.program != 0 {
                        glDeleteProgram(self.program);
                        self.program = 0;
                    }
                    if self.vao != 0 {
                        glDeleteVertexArrays(1, &self.vao);
                        self.vao = 0;
                    }
                    if self.vbo != 0 {
                        glDeleteBuffers(1, &self.vbo);
                        self.vbo = 0;
                    }
                    if self.ubo != 0 {
                        glDeleteBuffers(1, &self.ubo);
                        self.ubo = 0;
                    }
                    if self.default_texture != 0 {
                        glDeleteTextures(1, &self.default_texture);
                        self.default_texture = 0;
                    }
                    if self.entity_texture != 0 {
                        glDeleteTextures(1, &self.entity_texture);
                        self.entity_texture = 0;
                    }
                    eglTerminate(self.display);
                    self.display = EGL_NO_DISPLAY;
                }
            }
        }
    }
}

#[cfg(not(target_os = "android"))]
mod imp {
    use super::*;

    /// Safe no-op mock implementation for non-Android platforms (e.g. host unit tests).
    pub struct HardwareEglRenderer {
        active: bool,
        width: u32,
        height: u32,
    }

    impl Default for HardwareEglRenderer {
        fn default() -> Self {
            Self::new()
        }
    }

    impl HardwareEglRenderer {
        pub fn new() -> Self {
            Self {
                active: false,
                width: 0,
                height: 0,
            }
        }

        pub fn make_current(&self) -> bool {
            self.active
        }

        pub fn attach_surface(
            &mut self,
            _raw_env: *mut jni::sys::JNIEnv,
            _surface_obj: jni::sys::jobject,
            width: u32,
            height: u32,
        ) -> bool {
            self.active = true;
            self.width = width;
            self.height = height;
            true
        }

        pub fn resize(&mut self, width: u32, height: u32) {
            self.width = width;
            self.height = height;
        }

        pub fn render_and_swap(&mut self, _renderer: &Gles3BatchRenderer) {
            // No-op on host platforms
        }

        pub fn destroy_surface(&mut self) {
            self.active = false;
        }

        pub fn is_active(&self) -> bool {
            self.active
        }
    }
}

pub use imp::HardwareEglRenderer;

#[cfg(target_os = "android")]
pub use imp::alog;

#[cfg(not(target_os = "android"))]
pub fn alog(_msg: &str) {}
