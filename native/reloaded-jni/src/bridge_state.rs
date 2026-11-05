use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI32, AtomicU32, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use reloaded_core::input::*;
use reloaded_core::runtime::{EngineState, ReloadedRuntime, SpeedMode};
use reloaded_core::save::SaveManager;
use serde::{Deserialize, Serialize};

use crate::egl_renderer::HardwareEglRenderer;
use crate::lifecycle::{ActivityState, SurfaceLifecycle, SurfaceState};

/// Mapping from native input bitmask to GameMaker 8 virtual key codes.
pub const INPUT_MAP: &[(u32, u8)] = &[
    (BTN_UP, 0x26),     // UpArrow
    (BTN_DOWN, 0x28),   // DownArrow
    (BTN_LEFT, 0x25),   // LeftArrow
    (BTN_RIGHT, 0x27),  // RightArrow
    (BTN_Z, 0x5A),      // 'Z'
    (BTN_X, 0x58),      // 'X'
    (BTN_C, 0x43),      // 'C'
    (BTN_A, 0x41),      // 'A'
    (BTN_S, 0x53),      // 'S'
    (BTN_D, 0x44),      // 'D'
    (BTN_ENTER, 0x0D),  // Return
    (BTN_ESCAPE, 0x1B), // Escape
    (BTN_SPACE, 0x20),  // Space
    (BTN_F1, 0x70),     // F1
    (BTN_F2, 0x71),     // F2
    (BTN_F3, 0x72),     // F3
    (BTN_F4, 0x73),     // F4
    (BTN_F5, 0x74),     // F5
    (BTN_F6, 0x75),     // F6
    (BTN_F7, 0x76),     // F7
    (BTN_F8, 0x77),     // F8
    (BTN_F9, 0x78),     // F9
    (BTN_F10, 0x79),    // F10
    (BTN_F11, 0x7A),    // F11
    (BTN_F12, 0x7B),    // F12
];

/// Synchronizes native input mask transitions into GM8 virtual key states.
pub fn sync_mask_to_gm8_input(input: &mut gm8emulator::input::Input, current: u32, prev: u32) {
    let pressed = current & !prev;
    let released = !current & prev;

    for &(bit, vk) in INPUT_MAP {
        if (pressed & bit) != 0 {
            input.button_press(vk, true);
        } else if (released & bit) != 0 {
            input.button_release(vk, true);
        }
    }
}

/// Active running GameMaker 8 emulator session.
pub struct ActiveGm8Game {
    pub game: gm8emulator::game::Game,
    pub exe_path: PathBuf,
    pub prev_input_mask: u32,
}

unsafe impl Send for ActiveGm8Game {}

/// Global bridge state instance.
static GLOBAL_BRIDGE: OnceLock<BridgeState> = OnceLock::new();

/// Returns the singleton instance of the `BridgeState`.
pub fn get_bridge() -> &'static BridgeState {
    GLOBAL_BRIDGE.get_or_init(BridgeState::new)
}

/// Package import result payload.
#[derive(Debug, Serialize, Deserialize)]
pub struct ImportPackageResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<reloaded_core::loader::GameMetadata>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub atlas_cache_valid: Option<bool>,
}

/// Comprehensive engine metrics response.
#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub state: EngineState,
    pub speed_mode: SpeedMode,
    pub delta_ms: f32,
    pub logic_ticks: u32,
    pub draw_calls: u32,
    pub batch_count: u32,
    #[serde(rename = "drawBatches")]
    pub draw_batches: u32,
    pub quad_count: u32,
    pub surface_active: bool,
    pub total_ticks: u64,
    pub input_mask: u32,
    pub current_fps: u32,
    pub fps: f32,
    pub surface_state: SurfaceState,
    pub activity_state: ActivityState,
    #[serde(default)]
    pub room_id: i32,
    #[serde(default)]
    pub room_name: String,
    #[serde(default)]
    pub room_speed: u32,
    #[serde(default)]
    pub instance_count: usize,
    #[serde(default)]
    pub steps_total: u64,
}

/// Thread-safe Native Bridge State coordinating Android UI, rendering, and engine runtime.
pub struct BridgeState {
    /// 100% lock-free input bitmask for zero-latency touch and gamepad events.
    /// Android UI thread writes to this directly without ever contending with the render mutex!
    pub input_mask: AtomicU32,

    /// Latch for short button taps that occur between frames.
    /// Bitwise OR'd on press so taps shorter than 1 frame time (20-50ms) are NEVER lost!
    pub input_pressed_latch: AtomicU32,

    /// Previous effective input mask for transition detection across both frames and inner modal poll loops.
    pub prev_input_mask: AtomicU32,

    /// Surface and Activity lifecycle state machine.
    pub lifecycle: Mutex<SurfaceLifecycle>,

    /// Core engine runtime instance.
    pub runtime: Mutex<Option<ReloadedRuntime>>,

    /// Active GameMaker 8 game instance.
    pub gm8_game: Mutex<Option<ActiveGm8Game>>,

