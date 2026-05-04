use std::f32::consts::TAU;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream};

use crate::app::{NoiseTexture, Waveform};

#[derive(Debug)]
pub struct SharedSynthState {
    pub frequency_hz: f32,
    pub gain: f32,
    pub cutoff_hz: f32,
    pub waveform: Waveform,
    pub noise_texture: NoiseTexture,
    pub noise_amount: f32,
    pub gate_open: bool,
    pub pluck_level: f32,
}

impl Default for SharedSynthState {
    fn default() -> Self {
        Self {
            frequency_hz: 261.63,
            gain: 0.22,
            cutoff_hz: 1_800.0,
            waveform: Waveform::Sine,
            noise_texture: NoiseTexture::None,
            noise_amount: 0.18,
            gate_open: false,
            pluck_level: 0.0,
        }
    }
}

#[derive(Clone)]
pub struct AudioControl {
    shared_state: Arc<Mutex<SharedSynthState>>,
    status: String,
}

impl AudioControl {
    pub fn silent(status: impl Into<String>) -> Self {
        Self {
            shared_state: Arc::new(Mutex::new(SharedSynthState::default())),
            status: status.into(),
        }
    }

    pub fn shared_state(&self) -> Arc<Mutex<SharedSynthState>> {
        Arc::clone(&self.shared_state)
    }

    pub fn trigger_pluck(&self) {
        if let Ok(mut state) = self.shared_state.lock() {
            state.pluck_level = 1.0;
        }
    }

    pub fn status(&self) -> &str {
        &self.status
    }
}

pub struct AudioEngine {
    _stream: Option<Stream>,
    shared_state: Arc<Mutex<SharedSynthState>>,
    status: String,
}

impl AudioEngine {
    pub fn new() -> Self {
        let shared_state = Arc::new(Mutex::new(SharedSynthState::default()));
        match start_stream(&shared_state) {
            Ok(stream) => Self {
                _stream: Some(stream),
                shared_state,
                status: "audio: live".to_string(),
            },
            Err(err) => Self {
                _stream: None,
                shared_state,
                status: format!("audio: silent ({err})"),
            },
        }
    }

    pub fn control(&self) -> AudioControl {
        AudioControl {
            shared_state: Arc::clone(&self.shared_state),
            status: self.status.clone(),
        }
    }
}

fn start_stream(shared_state: &Arc<Mutex<SharedSynthState>>) -> Result<Stream> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| anyhow!("no default audio output device found"))?;
    let config = device.default_output_config()?;

    let stream = match config.sample_format() {
        SampleFormat::F32 => build_stream::<f32>(&device, &config.into(), shared_state)?,
        SampleFormat::I16 => build_stream::<i16>(&device, &config.into(), shared_state)?,
        SampleFormat::U16 => build_stream::<u16>(&device, &config.into(), shared_state)?,
        format => return Err(anyhow!("unsupported audio sample format: {format:?}")),
    };
    stream.play()?;

    Ok(stream)
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    shared_state: &Arc<Mutex<SharedSynthState>>,
) -> Result<Stream>
where
    T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32>,
{
    let sample_rate = config.sample_rate.0 as f32;
    let channels = config.channels as usize;
    let mut phase = 0.0_f32;
    let mut texture_phase = 0.0_f32;
    let mut filter = 0.0_f32;
    let mut envelope = 0.0_f32;
    let mut brown = 0.0_f32;
    let mut wind = 0.0_f32;
    let mut rain_filter = 0.0_f32;
    let shared_state = Arc::clone(shared_state);

    let stream = device.build_output_stream(
        config,
        move |output: &mut [T], _| {
            let mut state = shared_state.lock().expect("audio state poisoned");
            let cutoff = state.cutoff_hz.clamp(80.0, sample_rate * 0.45);
            let alpha = (TAU * cutoff / sample_rate).clamp(0.0, 1.0);
            let frequency = state.frequency_hz.clamp(20.0, sample_rate * 0.45);
            let gain = state.gain;
            let waveform = state.waveform;
            let noise_texture = state.noise_texture;
            let noise_amount = state.noise_amount;
            let gate_open = state.gate_open;
            let mut pluck = state.pluck_level;

            for frame in output.chunks_mut(channels) {
                let target = if gate_open { 1.0 } else { 0.0 };
                envelope += (target - envelope) * if gate_open { 0.004 } else { 0.0016 };
                pluck *= 0.994;

                phase = (phase + frequency / sample_rate).fract();
                let dry = oscillator(waveform, phase);
                filter += alpha * (dry - filter);
                let click = (random_unit() * 0.22) * pluck;
                let texture = noise_sample(
                    noise_texture,
                    &mut texture_phase,
                    &mut brown,
                    &mut wind,
                    &mut rain_filter,
                    sample_rate,
                );
                let sample = (((filter * envelope) + click) * gain + texture * noise_amount)
                    .clamp(-0.95, 0.95);

                for channel in frame {
                    *channel = T::from_sample(sample);
                }
            }

            state.pluck_level = pluck;
        },
        move |err| eprintln!("audio stream error: {err}"),
        None,
    )?;

    Ok(stream)
}

fn oscillator(waveform: Waveform, phase: f32) -> f32 {
    match waveform {
        Waveform::Sine => (phase * TAU).sin(),
        Waveform::Square => {
            if phase < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
        Waveform::Saw => (phase * 2.0) - 1.0,
        Waveform::Triangle => 1.0 - (4.0 * (phase - 0.5).abs()),
    }
}

fn noise_sample(
    texture: NoiseTexture,
    texture_phase: &mut f32,
    brown: &mut f32,
    wind: &mut f32,
    rain_filter: &mut f32,
    sample_rate: f32,
) -> f32 {
    match texture {
        NoiseTexture::None => 0.0,
        NoiseTexture::White => random_unit() * 0.34,
        NoiseTexture::Brown => {
            *brown = (*brown + random_unit() * 0.022).clamp(-1.0, 1.0);
            *brown * 0.48
        }
        NoiseTexture::Rain => {
            let white = random_unit();
            *rain_filter += (white - *rain_filter) * 0.025;
            let hiss = (white - *rain_filter) * 0.24;
            let drop = if rand::random::<f32>() > 0.996 {
                random_unit().abs() * 0.9
            } else {
                0.0
            };
            hiss + drop
        }
        NoiseTexture::Wind => {
            *texture_phase = (*texture_phase + 0.055 / sample_rate).fract();
            let gust = ((*texture_phase * TAU).sin() + 1.0) * 0.5;
            *wind += (random_unit() - *wind) * 0.0018;
            *wind * (0.38 + gust * 0.55)
        }
    }
}

fn random_unit() -> f32 {
    rand::random::<f32>() * 2.0 - 1.0
}
