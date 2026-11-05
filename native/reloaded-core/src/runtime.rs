use std::path::{Path, PathBuf};
use std::sync::Arc;
use serde::{Deserialize, Serialize};

use crate::audio::LockFreeAudioMixer;
use crate::cache::{AtlasCacheManager, AtlasCacheManifest, SpriteInput};
use crate::error::CoreError;
use crate::gml_vm::GmlVm;
use crate::input::AtomicInputState;
use crate::loader::{GameMetadata, Gm8PackageLoader};
use crate::renderer::Gles3BatchRenderer;
use crate::save::SaveManager;

/// Engine lifecycle states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EngineState {
    Uninitialized,
    Ready,
    Running,
    PausedSurfaceLost,
    Stopped,
}

impl Default for EngineState {
    fn default() -> Self {
        EngineState::Uninitialized
    }
}

/// Simulation speed multiplier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum SpeedMode {
    Normal1x = 1,
    Fast2x = 2,
}

impl Default for SpeedMode {
    fn default() -> Self {
        SpeedMode::Normal1x
    }
}

/// Performance and frame execution telemetry.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EngineMetrics {
    pub state: EngineState,
    pub speed_mode: SpeedMode,
    pub delta_ms: f32,
    pub logic_ticks: u32,
    pub draw_calls: u32,
    pub batch_count: u32,
    pub quad_count: u32,
    pub surface_active: bool,
    pub total_ticks: u64,
}

/// Central engine runtime coordinating input, VM, rendering, audio, caching, and saves.
pub struct ReloadedRuntime {
    pub state: EngineState,
    pub speed_mode: SpeedMode,
    pub storage_root: PathBuf,

    pub vm: GmlVm,
    pub input: Arc<AtomicInputState>,
    pub renderer: Gles3BatchRenderer,
    pub audio: LockFreeAudioMixer,
    pub atlas_cache: AtlasCacheManager,
    pub save_manager: SaveManager,

    pub metadata: Option<GameMetadata>,
    pub manifest: Option<AtlasCacheManifest>,
    pub extracted_exe_path: Option<PathBuf>,

    pub total_ticks: u64,
    was_running_before_surface_loss: bool,
}

impl ReloadedRuntime {
    /// Creates a new `ReloadedRuntime` rooted at the given storage directory.
    pub fn new(storage_root: PathBuf) -> Self {
        let save_manager = SaveManager::new(&storage_root);
        let atlas_cache = AtlasCacheManager::default();
        let renderer = Gles3BatchRenderer::default();
        let audio = LockFreeAudioMixer::new(4096);
        let input = Arc::new(AtomicInputState::new());
        let vm = GmlVm::new_interactive_room(800.0, 480.0);

        Self {
            state: EngineState::Uninitialized,
            speed_mode: SpeedMode::Normal1x,
            storage_root,
            vm,
            input,
            renderer,
            audio,
            atlas_cache,
            save_manager,
            metadata: None,
            manifest: None,
            extracted_exe_path: None,
            total_ticks: 0,
            was_running_before_surface_loss: false,
        }
    }

    /// Imports and validates a GM8 package or descriptor, using fast atlas cache when available.
    pub fn import_package(
        &mut self,
        package_path: &Path,
        cache_dir: &Path,
    ) -> Result<GameMetadata, CoreError> {
        let metadata = Gm8PackageLoader::load_from_path(package_path)?;

        // Extract package to game_extracted directory if available
        let extract_dir = self.storage_root.join("game_extracted");
        let extracted_exe_path =
            Gm8PackageLoader::extract_package_to_dir(package_path, &extract_dir, metadata.checksum_fnv1a)?;

        // Check if atlas cache manifest is valid for this package checksum
        let manifest = if self
            .atlas_cache
            .is_manifest_valid(cache_dir, metadata.checksum_fnv1a)
        {
            self.atlas_cache.load_manifest(cache_dir)?
        } else {
            // Generate initial atlas packing descriptors (capped to 512 sprites for <5ms boot)
            let sprite_count = metadata.sprite_count.clamp(1, 512);
            let mut sprites = Vec::with_capacity(sprite_count);
            for i in 0..sprite_count {
                sprites.push(SpriteInput {
                    id: format!("spr_{}", i),
                    width: 32,
                    height: 32,
                });
            }

            let new_manifest = self
                .atlas_cache
                .pack_sprites(&sprites, metadata.checksum_fnv1a)?;
            let _ = self.atlas_cache.save_manifest(&new_manifest, cache_dir);
            new_manifest
        };

        self.metadata = Some(metadata.clone());
        self.extracted_exe_path = extracted_exe_path;
        self.manifest = Some(manifest);
        self.state = EngineState::Ready;

        Ok(metadata)
    }