    /// Path of an executable that failed to load to prevent endless reload loops.
    pub gm8_failed_exe: Mutex<Option<PathBuf>>,

    /// Hardware EGL and OpenGL ES 3.0 renderer.
    pub egl_renderer: Mutex<HardwareEglRenderer>,

    /// Cached frame rate calculation.
    pub last_fps: AtomicU32,

    /// Total step count executed for exact metrics.
    pub total_steps: AtomicU64,

    /// Active speed multiplier (1 for normal, 2 for fast).
    pub speed_multiplier: AtomicI32,

    /// Storage root path.
    pub storage_path: Mutex<Option<PathBuf>>,
}

impl Default for BridgeState {
    fn default() -> Self {
        Self::new()
    }
}

impl BridgeState {
    /// Creates a new `BridgeState`.
    pub fn new() -> Self {
        std::panic::set_hook(Box::new(|info| {
            crate::egl_renderer::alog(&format!("RUST PANIC: {}", info));
        }));
        Self {
            input_mask: AtomicU32::new(0),
            input_pressed_latch: AtomicU32::new(0),
            prev_input_mask: AtomicU32::new(0),
            lifecycle: Mutex::new(SurfaceLifecycle::new()),
            runtime: Mutex::new(None),
            gm8_game: Mutex::new(None),
            gm8_failed_exe: Mutex::new(None),
            egl_renderer: Mutex::new(HardwareEglRenderer::new()),
            last_fps: AtomicU32::new(60),
            total_steps: AtomicU64::new(0),
            speed_multiplier: AtomicI32::new(1),
            storage_path: Mutex::new(None),
        }
    }

    /// Overwrites the full input mask (100% lock-free).
    /// Newly pressed bits are automatically latched into `input_pressed_latch`.
    #[inline]
    pub fn update_input_mask(&self, mask: u32) {
        let prev = self.input_mask.swap(mask, Ordering::Relaxed);
        let newly_pressed = mask & !prev;
        if newly_pressed != 0 {
            self.input_pressed_latch.fetch_or(newly_pressed, Ordering::Relaxed);
        }
    }

    /// Sets or clears an individual button bit (100% lock-free).
    /// Pressed events are latched into `input_pressed_latch` so taps between frames are never missed.
    #[inline]
    pub fn set_button_state(&self, button_bit: u32, pressed: bool) {
        if pressed {
            self.input_mask.fetch_or(button_bit, Ordering::Relaxed);
            self.input_pressed_latch.fetch_or(button_bit, Ordering::Relaxed);
        } else {
            self.input_mask.fetch_and(!button_bit, Ordering::Relaxed);
        }
    }

    /// Reads the current active input bitmask (100% lock-free).
    #[inline]
    pub fn get_input_mask(&self) -> u32 {
        self.input_mask.load(Ordering::Relaxed)
    }

    /// Resets all inputs (clears `input_mask`, `input_pressed_latch`, and `prev_input_mask`).
    pub fn reset_inputs(&self) {
        self.input_mask.store(0, Ordering::Relaxed);
        self.input_pressed_latch.store(0, Ordering::Relaxed);
        self.prev_input_mask.store(0, Ordering::Relaxed);
    }

    /// Initializes the engine runtime with the specified storage directory.
    /// Idempotent if a runtime is already initialized so imported package state is preserved.
    pub fn init(&self, storage_path: PathBuf) -> bool {
        log::info!("Initializing NativeBridge with storage path: {:?}", storage_path);

        if let Ok(mut path_guard) = self.storage_path.lock() {
            *path_guard = Some(storage_path.clone());
        }

        if let Ok(mut rt_guard) = self.runtime.lock() {
            if rt_guard.is_none() {
                *rt_guard = Some(ReloadedRuntime::new(storage_path));
            }
            true
        } else {
            log::error!("Failed to acquire runtime lock during nativeInit");
            false
        }
    }

    /// Imports a game package or descriptor and configures texture atlas cache.
    pub fn import_package(&self, package_path: &Path, cache_dir: &Path) -> String {
        let mut rt_guard = match self.runtime.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        let runtime = match rt_guard.as_mut() {
            Some(rt) => rt,
            None => {
                let err_resp = ImportPackageResponse {
                    status: "error".to_string(),
                    message: Some("Runtime not initialized. Call nativeInit first.".to_string()),
                    metadata: None,
                    atlas_cache_valid: None,
                };
                return serde_json::to_string(&err_resp).unwrap_or_default();
            }
        };

        match runtime.import_package(package_path, cache_dir) {
            Ok(meta) => {
                if let Ok(mut g) = self.gm8_game.lock() {
                    *g = None;
                }
                if let Ok(mut f) = self.gm8_failed_exe.lock() {
                    *f = None;
                }

                let cache_valid = runtime
                    .atlas_cache
                    .is_manifest_valid(cache_dir, meta.checksum_fnv1a);

                let resp = ImportPackageResponse {
                    status: "ok".to_string(),
                    message: None,
                    metadata: Some(meta),
                    atlas_cache_valid: Some(cache_valid),
                };
                serde_json::to_string(&resp).unwrap_or_default()
            }
            Err(e) => {
                let err_resp = ImportPackageResponse {
                    status: "error".to_string(),
                    message: Some(format!("Failed to import package: {e}")),
                    metadata: None,
                    atlas_cache_valid: None,
                };
                serde_json::to_string(&err_resp).unwrap_or_default()
            }
        }
    }

