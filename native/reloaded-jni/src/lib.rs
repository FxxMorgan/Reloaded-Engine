pub mod bridge_state;
pub mod egl_renderer;
pub mod lifecycle;

use std::path::{Path, PathBuf};
use jni::objects::{JClass, JFloatArray, JObject, JString};
use jni::sys::{jboolean, jfloat, jint, jstring, JNI_FALSE, JNI_TRUE};
use jni::JNIEnv;

use crate::bridge_state::get_bridge;

/// JNI entrypoint: Initializes the Pokemon Reloaded engine runtime.
#[no_mangle]
pub unsafe extern "system" fn Java_com_reloaded_android_bridge_NativeBridge_nativeInit(
    mut env: JNIEnv,
    _class: JClass,
    storage_path: JString,
) -> jboolean {
    let path_str: String = match env.get_string(&storage_path) {
        Ok(s) => s.into(),
        Err(e) => {
            log::error!("nativeInit: failed to parse storage_path: {e}");
            return JNI_FALSE;
        }
    };

    let path = PathBuf::from(path_str);
    if get_bridge().init(path) {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

/// JNI entrypoint: Imports a GM8 game package and initializes texture atlas cache.
/// Returns JSON string with status, game metadata, and atlas cache status.
#[no_mangle]
pub unsafe extern "system" fn Java_com_reloaded_android_bridge_NativeBridge_nativeImportPackage(
    mut env: JNIEnv,
    _class: JClass,
    package_path: JString,
    cache_dir: JString,
) -> jstring {
    let pkg_path: String = match env.get_string(&package_path) {
        Ok(s) => s.into(),
        Err(e) => {
            let err_json = format!(r#"{{"status":"error","message":"Failed to read package_path: {e}"}}"#);
            return env.new_string(err_json).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut());
        }
    };

    let c_dir: String = match env.get_string(&cache_dir) {
        Ok(s) => s.into(),
        Err(e) => {
            let err_json = format!(r#"{{"status":"error","message":"Failed to read cache_dir: {e}"}}"#);
            return env.new_string(err_json).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut());
        }
    };

    let result_json = get_bridge().import_package(Path::new(&pkg_path), Path::new(&c_dir));

    env.new_string(result_json)
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

/// JNI entrypoint: Attaches Android ANativeWindow surface and initializes hardware EGL.
#[no_mangle]
pub unsafe extern "system" fn Java_com_reloaded_android_bridge_NativeBridge_nativeAttachSurface(
    env: JNIEnv,
    _class: JClass,
    surface: JObject,
    width: jint,
    height: jint,
) {
    let raw_env = env.get_raw();
    let raw_surface = surface.as_raw();
    get_bridge().attach_surface(raw_env, raw_surface, width, height);
}

/// JNI entrypoint: Called when Android creates the GLES drawing surface.
#[no_mangle]
pub unsafe extern "system" fn Java_com_reloaded_android_bridge_NativeBridge_nativeOnSurfaceCreated(
    _env: JNIEnv,
    _class: JClass,
    width: jint,
    height: jint,
) {
    get_bridge().on_surface_created(width, height);
}

/// JNI entrypoint: Called when Android changes drawing surface dimensions.
#[no_mangle]
pub unsafe extern "system" fn Java_com_reloaded_android_bridge_NativeBridge_nativeOnSurfaceChanged(
    _env: JNIEnv,
    _class: JClass,
    width: jint,
    height: jint,
) {
    get_bridge().on_surface_changed(width, height);
}

/// JNI entrypoint: Called when Android destroys drawing surface (app backgrounded).
#[no_mangle]
pub unsafe extern "system" fn Java_com_reloaded_android_bridge_NativeBridge_nativeOnSurfaceDestroyed(
    _env: JNIEnv,
    _class: JClass,
) {
    get_bridge().on_surface_destroyed();
}

/// JNI entrypoint: Steps the frame by `delta_ms` and returns instantaneous FPS.
/// Synchronizes lock-free input mask into engine runtime.
#[no_mangle]
pub unsafe extern "system" fn Java_com_reloaded_android_bridge_NativeBridge_nativeStepFrame(
    _env: JNIEnv,
    _class: JClass,
    delta_ms: jfloat,
) -> jint {
    get_bridge().step_frame(delta_ms)
}

/// JNI entrypoint: 100% lock-free input mask update for zero-latency touch controls.
#[no_mangle]
pub unsafe extern "system" fn Java_com_reloaded_android_bridge_NativeBridge_nativeUpdateInputMask(
    _env: JNIEnv,
    _class: JClass,
    mask: jint,
) {
    get_bridge().update_input_mask(mask as u32);
}

/// JNI entrypoint: 100% lock-free single button state toggle (fetch_or / fetch_and).
#[no_mangle]
pub unsafe extern "system" fn Java_com_reloaded_android_bridge_NativeBridge_nativeSetButtonState(
    _env: JNIEnv,
    _class: JClass,
    button_bit: jint,
    pressed: jboolean,
) {
    get_bridge().set_button_state(button_bit as u32, pressed != JNI_FALSE);
}

/// JNI entrypoint: 100% lock-free reset of input mask and tap latch.
#[no_mangle]
pub unsafe extern "system" fn Java_com_reloaded_android_bridge_NativeBridge_nativeResetInputs(
    _env: JNIEnv,
    _class: JClass,
) {
    get_bridge().reset_inputs();
}

/// JNI entrypoint: Configures simulation speed multiplier (1 or 2).
#[no_mangle]
pub unsafe extern "system" fn Java_com_reloaded_android_bridge_NativeBridge_nativeSetSpeedMultiplier(
    _env: JNIEnv,
    _class: JClass,
    multiplier: jint,
) {
    get_bridge().set_speed_multiplier(multiplier);
}

/// JNI entrypoint: Returns the current GameMaker room speed (FPS target, typically 30).
#[no_mangle]
pub unsafe extern "system" fn Java_com_reloaded_android_bridge_NativeBridge_nativeGetRoomSpeed(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    get_bridge().get_room_speed()
}

/// JNI entrypoint: Pauses the engine and halts audio/render pipelines.
#[no_mangle]
pub unsafe extern "system" fn Java_com_reloaded_android_bridge_NativeBridge_nativePause(
    _env: JNIEnv,
    _class: JClass,
) {
    get_bridge().pause();
}

/// JNI entrypoint: Resumes engine execution.
#[no_mangle]
pub unsafe extern "system" fn Java_com_reloaded_android_bridge_NativeBridge_nativeResume(
    _env: JNIEnv,
    _class: JClass,
) {
    get_bridge().resume();
}

/// JNI entrypoint: Returns engine telemetry as a JSON string.
#[no_mangle]
pub unsafe extern "system" fn Java_com_reloaded_android_bridge_NativeBridge_nativeGetEngineMetricsJson(
    env: JNIEnv,
    _class: JClass,
) -> jstring {
    let metrics_json = get_bridge().get_engine_metrics_json();
    env.new_string(metrics_json)
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

/// JNI entrypoint: Returns list of save slots as a JSON string.
#[no_mangle]
pub unsafe extern "system" fn Java_com_reloaded_android_bridge_NativeBridge_nativeListSavesJson(
    mut env: JNIEnv,
    _class: JClass,
    game_dir: JString,
) -> jstring {
    let dir_str: String = match env.get_string(&game_dir) {
        Ok(s) => s.into(),
        Err(e) => {
            log::error!("nativeListSavesJson: failed to parse game_dir: {e}");
            return env.new_string("[]").map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut());
        }
    };

    let json = get_bridge().list_saves_json(Path::new(&dir_str));
    env.new_string(json)
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

/// JNI entrypoint: Creates a timestamped backup of a designated save slot.
#[no_mangle]
pub unsafe extern "system" fn Java_com_reloaded_android_bridge_NativeBridge_nativeCreateSaveBackup(
    mut env: JNIEnv,
    _class: JClass,
    game_dir: JString,
    slot_name: JString,
) -> jboolean {
    let dir_str: String = match env.get_string(&game_dir) {
        Ok(s) => s.into(),
        Err(e) => {
            log::error!("nativeCreateSaveBackup: failed to parse game_dir: {e}");
            return JNI_FALSE;
        }
    };

    let slot_str: String = match env.get_string(&slot_name) {
        Ok(s) => s.into(),
        Err(e) => {
            log::error!("nativeCreateSaveBackup: failed to parse slot_name: {e}");
            return JNI_FALSE;
        }
    };

    if get_bridge().create_save_backup(Path::new(&dir_str), &slot_str) {
        JNI_TRUE
    } else {
        JNI_FALSE
    }
}

/// JNI entrypoint: Pulls interleaved stereo f32 audio samples for Android AudioTrack / Oboe.
#[no_mangle]
pub unsafe extern "system" fn Java_com_reloaded_android_bridge_NativeBridge_nativePullAudioSamples(
    env: JNIEnv,
    _class: JClass,
    out_buffer: JFloatArray,
) -> jint {
    let len = match env.get_array_length(&out_buffer) {
        Ok(l) => l as usize,
        Err(e) => {
            log::error!("nativePullAudioSamples: get_array_length failed: {e}");
            return 0;
        }
    };

    let mut temp_buf = vec![0.0f32; len];
    let pulled = get_bridge().pull_audio_samples(&mut temp_buf);

    if let Err(e) = env.set_float_array_region(&out_buffer, 0, &temp_buf) {
        log::error!("nativePullAudioSamples: set_float_array_region failed: {e}");
        return 0;
    }

    pulled as jint
}