    /// Called when the Android GLES surface is created or recreated.
    pub fn on_surface_created(&mut self, width: u32, height: u32) {
        self.renderer.on_surface_created(width, height);
        self.vm.current_room.width = width as f32;
        self.vm.current_room.height = height as f32;

        if self.state == EngineState::PausedSurfaceLost {
            if self.was_running_before_surface_loss {
                self.state = EngineState::Running;
                self.audio.resume();
            } else {
                self.state = EngineState::Ready;
            }
        } else if self.state == EngineState::Ready || self.state == EngineState::Uninitialized {
            self.state = EngineState::Running;
        }
    }

    /// Called on surface dimension changes (e.g. orientation or window resize).
    pub fn on_surface_changed(&mut self, width: u32, height: u32) {
        self.renderer.on_surface_changed(width, height);
        self.vm.current_room.width = width as f32;
        self.vm.current_room.height = height as f32;
    }

    /// Called when Android destroys the GLES surface (app minimized/backgrounded).
    /// Transitions to `PausedSurfaceLost`, pauses audio, and preserves RAM state.
    pub fn on_surface_destroyed(&mut self) {
        self.was_running_before_surface_loss = self.state == EngineState::Running;
        self.state = EngineState::PausedSurfaceLost;
        self.renderer.on_surface_destroyed();
        self.audio.pause();
    }

    /// Pauses game execution.
    pub fn pause(&mut self) {
        if self.state == EngineState::Running {
            self.state = EngineState::Ready;
            self.audio.pause();
        }
    }

    /// Resumes game execution if surface is active.
    pub fn resume(&mut self) {
        if self.renderer.surface_active {
            self.state = EngineState::Running;
            self.audio.resume();
        } else {
            self.state = EngineState::PausedSurfaceLost;
        }
    }

    /// Sets simulation speed mode (1x or 2x).
    pub fn set_speed_mode(&mut self, mode: SpeedMode) {
        self.speed_mode = mode;
    }

    /// Ticks the engine by `delta_ms`. Runs `speed_mode as u32` logic ticks,
    /// advances the interactive GML VM, emits quads to the batch renderer,
    /// and performs exactly 1 render pass if the surface is active.
    pub fn step_frame(&mut self, delta_ms: f32) -> EngineMetrics {
        if self.state != EngineState::Running {
            return EngineMetrics {
                state: self.state,
                speed_mode: self.speed_mode,
                delta_ms,
                logic_ticks: 0,
                draw_calls: 0,
                batch_count: 0,
                quad_count: 0,
                surface_active: self.renderer.surface_active,
                total_ticks: self.total_ticks,
            };
        }

        let ticks = self.speed_mode as u32;
        for _ in 0..ticks {
            // Snapshot inputs for this tick and step the VM
            let snap = self.input.snapshot();
            self.vm.step(&snap, &mut self.audio);
            self.total_ticks += 1;
        }

        let mut draw_calls = 0;
        let mut batch_count = 0;
        let mut quad_count = 0;

        if self.renderer.surface_active {
            // Emit room background tiles & entity quads to renderer
            self.vm.emit_quads(&mut self.renderer);
            self.renderer.flush_batches();

            draw_calls = self.renderer.execute_render_pass();
            batch_count = self.renderer.batches.len() as u32;
            quad_count = self
                .renderer
                .batches
                .iter()
                .map(|b| b.quad_count as u32)
                .sum();
        }

        EngineMetrics {
            state: self.state,
            speed_mode: self.speed_mode,
            delta_ms,
            logic_ticks: ticks,
            draw_calls,
            batch_count,
            quad_count,
            surface_active: self.renderer.surface_active,
            total_ticks: self.total_ticks,
        }
    }