    /// Handles Android Surface creation.
    pub fn on_surface_created(&self, width: i32, height: i32) {
        log::info!("Surface created: {}x{}", width, height);

        if let Ok(mut lc) = self.lifecycle.lock() {
            lc.on_surface_created(width, height);
        }

        if let Ok(mut rt_guard) = self.runtime.lock() {
            if let Some(rt) = rt_guard.as_mut() {
                rt.on_surface_created(width.max(0) as u32, height.max(0) as u32);
            }
        }
    }

    /// Attaches an Android ANativeWindow surface and initializes hardware EGL rendering.
    pub fn attach_surface(
        &self,
        raw_env: *mut jni::sys::JNIEnv,
        surface: jni::sys::jobject,
        width: i32,
        height: i32,
    ) -> bool {
        let attached = if let Ok(mut egl) = self.egl_renderer.lock() {
            egl.attach_surface(raw_env, surface, width.max(0) as u32, height.max(0) as u32)
        } else {
            false
        };

        if attached {
            self.on_surface_created(width, height);
            self.on_surface_changed(width, height);
            self.resume();
        }

        attached
    }

    /// Handles Android Surface dimension changes.
    pub fn on_surface_changed(&self, width: i32, height: i32) {
        log::info!("Surface changed: {}x{}", width, height);

        if let Ok(mut egl) = self.egl_renderer.lock() {
            egl.resize(width.max(0) as u32, height.max(0) as u32);
        }

        if let Ok(mut lc) = self.lifecycle.lock() {
            lc.on_surface_changed(width, height);
        }

        if let Ok(mut rt_guard) = self.runtime.lock() {
            if let Some(rt) = rt_guard.as_mut() {
                rt.on_surface_changed(width.max(0) as u32, height.max(0) as u32);
            }
        }
    }

    /// Handles Android Surface destruction (app backgrounded or lost window).
    /// Ensures background freeze prevention.
    pub fn on_surface_destroyed(&self) {
        log::info!("Surface destroyed. Halting render loop to prevent background freeze.");

        if let Ok(mut egl) = self.egl_renderer.lock() {
            egl.destroy_surface();
        }

        if let Ok(mut lc) = self.lifecycle.lock() {
            lc.on_surface_destroyed();
        }

        if let Ok(mut rt_guard) = self.runtime.lock() {
            if let Some(rt) = rt_guard.as_mut() {
                rt.on_surface_destroyed();
            }
        }
    }

