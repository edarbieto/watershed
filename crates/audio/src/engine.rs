use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample, StreamConfig};
use rtrb::{Consumer, Producer, RingBuffer};

use crate::mixer::{MAX_BLOCK, Voice};
use crate::{AtomicF32, Song};

/// How long pause and resume fade, in seconds. Avoids clicks.
const FADE_SECS: f32 = 0.01;

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("no audio output device found")]
    NoDevice,
    #[error("audio device error: {0}")]
    Device(#[from] cpal::Error),
    #[error("unsupported sample format {0}")]
    UnsupportedFormat(SampleFormat),
}

/// An output device as shown to the user.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputDevice {
    /// Stable identifier, suitable for saving in the config.
    pub id: String,
    pub name: String,
}

/// Lists the output devices of the default host.
pub fn output_devices() -> Vec<OutputDevice> {
    let host = cpal::default_host();
    let Ok(devices) = host.output_devices() else {
        return Vec::new();
    };
    devices.filter_map(|d| describe(&d)).collect()
}

fn describe(device: &cpal::Device) -> Option<OutputDevice> {
    let id = device.id().ok()?.to_string();
    let name = device
        .description()
        .map(|d| d.name().to_owned())
        .unwrap_or_else(|_| id.clone());
    Some(OutputDevice { id, name })
}

enum Command {
    Load(Box<Voice>),
    Seek(usize),
}

/// State shared between the UI and the audio thread.
#[derive(Default)]
struct Shared {
    playing: AtomicBool,
    /// Playback position in frames, published by the audio thread.
    position: AtomicU64,
    master_gain: AtomicF32,
    /// Set when the stream reports an error, e.g. the device was unplugged.
    failed: AtomicBool,
}

/// Audio output. Owns the cpal stream; dropping the engine stops playback.
pub struct Engine {
    _stream: cpal::Stream,
    commands: Producer<Command>,
    garbage: Consumer<Box<Voice>>,
    shared: Arc<Shared>,
    device: OutputDevice,
    sample_rate: u32,
    song: Option<Arc<Song>>,
}

impl Engine {
    /// Opens the device with the given id, or the default device when `None` or not found.
    pub fn start(device_id: Option<&str>) -> Result<Self, EngineError> {
        let host = cpal::default_host();
        let device = device_id
            .and_then(|id| id.parse().ok())
            .and_then(|id| host.device_by_id(&id))
            .or_else(|| host.default_output_device())
            .ok_or(EngineError::NoDevice)?;
        let info = describe(&device).ok_or(EngineError::NoDevice)?;

        let supported = device.default_output_config()?;
        let format = supported.sample_format();
        let config: StreamConfig = supported.config();
        log::info!(
            "opening {} at {} Hz, {} channels, {format}",
            info.name,
            config.sample_rate,
            config.channels
        );

        let shared = Arc::new(Shared {
            master_gain: AtomicF32::new(1.0),
            ..Default::default()
        });
        let (commands, command_rx) = RingBuffer::new(64);
        let (garbage_tx, garbage) = RingBuffer::new(64);
        let state = CallbackState {
            commands: command_rx,
            garbage: garbage_tx,
            shared: shared.clone(),
            voice: None,
            position: 0,
            fade: 0.0,
            fade_step: 1.0 / (FADE_SECS * config.sample_rate as f32),
            mix: vec![0.0; MAX_BLOCK * 2],
            channels: usize::from(config.channels),
        };

        let stream = match format {
            SampleFormat::F32 => build::<f32>(&device, &config, state, &shared)?,
            SampleFormat::I16 => build::<i16>(&device, &config, state, &shared)?,
            SampleFormat::I32 => build::<i32>(&device, &config, state, &shared)?,
            SampleFormat::U16 => build::<u16>(&device, &config, state, &shared)?,
            other => return Err(EngineError::UnsupportedFormat(other)),
        };
        stream.play()?;

        Ok(Self {
            _stream: stream,
            commands,
            garbage,
            shared,
            device: info,
            sample_rate: config.sample_rate,
            song: None,
        })
    }

    pub fn device(&self) -> &OutputDevice {
        &self.device
    }

    /// The device sample rate. Songs must be resampled to it before loading.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// True if the stream failed, e.g. because the device was disconnected.
    pub fn failed(&self) -> bool {
        self.shared.failed.load(Ordering::Relaxed)
    }

    pub fn song(&self) -> Option<&Arc<Song>> {
        self.song.as_ref()
    }

    /// Replaces the current song and rewinds to the start, paused.
    pub fn load(&mut self, song: Arc<Song>) {
        debug_assert_eq!(song.sample_rate, self.sample_rate);
        self.pause();
        self.send(Command::Load(Box::new(Voice::new(song.clone()))));
        self.shared.position.store(0, Ordering::Relaxed);
        self.song = Some(song);
    }

    pub fn play(&mut self) {
        if let Some(song) = &self.song {
            if self.position() >= song.frames() {
                self.seek(0);
            }
            self.shared.playing.store(true, Ordering::Relaxed);
        }
    }

