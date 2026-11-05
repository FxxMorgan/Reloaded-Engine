use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

/// Single-Producer Single-Consumer (SPSC) lock-free ring buffer for 32-bit float audio samples.
/// Implemented entirely with safe Rust using atomic operations.
pub struct SpscAudioRingBuffer {
    buffer: Vec<AtomicU32>,
    capacity: usize,
    mask: usize,
    write_idx: AtomicUsize,
    read_idx: AtomicUsize,
}

impl SpscAudioRingBuffer {
    /// Creates a new `SpscAudioRingBuffer` with the capacity rounded up to the next power of two.
    pub fn new(capacity: usize) -> Self {
        let cap = capacity.max(64).next_power_of_two();
        let mut buffer = Vec::with_capacity(cap);
        for _ in 0..cap {
            buffer.push(AtomicU32::new(0));
        }

        Self {
            buffer,
            capacity: cap,
            mask: cap - 1,
            write_idx: AtomicUsize::new(0),
            read_idx: AtomicUsize::new(0),
        }
    }

    /// Total capacity of the buffer.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Number of samples available to read.
    pub fn available_read(&self) -> usize {
        let r = self.read_idx.load(Ordering::Relaxed);
        let w = self.write_idx.load(Ordering::Acquire);
        w.saturating_sub(r)
    }

    /// Number of empty sample slots available to write.
    pub fn available_write(&self) -> usize {
        let r = self.read_idx.load(Ordering::Acquire);
        let w = self.write_idx.load(Ordering::Relaxed);
        self.capacity.saturating_sub(w.saturating_sub(r))
    }

    /// Returns true if there are no samples available to read.
    pub fn is_empty(&self) -> bool {
        self.available_read() == 0
    }

    /// Returns true if the buffer has no space for more samples.
    pub fn is_full(&self) -> bool {
        self.available_write() == 0
    }

    /// Pushes a single sample into the buffer. Returns false if the buffer is full.
    pub fn push(&self, sample: f32) -> bool {
        let w = self.write_idx.load(Ordering::Relaxed);
        let r = self.read_idx.load(Ordering::Acquire);

        if w.saturating_sub(r) >= self.capacity {
            return false;
        }

        let idx = w & self.mask;
        self.buffer[idx].store(sample.to_bits(), Ordering::Relaxed);
        self.write_idx.store(w + 1, Ordering::Release);
        true
    }

    /// Pops a single sample from the buffer. Returns None if the buffer is empty.
    pub fn pop(&self) -> Option<f32> {
        let r = self.read_idx.load(Ordering::Relaxed);
        let w = self.write_idx.load(Ordering::Acquire);

        if r >= w {
            return None;
        }

        let idx = r & self.mask;
        let bits = self.buffer[idx].load(Ordering::Relaxed);
        self.read_idx.store(r + 1, Ordering::Release);
        Some(f32::from_bits(bits))
    }

    /// Pushes a slice of samples into the buffer. Returns the number of samples pushed.
    pub fn push_slice(&self, samples: &[f32]) -> usize {
        let r = self.read_idx.load(Ordering::Acquire);
        let mut w = self.write_idx.load(Ordering::Relaxed);
        let free = self.capacity.saturating_sub(w.saturating_sub(r));
        let count = samples.len().min(free);

        for &sample in &samples[..count] {
            let idx = w & self.mask;
            self.buffer[idx].store(sample.to_bits(), Ordering::Relaxed);
            w += 1;
        }

        if count > 0 {
            self.write_idx.store(w, Ordering::Release);
        }

        count
    }

    /// Pops samples into the destination slice. Returns the number of samples written.
    pub fn pop_slice(&self, out: &mut [f32]) -> usize {
        let w = self.write_idx.load(Ordering::Acquire);
        let mut r = self.read_idx.load(Ordering::Relaxed);
        let avail = w.saturating_sub(r);
        let count = out.len().min(avail);

        for slot in &mut out[..count] {
            let idx = r & self.mask;
            let bits = self.buffer[idx].load(Ordering::Relaxed);
            *slot = f32::from_bits(bits);
            r += 1;
        }

        if count > 0 {
            self.read_idx.store(r, Ordering::Release);
        }

        count
    }

    /// Clears the ring buffer.
    pub fn clear(&self) {
        let w = self.write_idx.load(Ordering::Acquire);
        self.read_idx.store(w, Ordering::Release);
    }
}