    /// Synchronizes input mask, steps the frame, renders and swaps buffers, and returns current FPS.
    pub fn step_frame(&self, delta_ms: f32) -> i32 {
        // 1. Check lifecycle to prevent rendering to destroyed surface (avoid GPU driver freezes/crashes)
        let can_render = self
            .lifecycle
            .lock()
            .map(|lc| lc.can_render())
            .unwrap_or(false);

        if !can_render {
            return 0;
        }

        // 2. Synchronize lock-free AtomicU32 input mask + latched short taps
        let current_mask = self.input_mask.load(Ordering::Relaxed);
        let latched_mask = self.input_pressed_latch.swap(0, Ordering::Relaxed);
        let effective_mask = current_mask | latched_mask;

        #[cfg(target_os = "android")]
        {
            // Extract executable path without holding runtime lock across rendering
            let extracted_exe_path = {
                let rt_guard = self.runtime.lock().ok();
                if let Some(Some(rt)) = rt_guard.as_ref().map(|g| g.as_ref()) {
                    rt.extracted_exe_path.clone()
                } else {
                    None
                }
            };

            if let Some(ref exe_path) = extracted_exe_path {
                if let Ok(egl) = self.egl_renderer.lock() {
                    egl.make_current();
                }

                let already_failed = self
                    .gm8_failed_exe
                    .lock()
                    .map(|f| f.as_ref() == Some(exe_path))
                    .unwrap_or(false);

                if !already_failed {
                    let mut gm8_guard = match self.gm8_game.lock() {
                        Ok(guard) => guard,
                        Err(poisoned) => poisoned.into_inner(),
                    };

                    if gm8_guard.is_none() || gm8_guard.as_ref().map(|g| &g.exe_path) != Some(exe_path) {
                        crate::egl_renderer::alog(&format!("OpenGMK: Launching GM8 game from {:?}", exe_path));
                        match launch_gm8_game(exe_path) {
                            Ok(game) => {
                                crate::egl_renderer::alog("OpenGMK: GM8 game launched and initialized successfully!");
                                *gm8_guard = Some(ActiveGm8Game {
                                    game,
                                    exe_path: exe_path.clone(),
                                    prev_input_mask: 0,
                                });
                            }
                            Err(e) => {
                                crate::egl_renderer::alog(&format!("OpenGMK ERROR: Failed to launch {:?}: {}", exe_path, e));
                                if let Ok(mut failed_guard) = self.gm8_failed_exe.lock() {
                                    *failed_guard = Some(exe_path.clone());
                                }
                            }
                        }
                    }

                    if let Some(active) = gm8_guard.as_mut() {
                        let prev_mask = self.prev_input_mask.swap(effective_mask, Ordering::Relaxed);
                        sync_mask_to_gm8_input(&mut active.game.input, effective_mask, prev_mask);
                        active.prev_input_mask = effective_mask;

                        // Execute exactly 1 frame step per call; Android frame pacing (ReloadedGameThread)
                        // controls frequency/timing according to room.speed * speedMultiplier.
                        let step_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            active.game.step_single_frame()
                        }));
                        self.total_steps.fetch_add(1, Ordering::Relaxed);
                        match step_res {
                            Ok(Ok(_)) => {}
                            Ok(Err(e)) => {
                                crate::egl_renderer::alog(&format!("OpenGMK Frame Error: {}", e));
                            }
                            Err(_) => {
                                crate::egl_renderer::alog("OpenGMK Frame Panic caught by catch_unwind!");
                            }
                        }

                        let fps = if delta_ms > 0.001 {
                            (1000.0 / delta_ms).round() as i32
                        } else {
                            60
                        };

                        let clamped_fps = fps.clamp(0, 999);
                        self.last_fps.store(clamped_fps as u32, Ordering::Relaxed);
                        return clamped_fps;
                    }
                }
            }
        }

        // Fallback: synthetic engine renderer path
        let mut rt_guard = match self.runtime.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        if let Some(rt) = rt_guard.as_mut() {
            rt.input.set_mask(effective_mask);

            let _metrics = rt.step_frame(delta_ms);
            self.total_steps.fetch_add(1, Ordering::Relaxed);

            // 3. Hardware EGL render pass and buffer swap
            if let Ok(mut egl) = self.egl_renderer.lock() {
                egl.render_and_swap(&rt.renderer);
            }

            let fps = if delta_ms > 0.001 {
                (1000.0 / delta_ms).round() as i32
            } else {
                60
            };

            let clamped_fps = fps.clamp(0, 999);
            self.last_fps.store(clamped_fps as u32, Ordering::Relaxed);
            clamped_fps
        } else {
            0
        }
    }

    /// Pulls interleaved stereo audio frames from runtime audio mixer.
    pub fn pull_audio_samples(&self, out_buf: &mut [f32]) -> usize {
        let mut rt_guard = match self.runtime.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        if let Some(rt) = rt_guard.as_mut() {
            rt.audio.render_audio_frames(out_buf);
            out_buf.len()
        } else {
            out_buf.fill(0.0);
            0
        }
    }

    /// Sets simulation speed multiplier (1 or 2).
    pub fn set_speed_multiplier(&self, multiplier: i32) {
        let mult = match multiplier {
            2 => 2,
            _ => 1,
        };
        self.speed_multiplier.store(mult, Ordering::Relaxed);
        let mode = match mult {
            2 => SpeedMode::Fast2x,
            _ => SpeedMode::Normal1x,
        };

        if let Ok(mut rt_guard) = self.runtime.lock() {
            if let Some(rt) = rt_guard.as_mut() {
                rt.set_speed_mode(mode);
            }
        }
    }

    /// Pauses game execution.
    pub fn pause(&self) {
        if let Ok(mut lc) = self.lifecycle.lock() {
            lc.pause();
        }

        if let Ok(mut rt_guard) = self.runtime.lock() {
            if let Some(rt) = rt_guard.as_mut() {
                rt.pause();
            }
        }
    }

    /// Resumes game execution.
    pub fn resume(&self) {
        if let Ok(mut lc) = self.lifecycle.lock() {
            lc.resume();
        }

        if let Ok(mut rt_guard) = self.runtime.lock() {
            if let Some(rt) = rt_guard.as_mut() {
                rt.resume();
            }
        }
    }

    /// Returns JSON string of engine performance metrics.
    /// Uses non-blocking `try_lock()` so that UI thread queries never block on the game thread.
    pub fn get_engine_metrics_json(&self) -> String {
        let rt_guard = self.runtime.try_lock().ok();
        let lc_guard = self.lifecycle.try_lock().ok();

        let (surface_state, activity_state) = lc_guard
            .map(|lc| (lc.surface_state(), lc.activity_state()))
            .unwrap_or((SurfaceState::Ready { width: 0, height: 0 }, ActivityState::Active));

        let current_fps = self.last_fps.load(Ordering::Relaxed);
        let input_mask = self.input_mask.load(Ordering::Relaxed);
        let steps_total = self.total_steps.load(Ordering::Relaxed);

        #[cfg(target_os = "android")]
        {
            if let Ok(gm8_guard) = self.gm8_game.try_lock() {
                if let Some(active) = gm8_guard.as_ref() {
                    let room_id = active.game.room.id;
                    let room_speed = active.game.room.speed;
                    let room_name = active
                        .game
                        .assets
                        .rooms
                        .get(room_id as usize)
                        .and_then(|r| r.as_ref().map(|b| b.name.to_string()))
                        .unwrap_or_else(|| format!("Room_{room_id}"));
                    let instance_count = active.game.room.instance_list.count_all_active();

                    let speed_mult = self.speed_multiplier.load(Ordering::Relaxed);
                    let speed_mode = if speed_mult == 2 {
                        SpeedMode::Fast2x
                    } else {
                        SpeedMode::Normal1x
                    };

                    let resp = MetricsResponse {
                        state: EngineState::Running,
                        speed_mode,
                        delta_ms: if current_fps > 0 { 1000.0 / current_fps as f32 } else { 33.3 },
                        logic_ticks: 1,
                        draw_calls: 1,
                        batch_count: 1,
                        draw_batches: 1,
                        quad_count: instance_count as u32,
                        surface_active: true,
                        total_ticks: active.game.frame_counter as u64,
                        input_mask,
                        current_fps,
                        fps: current_fps as f32,
                        surface_state,
                        activity_state,
                        room_id,
                        room_name,
                        room_speed,
                        instance_count,
                        steps_total,
                    };
                    return serde_json::to_string(&resp).unwrap_or_default();
                }
            }
        }

        if let Some(Some(rt)) = rt_guard.as_ref().map(|g| g.as_ref()) {
            let batch_count = rt.renderer.batches.len() as u32;
            let quad_count: u32 = rt
                .renderer
                .batches
                .iter()
                .map(|b| b.quad_count as u32)
                .sum();
            let resp = MetricsResponse {
                state: rt.state,
                speed_mode: rt.speed_mode,
                delta_ms: 16.6,
                logic_ticks: rt.speed_mode as u32,
                draw_calls: batch_count,
                batch_count,
                draw_batches: batch_count.max(1),
                quad_count,
                surface_active: rt.renderer.surface_active,
                total_ticks: rt.total_ticks,
                input_mask,
                current_fps,
                fps: current_fps as f32,
                surface_state,
                activity_state,
                room_id: 0,
                room_name: String::new(),
                room_speed: 30,
                instance_count: 0,
                steps_total,
            };
            serde_json::to_string(&resp).unwrap_or_default()
        } else {
            let is_inited = self.storage_path.try_lock().ok().and_then(|p| p.clone()).is_some();
            let state = if is_inited {
                EngineState::Running
            } else {
                EngineState::Uninitialized
            };
            let speed_mult = self.speed_multiplier.load(Ordering::Relaxed);
            let speed_mode = if speed_mult == 2 {
                SpeedMode::Fast2x
            } else {
                SpeedMode::Normal1x
            };
            let resp = MetricsResponse {
                state,
                speed_mode,
                delta_ms: 16.6,
                logic_ticks: 1,
                draw_calls: 1,
                batch_count: 1,
                draw_batches: 1,
                quad_count: 1,
                surface_active: true,
                total_ticks: 0,
                input_mask,
                current_fps,
                fps: current_fps as f32,
                surface_state,
                activity_state,
                room_id: 0,
                room_name: String::new(),
                room_speed: 30,
                instance_count: 0,
                steps_total,
            };
            serde_json::to_string(&resp).unwrap_or_default()
        }
    }

    /// Lists save files in JSON format for the specified game directory.
    pub fn list_saves_json(&self, game_dir: &Path) -> String {
        let mgr = SaveManager::new(game_dir);
        match mgr.list_save_slots() {
            Ok(slots) => serde_json::to_string(&slots).unwrap_or_else(|_| "[]".to_string()),
            Err(e) => {
                log::error!("Failed to list saves: {e}");
                "[]".to_string()
            }
        }
    }

    /// Creates a timestamped backup of the designated save slot.
    pub fn create_save_backup(&self, game_dir: &Path, slot_name: &str) -> bool {
        let mgr = SaveManager::new(game_dir);
        if slot_name.trim().eq_ignore_ascii_case("Pokemon.ini") {
            let _ = mgr.ensure_directories();
            let ini_src = mgr.ini_path();
            if ini_src.exists() {
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let dst = mgr.backup_dir().join(format!("Pokemon_{}.ini", ts));
                return std::fs::copy(&ini_src, dst).is_ok();
            }
            return false;
        }

        let slot_num = Self::parse_slot_number(slot_name);
        if !(1..=3).contains(&slot_num) {
            log::warn!("Invalid slot identifier: '{}'", slot_name);
            return false;
        }

        match mgr.backup_save(slot_num) {
            Ok(path) => {
                log::info!("Created save backup at: {:?}", path);
                true
            }
            Err(e) => {
                log::error!("Failed to create save backup for slot {}: {e}", slot_num);
                false
            }
        }
    }

    /// Parses a slot number from strings like "1", "Save1", "Save2.sav", or "slot3".
    pub fn parse_slot_number(input: &str) -> u32 {
        let trimmed = input.trim();
        if let Ok(n) = trimmed.parse::<u32>() {
            return n;
        }

        // Search for first matching digit 1, 2, or 3
        for ch in trimmed.chars() {
            if let Some(digit) = ch.to_digit(10) {
                if digit >= 1 && digit <= 3 {
                    return digit;
                }
            }
        }

        0
    }

    /// Returns the current GameMaker room speed (FPS target, typically 30).
    pub fn get_room_speed(&self) -> i32 {
        #[cfg(target_os = "android")]
        {
            let guard = match self.gm8_game.lock() {
                Ok(g) => g,
                Err(poisoned) => poisoned.into_inner(),
            };
            if let Some(active) = guard.as_ref() {
                let speed = active.game.room.speed as i32;
                if speed > 0 {
                    return speed;
                }
            }
        }
        30
    }
}