    pub fn pause(&mut self) {
        self.shared.playing.store(false, Ordering::Relaxed);
    }

    pub fn toggle(&mut self) {
        if self.is_playing() {
            self.pause();
        } else {
            self.play();
        }
    }

    pub fn stop(&mut self) {
        self.pause();
        self.seek(0);
    }

    pub fn is_playing(&self) -> bool {
        self.shared.playing.load(Ordering::Relaxed)
    }

    pub fn seek(&mut self, frame: usize) {
        let frame = frame.min(self.song.as_ref().map_or(0, |s| s.frames()));
        self.send(Command::Seek(frame));
        // Show the new position right away instead of waiting for the next callback.
        self.shared.position.store(frame as u64, Ordering::Relaxed);
    }

    /// Current playback position in frames.
    pub fn position(&self) -> usize {
        self.shared.position.load(Ordering::Relaxed) as usize
    }

    pub fn master_gain(&self) -> f32 {
        self.shared.master_gain.load()
    }

    pub fn set_master_gain(&self, gain: f32) {
        self.shared.master_gain.store(gain.max(0.0));
    }

    /// Drops songs the audio thread is done with. Call regularly from the UI thread.
    pub fn collect_garbage(&mut self) {
        while self.garbage.pop().is_ok() {}
    }

    fn send(&mut self, command: Command) {
        self.collect_garbage();
        if self.commands.push(command).is_err() {
            log::warn!("audio command queue full, command dropped");
        }
    }
}

fn build<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    mut state: CallbackState,
    shared: &Arc<Shared>,
) -> Result<cpal::Stream, EngineError>
where
    T: SizedSample + FromSample<f32>,
{
    let shared = shared.clone();
    let stream = device.build_output_stream(
        *config,
        move |data: &mut [T], _| state.process(data),
        move |err| {
            log::error!("audio stream error: {err}");
            shared.failed.store(true, Ordering::Relaxed);
        },
        None,
    )?;
    Ok(stream)
}

/// Everything the audio callback owns. Nothing in here allocates after construction.
struct CallbackState {
    commands: Consumer<Command>,
    garbage: Producer<Box<Voice>>,
    shared: Arc<Shared>,
    voice: Option<Box<Voice>>,
    position: usize,
    /// Pause/resume fade level, 0.0 (silent) to 1.0 (playing).
    fade: f32,
    fade_step: f32,
    /// Stereo scratch buffer of `MAX_BLOCK` frames.
    mix: Vec<f32>,
    channels: usize,
}

impl CallbackState {
    fn process<T: SizedSample + FromSample<f32>>(&mut self, data: &mut [T]) {
        self.handle_commands();

        let playing = self.shared.playing.load(Ordering::Relaxed);
        let master = self.shared.master_gain.load();
        let channels = self.channels;

        for out in data.chunks_mut(MAX_BLOCK * channels) {
            let frames = out.len() / channels;
            let mix = &mut self.mix[..frames * 2];
            mix.fill(0.0);

            if let Some(voice) = &mut self.voice
                && (playing || self.fade > 0.0)
            {
                voice.mix_into(self.position, mix);
                for frame in mix.as_chunks_mut::<2>().0 {
                    self.fade = if playing {
                        (self.fade + self.fade_step).min(1.0)
                    } else {
                        (self.fade - self.fade_step).max(0.0)
                    };
                    let gain = master * self.fade;
                    frame[0] *= gain;
                    frame[1] *= gain;
                }
                self.position += frames;

                let end = voice.song.frames();
                if self.position >= end {
                    self.position = end;
                    self.fade = 0.0;
                    self.shared.playing.store(false, Ordering::Relaxed);
                }
            }

            for (dst, src) in out.chunks_exact_mut(channels).zip(mix.as_chunks::<2>().0) {
                // Hard clip as a last line of defense; stems can sum above full scale.
                let left = src[0].clamp(-1.0, 1.0);
                let right = src[1].clamp(-1.0, 1.0);
                match dst {
                    [mono] => *mono = T::from_sample((left + right) * 0.5),
                    [l, r, rest @ ..] => {
                        *l = T::from_sample(left);
                        *r = T::from_sample(right);
                        rest.fill(T::EQUILIBRIUM);
                    }
                    [] => {}
                }
            }
        }

        self.shared
            .position
            .store(self.position as u64, Ordering::Relaxed);
    }

    fn handle_commands(&mut self) {
        while let Ok(command) = self.commands.pop() {
            match command {
                Command::Load(voice) => {
                    if let Some(old) = self.voice.replace(voice) {
                        // Hand the old song back so it is freed on the UI thread.
                        // If the queue is full we free it here, which is rare and only a glitch.
                        let _ = self.garbage.push(old);
                    }
                    self.position = 0;
                    self.fade = 0.0;
                }
                Command::Seek(frame) => self.position = frame,
            }
        }
    }
}