/// Represents background music currently playing.
#[derive(Debug, Clone)]
pub struct BgmTrack {
    pub samples: Vec<f32>, // stereo interleaved [L0, R0, L1, R1, ...]
    pub position: usize,
    pub looping: bool,
    pub volume: f32,
    pub playing: bool,
}

/// Active sound effect voice channel.
#[derive(Debug, Clone)]
pub struct SfxVoice {
    pub id: u64,
    pub samples: Vec<f32>, // stereo interleaved [L0, R0, L1, R1, ...]
    pub position: usize,
    pub volume: f32,
    pub active: bool,
}

/// Multichannel lock-free audio mixer.
pub struct LockFreeAudioMixer {
    stream_ring_buffer: SpscAudioRingBuffer,
    bgm: Option<BgmTrack>,
    sfx_voices: Vec<SfxVoice>,
    master_volume: f32,
    bgm_volume: f32,
    sfx_volume: f32,
    paused: bool,
    next_sfx_id: u64,
}

impl LockFreeAudioMixer {
    /// Creates a new audio mixer with the given SPSC stream buffer size.
    pub fn new(stream_capacity: usize) -> Self {
        Self {
            stream_ring_buffer: SpscAudioRingBuffer::new(stream_capacity),
            bgm: None,
            sfx_voices: Vec::with_capacity(32),
            master_volume: 1.0,
            bgm_volume: 1.0,
            sfx_volume: 1.0,
            paused: false,
            next_sfx_id: 1,
        }
    }

    /// Reference to the underlying SPSC ring buffer for streaming sound feeds.
    pub fn ring_buffer(&self) -> &SpscAudioRingBuffer {
        &self.stream_ring_buffer
    }

    pub fn set_master_volume(&mut self, vol: f32) {
        self.master_volume = vol.clamp(0.0, 1.0);
    }

    pub fn get_master_volume(&self) -> f32 {
        self.master_volume
    }

    pub fn set_bgm_volume(&mut self, vol: f32) {
        self.bgm_volume = vol.clamp(0.0, 1.0);
    }

    pub fn get_bgm_volume(&self) -> f32 {
        self.bgm_volume
    }

    pub fn set_sfx_volume(&mut self, vol: f32) {
        self.sfx_volume = vol.clamp(0.0, 1.0);
    }

    pub fn get_sfx_volume(&self) -> f32 {
        self.sfx_volume
    }

    pub fn pause(&mut self) {
        self.paused = true;
    }