    /// Pulls mixed audio frames from the internal mixer into an interleaved stereo float slice.
    /// Returns the number of samples rendered.
    pub fn pull_audio_samples(&mut self, out_interleaved_stereo: &mut [f32]) -> usize {
        self.audio.render_audio_frames(out_interleaved_stereo);
        out_interleaved_stereo.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{BTN_F1, BTN_RIGHT, BTN_Z};

    #[test]
    fn test_runtime_lifecycle_and_speed_modes() {
        let temp_dir = std::env::temp_dir().join(format!("reloaded_rt_test_{}", std::process::id()));
        let mut runtime = ReloadedRuntime::new(temp_dir.clone());
        assert_eq!(runtime.state, EngineState::Uninitialized);

        // Before surface creation, step_frame does nothing
        let m0 = runtime.step_frame(16.6);
        assert_eq!(m0.logic_ticks, 0);

        // Create surface
        runtime.on_surface_created(800, 480);
        runtime.resume();
        assert_eq!(runtime.state, EngineState::Running);

        // Normal 1x: 1 logic tick per frame
        runtime.set_speed_mode(SpeedMode::Normal1x);
        let m1 = runtime.step_frame(16.6);
        assert_eq!(m1.logic_ticks, 1);
        assert_eq!(runtime.total_ticks, 1);

        // Fast 2x: 2 logic ticks per frame
        runtime.set_speed_mode(SpeedMode::Fast2x);
        let m2 = runtime.step_frame(16.6);
        assert_eq!(m2.logic_ticks, 2);
        assert_eq!(runtime.total_ticks, 3);

        // Surface destroyed (background)
        runtime.on_surface_destroyed();
        assert_eq!(runtime.state, EngineState::PausedSurfaceLost);
        assert!(runtime.audio.is_paused());

        // Step while backgrounded does 0 ticks
        let m4 = runtime.step_frame(16.6);
        assert_eq!(m4.logic_ticks, 0);
        assert_eq!(runtime.total_ticks, 3);

        // Return from background (surface re-created)
        runtime.on_surface_created(800, 480);
        assert_eq!(runtime.state, EngineState::Running);
        assert!(!runtime.audio.is_paused());

        let m5 = runtime.step_frame(16.6);
        assert_eq!(m5.logic_ticks, 2); // Fast 2x preserved!
        assert_eq!(runtime.total_ticks, 5);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_runtime_import_package_and_manifest_cache() {
        use std::io::Write;

        let temp_dir = std::env::temp_dir().join(format!("reloaded_rt_pkg_{}", std::process::id()));
        std::fs::create_dir_all(&temp_dir).unwrap();

        let pkg_path = temp_dir.join("PokemonReloaded.exe");
        let cache_dir = temp_dir.join("cache");

        let exe_bytes = Gm8PackageLoader::build_synthetic_gm8_exe(
            "Pokemon Reloaded Runtime Test",
            810,
            50,
            10,
            20,
        );
        let mut f = std::fs::File::create(&pkg_path).unwrap();
        f.write_all(&exe_bytes).unwrap();

        let mut runtime = ReloadedRuntime::new(temp_dir.clone());
        let meta = runtime
            .import_package(&pkg_path, &cache_dir)
            .expect("First import");
        assert_eq!(meta.title, "Pokemon Reloaded Runtime Test");
        assert_eq!(meta.sprite_count, 50);
        assert_eq!(runtime.state, EngineState::Ready);
        assert!(runtime.manifest.is_some());

        // Second import should use cached manifest
        let meta2 = runtime
            .import_package(&pkg_path, &cache_dir)
            .expect("Second import from cache");
        assert_eq!(meta2.checksum_fnv1a, meta.checksum_fnv1a);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_interactive_pipeline_step_render_audio() {
        let temp_dir = std::env::temp_dir().join(format!("reloaded_pipeline_{}", std::process::id()));
        let mut runtime = ReloadedRuntime::new(temp_dir.clone());
        runtime.on_surface_created(800, 480);
        runtime.resume();
        assert_eq!(runtime.state, EngineState::Running);

        let initial_player_x = runtime.vm.get_player().unwrap().x;

        // Press BTN_RIGHT + BTN_Z + BTN_F1
        runtime.input.set_button(BTN_RIGHT, true);
        runtime.input.set_button(BTN_Z, true);
        runtime.input.set_button(BTN_F1, true);

        // Step frame: runs logic tick, triggers bike, attacks, moves player, emits quads & batches
        let metrics = runtime.step_frame(16.6);

        // 1. Verify player instance moved
        let final_player_x = runtime.vm.get_player().unwrap().x;
        assert!(final_player_x > initial_player_x, "Player must have moved right");
        assert!(runtime.vm.bike_active, "Bike must be toggled active");

        // 2. Verify attack instance spawned
        let attack_spawned = runtime
            .vm
            .current_room
            .instances
            .iter()
            .any(|i| i.object_name == "obj_attack_thunderbolt");
        assert!(attack_spawned, "Attack instance must have spawned");

        // 3. Verify non-zero SpriteQuad batches into renderer
        assert!(metrics.batch_count > 0, "Batch count must be greater than 0");
        assert!(metrics.quad_count > 0, "Quad count must be greater than 0");
        assert!(metrics.draw_calls > 0, "Draw calls must be greater than 0");
        assert!(!runtime.renderer.batches.is_empty(), "Renderer batches must not be empty");
        assert!(!runtime.renderer.vertex_buffer.is_empty(), "Vertex buffer must have vertices");

        // 4. Verify non-zero audio samples via pull_audio_samples
        let mut audio_buffer = vec![0.0f32; 256];
        let written = runtime.pull_audio_samples(&mut audio_buffer);
        assert_eq!(written, 256);
        let has_sound = audio_buffer.iter().any(|&sample| sample.abs() > 1e-4);
        assert!(has_sound, "Audio buffer must contain active synthesized sound samples");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
