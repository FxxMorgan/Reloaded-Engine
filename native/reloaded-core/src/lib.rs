pub mod audio;
pub mod cache;
pub mod error;
pub mod gml_vm;
pub mod input;
pub mod loader;
pub mod renderer;
pub mod runtime;
pub mod save;

pub use audio::{BgmTrack, LockFreeAudioMixer, SfxVoice, SpscAudioRingBuffer};
pub use cache::{
    AtlasCacheManager, AtlasCacheManifest, AtlasPageDescriptor, SpriteInput, SpriteUvRect,
    DEFAULT_MAX_ATLAS_SIZE, MANIFEST_FILENAME, MANIFEST_VERSION,
};
pub use error::CoreError;
pub use gml_vm::{GmlEvent, GmlValue, GmlVm, Gm8Instance, Gm8Room};
pub use input::{
    AtomicInputState, InputSnapshot, BTN_A, BTN_C, BTN_D, BTN_DOWN, BTN_ENTER, BTN_ESCAPE,
    BTN_F1, BTN_F10, BTN_F11, BTN_F12, BTN_F2, BTN_F3, BTN_F4, BTN_F5, BTN_F6, BTN_F7, BTN_F8,
    BTN_F9, BTN_LEFT, BTN_RIGHT, BTN_S, BTN_SPACE, BTN_UP, BTN_X, BTN_Z,
};
pub use loader::{
    decrypt_gm8_stream, encrypt_gm8_stream, fnv1a_64, GameMetadata, Gm8PackageLoader, PackageType,
};
pub use renderer::{
    DrawBatch, GlStateTracker, Gles3BatchRenderer, RamAtlasPage, SpriteQuad, SpriteVertex,
    Std140UniformBlock, FRAGMENT_SHADER_ES30, VERTEX_SHADER_ES30,
};
pub use runtime::{EngineMetrics, EngineState, ReloadedRuntime, SpeedMode};
pub use save::{PokemonIniData, SaveManager, SaveSlotInfo};