    pub fn resume(&mut self) {
        self.paused = false;
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Plays a background music track.
    pub fn play_bgm(&mut self, interleaved_stereo: Vec<f32>, looping: bool, volume: f32) {
        self.bgm = Some(BgmTrack {
            samples: interleaved_stereo,
            position: 0,
            looping,
            volume: volume.clamp(0.0, 1.0),
            playing: true,
        });
    }

    /// Stops BGM playback.
    pub fn stop_bgm(&mut self) {
        self.bgm = None;
    }

    /// Plays a one-shot sound effect. Returns a unique voice ID.
    pub fn play_sfx(&mut self, interleaved_stereo: Vec<f32>, volume: f32) -> u64 {
        let id = self.next_sfx_id;
        self.next_sfx_id = self.next_sfx_id.wrapping_add(1);

        self.sfx_voices.push(SfxVoice {
            id,
            samples: interleaved_stereo,
            position: 0,
            volume: volume.clamp(0.0, 1.0),
            active: true,
        });

        id
    }

    /// Stops a specific sound effect voice.
    pub fn stop_sfx(&mut self, voice_id: u64) {
        if let Some(voice) = self.sfx_voices.iter_mut().find(|v| v.id == voice_id) {
            voice.active = false;
        }
    }

    /// Stops all audio playback.
    pub fn stop_all(&mut self) {
        self.bgm = None;
        self.sfx_voices.clear();
        self.stream_ring_buffer.clear();
    }

    /// Renders interleaved stereo audio frames `[L0, R0, L1, R1, ...]`.
    /// Performs multichannel mixing, volume scaling, looping, and master clamping.
    pub fn render_audio_frames(&mut self, out_interleaved_stereo: &mut [f32]) {
        // If paused, output silence
        if self.paused || out_interleaved_stereo.is_empty() {
            out_interleaved_stereo.fill(0.0);
            return;
        }

        out_interleaved_stereo.fill(0.0);

        // 1. Mix from SPSC streaming ring buffer
        let ring_read_count = self.stream_ring_buffer.pop_slice(out_interleaved_stereo);
        let master = self.master_volume;

        if ring_read_count > 0 && master != 1.0 {
            for sample in &mut out_interleaved_stereo[..ring_read_count] {
                *sample *= master;
            }
        }

        // 2. Mix BGM
        if let Some(ref mut bgm) = self.bgm {
            if bgm.playing && !bgm.samples.is_empty() {
                let bgm_gain = bgm.volume * self.bgm_volume * master;
                let num_samples = out_interleaved_stereo.len();
                let mut out_idx = 0;

                while out_idx < num_samples {
                    if bgm.position >= bgm.samples.len() {
                        if bgm.looping {
                            bgm.position = 0;
                        } else {
                            bgm.playing = false;
                            break;
                        }
                    }

                    out_interleaved_stereo[out_idx] += bgm.samples[bgm.position] * bgm_gain;
                    bgm.position += 1;
                    out_idx += 1;
                }
            }
        }

        // 3. Mix SFX Voices
        let sfx_master_gain = self.sfx_volume * master;
        for voice in &mut self.sfx_voices {
            if !voice.active || voice.samples.is_empty() {
                continue;
            }

            let voice_gain = voice.volume * sfx_master_gain;
            let needed = out_interleaved_stereo.len();
            let remaining = voice.samples.len() - voice.position;
            let mix_count = needed.min(remaining);

            for i in 0..mix_count {
                out_interleaved_stereo[i] += voice.samples[voice.position + i] * voice_gain;
            }

            voice.position += mix_count;
            if voice.position >= voice.samples.len() {
                voice.active = false;
            }
        }

        // Retain only active SFX voices
        self.sfx_voices.retain(|v| v.active);

        // 4. Clamping to [-1.0, 1.0]
        for sample in out_interleaved_stereo.iter_mut() {
            *sample = sample.clamp(-1.0, 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spsc_ring_buffer_push_pop() {
        let buffer = SpscAudioRingBuffer::new(128);
        assert_eq!(buffer.capacity(), 128);
        assert!(buffer.is_empty());
        assert_eq!(buffer.available_write(), 128);

        // Push slice
        let input: Vec<f32> = (0..64).map(|i| i as f32 * 0.01).collect();
        let pushed = buffer.push_slice(&input);
        assert_eq!(pushed, 64);
        assert_eq!(buffer.available_read(), 64);

        // Pop slice
        let mut output = vec![0.0f32; 64];
        let popped = buffer.pop_slice(&mut output);
        assert_eq!(popped, 64);
        assert_eq!(input, output);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_multichannel_audio_mixing() {
        let mut mixer = LockFreeAudioMixer::new(512);

        // Play BGM with constant 0.2 amplitude
        let bgm_data = vec![0.2; 100];
        mixer.play_bgm(bgm_data, true, 1.0);

        // Play SFX with constant 0.3 amplitude
        let sfx_data = vec![0.3; 50];
        mixer.play_sfx(sfx_data, 1.0);

        let mut output = vec![0.0f32; 10];
        mixer.render_audio_frames(&mut output);

        // Both BGM and SFX mixed: 0.2 + 0.3 = 0.5
        for &sample in &output {
            assert!((sample - 0.5).abs() < 1e-4, "Sample was: {}", sample);
        }

        // Test pausing
        mixer.pause();
        assert!(mixer.is_paused());
        mixer.render_audio_frames(&mut output);
        for &sample in &output {
            assert_eq!(sample, 0.0);
        }

        // Resume and verify audio resumes
        mixer.resume();
        assert!(!mixer.is_paused());
        mixer.render_audio_frames(&mut output);
        for &sample in &output {
            assert!(sample > 0.0);
        }
    }

    #[test]
    fn test_audio_clamping() {
        let mut mixer = LockFreeAudioMixer::new(512);
        mixer.play_bgm(vec![0.8; 100], true, 1.0);
        mixer.play_sfx(vec![0.8; 100], 1.0);

        let mut output = vec![0.0f32; 10];
        mixer.render_audio_frames(&mut output);

        for &sample in &output {
            assert_eq!(sample, 1.0, "Audio should be clamped to 1.0");
        }
    }
}