/// Helper to load GM8 assets from exe, construct Game, and initialize first room.
#[cfg(target_os = "android")]
fn launch_gm8_game(exe_path: &Path) -> Result<gm8emulator::game::Game, Box<dyn std::error::Error>> {
    crate::egl_renderer::alog(&format!("OpenGMK: reading exe {:?}", exe_path));
    let mut exe_bytes = std::fs::read(exe_path)?;
    crate::egl_renderer::alog(&format!("OpenGMK: parsing {} bytes with gm8exe", exe_bytes.len()));
    let mut assets = gm8exe::reader::from_exe(
        &mut exe_bytes,
        Some(|s: &str| crate::egl_renderer::alog(&format!("gm8exe: {}", s))),
        false,
        false,
    )?;

    // Patch GM8 scripts for Pokemon Reloaded anti-tamper / date checks
    let mut patched_validate_cv = false;
    if assets.scripts.len() > 10002 {
        if let Some(ref mut script) = assets.scripts[10002] {
            if script.name.0.as_ref() == b"ValidateCV" || script.name.to_string() == "ValidateCV" {
                crate::egl_renderer::alog("OpenGMK: Patching script ValidateCV at index 10002 (return 1)");
                script.source.0 = b"return 1;".to_vec().into_boxed_slice();
                patched_validate_cv = true;
            }
        }
    }
    if !patched_validate_cv {
        for opt in assets.scripts.iter_mut().flatten() {
            if opt.name.0.as_ref() == b"ValidateCV" || opt.name.to_string() == "ValidateCV" {
                crate::egl_renderer::alog("OpenGMK: Patching script ValidateCV via search (return 1)");
                opt.source.0 = b"return 1;".to_vec().into_boxed_slice();
                break;
            }
        }
    }

    let mut patched_opt_load_fs = false;
    if assets.scripts.len() > 8867 {
        if let Some(ref mut script) = assets.scripts[8867] {
            if script.name.0.as_ref() == b"OptLoadFS" || script.name.to_string() == "OptLoadFS" {
                crate::egl_renderer::alog("OpenGMK: Patching script OptLoadFS at index 8867 (bypass lastDateStored and min date)");
                let src_str = String::from_utf8_lossy(&script.source.0);
                let patched_str = src_str
                    .replace("if(global.lastDateStored>ex(7814)){ex(8768,2);exit;}", "/* bypass lastDateStored */")
                    .replace("if(ex(7812,14,5,2026,0,0,0)>ex(7814)){ex(8768,2);exit;}", "/* bypass min date */");
                script.source.0 = patched_str.into_bytes().into_boxed_slice();
                patched_opt_load_fs = true;
            }
        }
    }
    if !patched_opt_load_fs {
        for opt in assets.scripts.iter_mut().flatten() {
            if opt.name.0.as_ref() == b"OptLoadFS" || opt.name.to_string() == "OptLoadFS" {
                crate::egl_renderer::alog("OpenGMK: Patching script OptLoadFS via search (bypass lastDateStored and min date)");
                let src_str = String::from_utf8_lossy(&opt.source.0);
                let patched_str = src_str
                    .replace("if(global.lastDateStored>ex(7814)){ex(8768,2);exit;}", "/* bypass lastDateStored */")
                    .replace("if(ex(7812,14,5,2026,0,0,0)>ex(7814)){ex(8768,2);exit;}", "/* bypass min date */");
                opt.source.0 = patched_str.into_bytes().into_boxed_slice();
                break;
            }
        }
    }

    let temp_dir = exe_path.parent().unwrap_or(Path::new(".")).join("gm_temp");
    let _ = std::fs::create_dir_all(&temp_dir);

    crate::egl_renderer::alog("OpenGMK: calling Game::launch...");
    let mut game = gm8emulator::game::Game::launch(
        assets,
        exe_path.to_path_buf(),
        Vec::new(),
        Some(temp_dir),
        encoding_rs::WINDOWS_1252,
        false, // frame_limiter
        0,     // frame_limit_at
        false, // capture_recording
        gm8emulator::game::PlayType::Normal,
    )?;

    // Game::launch initializes the clock to zero (1899-12-30). StartupEpoch
    // allows DateTime::now() to advance continuously in real time during the game.
    game.clock = gm8emulator::game::GameClock::StartupEpoch(std::time::Instant::now());
    game.esc_close_game = false;
    game.uninit_fields_are_zero = true;
    game.scaling = gm8emulator::render::Scaling::Aspect(-1.0);

    game.input_poll_hook = Some(Box::new(|input| {
        let bridge = get_bridge();
        let current = bridge.input_mask.load(Ordering::Relaxed);
        let latched = bridge.input_pressed_latch.swap(0, Ordering::Relaxed);
        let effective = current | latched;
        let prev = bridge.prev_input_mask.swap(effective, Ordering::Relaxed);
        sync_mask_to_gm8_input(input, effective, prev);
    }));

    crate::egl_renderer::alog("OpenGMK: calling game.init()...");
    game.init()?;
    Ok(game)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_lock_free_input_mask_concurrent_threads() {
        let bridge = Arc::new(BridgeState::new());
        assert_eq!(bridge.get_input_mask(), 0);

        let mut handles = Vec::new();

        // 8 threads concurrently setting and clearing different bits
        let button_bits = [
            BTN_UP, BTN_DOWN, BTN_LEFT, BTN_RIGHT, BTN_Z, BTN_X, BTN_ENTER, BTN_ESCAPE,
        ];

        for &bit in &button_bits {
            let b = Arc::clone(&bridge);
            handles.push(thread::spawn(move || {
                for _ in 0..10_000 {
                    b.set_button_state(bit, true);
                    let m = b.get_input_mask();
                    assert_ne!(m & bit, 0);

                    b.set_button_state(bit, false);
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        // All threads finished, mask should be 0
        assert_eq!(bridge.get_input_mask(), 0);
    }

    #[test]
    fn test_atomic_input_mask_direct_store_and_f_keys() {
        let bridge = BridgeState::new();

        bridge.update_input_mask(BTN_F1 | BTN_Z | BTN_SPACE);
        assert_eq!(bridge.get_input_mask(), BTN_F1 | BTN_Z | BTN_SPACE);

        bridge.set_button_state(BTN_F12, true);
        assert_eq!(
            bridge.get_input_mask(),
            BTN_F1 | BTN_Z | BTN_SPACE | BTN_F12
        );

        bridge.set_button_state(BTN_Z, false);
        assert_eq!(bridge.get_input_mask(), BTN_F1 | BTN_SPACE | BTN_F12);

        bridge.update_input_mask(0);
        assert_eq!(bridge.get_input_mask(), 0);
    }

    #[test]
    fn test_parse_slot_number() {
        assert_eq!(BridgeState::parse_slot_number("1"), 1);
        assert_eq!(BridgeState::parse_slot_number("2"), 2);
        assert_eq!(BridgeState::parse_slot_number("3"), 3);
        assert_eq!(BridgeState::parse_slot_number("Save1.sav"), 1);
        assert_eq!(BridgeState::parse_slot_number("Save2"), 2);
        assert_eq!(BridgeState::parse_slot_number("slot3"), 3);
        assert_eq!(BridgeState::parse_slot_number("invalid"), 0);
        assert_eq!(BridgeState::parse_slot_number("99"), 99);
    }

    #[test]
    fn test_bridge_lifecycle_and_step_frame_execution() {
        let temp_dir = std::env::temp_dir().join(format!("bridge_test_{}", std::process::id()));
        let bridge = BridgeState::new();

        assert!(bridge.init(temp_dir.clone()));

        // Before surface is created, step_frame returns 0 (prevents background crash)
        assert_eq!(bridge.step_frame(16.6), 0);

        // Surface Created & Changed
        bridge.on_surface_created(1280, 720);
        bridge.on_surface_changed(1280, 720);
        bridge.resume();

        // Now frame steps successfully and returns calculated FPS
        let fps = bridge.step_frame(16.666);
        assert_eq!(fps, 60);

        // Update inputs and step
        bridge.set_button_state(BTN_Z, true);
        assert_eq!(bridge.get_input_mask(), BTN_Z);

        let fps2 = bridge.step_frame(16.666);
        assert_eq!(fps2, 60);

        // Surface Destroyed (Background Freeze Prevention)
        bridge.on_surface_destroyed();
        assert_eq!(
            bridge.step_frame(16.6),
            0,
            "Must return 0 while backgrounded"
        );

        // Recreated
        bridge.on_surface_created(1280, 720);
        bridge.on_surface_changed(1280, 720);
        assert_eq!(bridge.step_frame(16.666), 60);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_speed_multipliers() {
        let temp_dir = std::env::temp_dir().join(format!("bridge_speed_test_{}", std::process::id()));
        let bridge = BridgeState::new();
        bridge.init(temp_dir.clone());

        bridge.set_speed_multiplier(2);
        let metrics_json = bridge.get_engine_metrics_json();
        assert!(metrics_json.contains("Fast2x"));

        // Unsupported values fall back to normal speed.
        bridge.set_speed_multiplier(3);
        let metrics_json2 = bridge.get_engine_metrics_json();
        assert!(metrics_json2.contains("Normal1x"));

        bridge.set_speed_multiplier(1);
        let metrics_json3 = bridge.get_engine_metrics_json();
        assert!(metrics_json3.contains("Normal1x"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_attach_surface_and_pull_audio_samples() {
        let temp_dir = std::env::temp_dir().join(format!("bridge_audio_test_{}", std::process::id()));
        let bridge = BridgeState::new();
        bridge.init(temp_dir.clone());

        // Test attach surface (safe stub on host, initializes lifecycle)
        assert!(bridge.attach_surface(std::ptr::null_mut(), std::ptr::null_mut(), 1920, 1080));
        assert!(bridge.lifecycle.lock().unwrap().can_render());

        // Test pulling audio samples
        let mut audio_buf = vec![0.0f32; 512];
        let pulled = bridge.pull_audio_samples(&mut audio_buf);
        assert_eq!(pulled, 512);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_sync_mask_to_gm8_input() {
        let mut input = gm8emulator::input::Input::new();

        // Initially nothing pressed
        assert!(!input.keyboard_check(0x5A)); // Z
        assert!(!input.keyboard_check(0x26)); // UP

        // Press Z and UP
        let mask1 = BTN_Z | BTN_UP;
        sync_mask_to_gm8_input(&mut input, mask1, 0);

        assert!(input.keyboard_check(0x5A));
        assert!(input.keyboard_check(0x26));
        assert!(input.keyboard_check_pressed(0x5A));
        assert!(input.keyboard_check_pressed(0x26));
        assert!(!input.keyboard_check(0x58)); // X

        // Step clears pressed state
        input.step();
        assert!(input.keyboard_check(0x5A));
        assert!(!input.keyboard_check_pressed(0x5A));

        // Release Z, press X (UP remains held)
        let mask2 = BTN_UP | BTN_X;
        sync_mask_to_gm8_input(&mut input, mask2, mask1);

        assert!(!input.keyboard_check(0x5A));
        assert!(input.keyboard_check_released(0x5A));
        assert!(input.keyboard_check(0x26));
        assert!(input.keyboard_check(0x58));
        assert!(input.keyboard_check_pressed(0x58));

        // Release all
        sync_mask_to_gm8_input(&mut input, 0, mask2);
        assert!(!input.keyboard_check(0x26));
        assert!(!input.keyboard_check(0x58));
        assert!(input.keyboard_check_released(0x26));
        assert!(input.keyboard_check_released(0x58));
    }

    #[test]
    fn test_sub_frame_tap_latch_preserves_short_clicks() {
        let bridge = BridgeState::new();

        // 1. User performs a rapid 20ms tap on BTN_Z (ACTION_DOWN then ACTION_UP)
        bridge.set_button_state(BTN_Z, true);
        assert_ne!(bridge.get_input_mask() & BTN_Z, 0);

        bridge.set_button_state(BTN_Z, false);
        // Current input mask is now 0 because the finger was released before step_frame
        assert_eq!(bridge.get_input_mask(), 0);

        // But latch still holds BTN_Z!
        assert_ne!(bridge.input_pressed_latch.load(Ordering::Relaxed) & BTN_Z, 0);

        // 2. step_frame consumes latched_mask and current_mask
        let current = bridge.input_mask.load(Ordering::Relaxed);
        let latched = bridge.input_pressed_latch.swap(0, Ordering::Relaxed);
        let effective = current | latched;

        // Effective mask captures the press!
        assert_eq!(effective, BTN_Z);

        // Latch has been drained
        assert_eq!(bridge.input_pressed_latch.load(Ordering::Relaxed), 0);

        // 3. Next frame without touch gives effective 0, cleanly triggering release
        let current2 = bridge.input_mask.load(Ordering::Relaxed);
        let latched2 = bridge.input_pressed_latch.swap(0, Ordering::Relaxed);
        let effective2 = current2 | latched2;
        assert_eq!(effective2, 0);
    }

    #[test]
    fn test_update_input_mask_latches_new_presses() {
        let bridge = BridgeState::new();

        // Overwrite mask with BTN_SPACE and BTN_X
        bridge.update_input_mask(BTN_SPACE | BTN_X);
        assert_eq!(bridge.input_pressed_latch.load(Ordering::Relaxed), BTN_SPACE | BTN_X);

        // Reset inputs clears both mask and latch
        bridge.reset_inputs();
        assert_eq!(bridge.get_input_mask(), 0);
        assert_eq!(bridge.input_pressed_latch.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_get_room_speed_default() {
        let bridge = BridgeState::new();
        // Uninitialized or non-Android returns default room speed 30
        assert_eq!(bridge.get_room_speed(), 30);
    }
}
