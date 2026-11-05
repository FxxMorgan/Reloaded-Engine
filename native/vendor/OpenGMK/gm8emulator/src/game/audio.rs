pub mod midi;

use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
};

pub use self::midi::MidiHandle;

#[derive(Clone, Serialize, Deserialize)]
pub struct Mp3Handle {
    pub id: i32,
    pub length_ns: u128,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct WavHandle {
    pub params: Arc<SoundParams>,
    pub exclusive: bool,
    pub id: i32,
    pub length_ns: u128,
}

#[derive(Serialize, Deserialize)]
pub struct SoundParams {
    pub volume: AtomicU32,
}

impl WavHandle {
    pub fn set_volume(&self, vol: f64) {
        self.params.volume.store(make_volume(vol).to_bits(), Ordering::Release);
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AudioState {
    global_volume: Arc<AtomicU32>,
    end_times: HashMap<i32, Option<u128>>,
    multimedia_end: Option<(i32, Option<u128>)>,
}

pub struct AudioManager {
    pub do_output: bool,
    global_volume: Arc<AtomicU32>,
    end_times: HashMap<i32, Option<u128>>,
    multimedia_end: Option<(i32, Option<u128>)>,
}

impl AudioManager {
    pub fn new(do_output: bool, _capture_audio: bool) -> Self {
        Self {
            do_output,
            global_volume: Arc::new(AtomicU32::from(1.0f32.to_bits())),
            end_times: HashMap::new(),
            multimedia_end: None,
        }
    }

    pub fn capture_audio(&mut self) {}

    pub fn stop_audio_capture(&mut self) {}

    pub fn add_mp3(&mut self, file: Box<[u8]>, sound_id: i32) -> Option<Mp3Handle> {
        let approx_ns = (file.len() as u128 * 8 * 1_000_000_000) / 128_000;
        Some(Mp3Handle {
            id: sound_id,
            length_ns: approx_ns.max(1_000_000),
        })
    }

    pub fn add_wav(
        &mut self,
        file: Box<[u8]>,
        sound_id: i32,
        volume: f64,
        _use_3d: bool,
        exclusive: bool,
    ) -> Option<WavHandle> {
        let approx_ns = (file.len().saturating_sub(44) as u128 * 1_000_000_000) / 176_400;
        Some(WavHandle {
            params: Arc::new(SoundParams { volume: AtomicU32::new(make_volume(volume).to_bits()) }),
            exclusive,
            id: sound_id,
            length_ns: approx_ns.max(100_000),
        })
    }

    pub fn add_midi(&mut self, data: Box<[u8]>, sound_id: i32, volume: f64, exclusive: bool) -> Option<MidiHandle> {
        MidiHandle::new(data, sound_id, volume, exclusive)
    }

    pub fn play_mp3(&mut self, handle: &Mp3Handle, start_time: u128) {
        let end_time = handle.length_ns + start_time;
        self.multimedia_end = Some((handle.id, Some(end_time)));
    }

    pub fn play_wav(&mut self, handle: &WavHandle, start_time: u128) {
        let end_time = handle.length_ns + start_time;
        if handle.exclusive {
            self.multimedia_end = Some((handle.id, Some(end_time)));
        } else if self.end_times.get(&handle.id) != Some(&None) {
            self.end_times.insert(handle.id, Some(end_time));
        }
    }

    pub fn play_midi(&mut self, handle: &MidiHandle, start_time: u128) {
        let approx_ns = (handle.length() as u128 * 1_000_000_000) / 44100;
        let end_time = approx_ns + start_time;
        if handle.exclusive {
            self.multimedia_end = Some((handle.id, Some(end_time)));
        } else if self.end_times.get(&handle.id) != Some(&None) {
            self.end_times.insert(handle.id, Some(end_time));
        }
    }

    pub fn loop_mp3(&mut self, handle: &Mp3Handle) {
        self.multimedia_end = Some((handle.id, None));
    }

    pub fn loop_wav(&mut self, handle: &WavHandle) {
        if handle.exclusive {
            self.multimedia_end = Some((handle.id, None));
        } else {
            self.end_times.insert(handle.id, None);
        }
    }

    pub fn loop_midi(&mut self, handle: &MidiHandle) {
        if handle.exclusive {
            self.multimedia_end = Some((handle.id, None));
        } else {
            self.end_times.insert(handle.id, None);
        }
    }

    pub fn stop_sound(&mut self, id: i32) {
        self.end_times.remove(&id);
        if self.multimedia_end.map(|(x, _)| x) == Some(id) {
            self.multimedia_end = None;
        }
    }

    pub fn stop_all(&mut self) {
        self.end_times.clear();
        self.multimedia_end = None;
    }

    pub fn set_global_volume(&self, vol: f64) {
        self.global_volume.store(make_volume(vol).to_bits(), Ordering::Release);
    }

    pub fn sound_playing(&self, sound_id: i32, current_time: u128) -> bool {
        self.mp3_playing(sound_id, current_time) || self.wav_playing(sound_id, current_time)
    }

    fn mp3_playing(&self, sound_id: i32, current_time: u128) -> bool {
        self.multimedia_end
            .map(|(id, end_time)| id == sound_id && end_time.map(|x| x > current_time).unwrap_or(true))
            .unwrap_or(false)
    }

    fn wav_playing(&self, sound_id: i32, current_time: u128) -> bool {
        match self.end_times.get(&sound_id) {
            Some(&Some(x)) => x > current_time,
            Some(None) => true,
            None => false,
        }
    }

    pub fn state(&self) -> AudioState {
        AudioState {
            global_volume: self.global_volume.clone(),
            end_times: self.end_times.clone(),
            multimedia_end: self.multimedia_end,
        }
    }

    pub fn set_state(&mut self, state: AudioState) {
        self.global_volume = state.global_volume;
        self.end_times = state.end_times;
        self.multimedia_end = state.multimedia_end;
    }
}

fn make_volume(vol: f64) -> f32 {
    1000.0f64.powf(vol.clamp(0.0, 1.0) - 1.0) as f32
}
