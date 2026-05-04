use std::sync::{Arc, Mutex};

use crate::audio::{AudioControl, SharedSynthState};
use crate::engine::{ParticleEngine, ParticleSnapshot};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Waveform {
    Sine,
    Square,
    Saw,
    Triangle,
}

impl Waveform {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sine => "sine",
            Self::Square => "square",
            Self::Saw => "saw",
            Self::Triangle => "triangle",
        }
    }

    pub fn next_color_index(self) -> usize {
        match self {
            Self::Sine => 0,
            Self::Square => 1,
            Self::Saw => 2,
            Self::Triangle => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NoiseTexture {
    None,
    White,
    Brown,
    Rain,
    Wind,
}

impl NoiseTexture {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "off",
            Self::White => "white",
            Self::Brown => "brown",
            Self::Rain => "rain",
            Self::Wind => "wind",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VisualMode {
    TwoD,
    ThreeD,
}

impl VisualMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TwoD => "2d",
            Self::ThreeD => "3d",
        }
    }
}

pub struct App {
    pub should_quit: bool,
    pub octave: i32,
    pub root_midi: i32,
    pub waveform: Waveform,
    pub noise_texture: NoiseTexture,
    pub noise_amount: f32,
    pub gain: f32,
    pub cutoff_hz: f32,
    pub gate_open: bool,
    pub last_note: String,
    pub particles: Vec<ParticleSnapshot>,
    pub meters: [f32; 48],
    pub audio_status: String,
    pub visual_phase: f32,
    pub visual_mode: VisualMode,
    engine: ParticleEngine,
    audio: AudioControl,
    shared_state: Arc<Mutex<SharedSynthState>>,
    meter_phase: f32,
}

impl App {
    pub fn new(audio: AudioControl) -> Self {
        let shared_state = audio.shared_state();
        let app = Self {
            should_quit: false,
            octave: 4,
            root_midi: 60,
            waveform: Waveform::Sine,
            noise_texture: NoiseTexture::Rain,
            noise_amount: 0.3,
            gain: 0.22,
            cutoff_hz: 1_800.0,
            gate_open: false,
            last_note: "C4".to_string(),
            particles: Vec::new(),
            meters: [0.0; 48],
            audio_status: audio.status().to_string(),
            visual_phase: 0.0,
            visual_mode: VisualMode::TwoD,
            engine: ParticleEngine::default(),
            audio,
            shared_state,
            meter_phase: 0.0,
        };
        app.sync_audio();
        app
    }

    pub fn tick(&mut self, dt: f32) {
        self.visual_phase = (self.visual_phase + dt) % 1_000.0;
        self.engine.update(dt);
        self.particles = self.engine.snapshot();
        self.update_meters(dt);
    }

    pub fn toggle_gate(&mut self) {
        self.gate_open = !self.gate_open;
        self.sync_audio();
        if self.gate_open {
            self.engine.burst(self.waveform.next_color_index(), 36);
        }
    }

    pub fn set_waveform(&mut self, waveform: Waveform) {
        self.waveform = waveform;
        self.sync_audio();
        self.engine.burst(self.waveform.next_color_index(), 28);
    }

    pub fn set_noise_texture(&mut self, noise_texture: NoiseTexture) {
        self.noise_texture = noise_texture;
        self.sync_audio();
        self.engine.burst(self.texture_color_index(), 24);
    }

    pub fn shift_note(&mut self, semitones: i32) {
        let next = (self.root_midi + semitones).clamp(24, 96);
        self.set_midi_note(next);
    }

    pub fn play_note(&mut self, semitone: i32) {
        self.set_midi_note(12 * (self.octave + 1) + semitone);
        self.gate_open = true;
        self.sync_audio();
        self.audio.trigger_pluck();
        self.engine.burst(self.waveform.next_color_index(), 40);
    }

    pub fn adjust_gain(&mut self, delta: f32) {
        self.gain = (self.gain + delta).clamp(0.0, 0.8);
        self.sync_audio();
    }

    pub fn adjust_noise_amount(&mut self, delta: f32) {
        self.noise_amount = (self.noise_amount + delta).clamp(0.0, 0.6);
        self.sync_audio();
    }

    pub fn adjust_cutoff(&mut self, delta: f32) {
        self.cutoff_hz = (self.cutoff_hz + delta).clamp(120.0, 8_000.0);
        self.sync_audio();
    }

    pub fn toggle_visual_mode(&mut self) {
        self.visual_mode = match self.visual_mode {
            VisualMode::TwoD => VisualMode::ThreeD,
            VisualMode::ThreeD => VisualMode::TwoD,
        };
        self.engine.burst(self.texture_color_index(), 26);
    }

    fn set_midi_note(&mut self, midi: i32) {
        self.root_midi = midi.clamp(24, 96);
        self.octave = self.root_midi / 12 - 1;
        self.last_note = midi_to_note_name(self.root_midi);
        self.sync_audio();
    }

    fn sync_audio(&self) {
        let mut state = self.shared_state.lock().expect("audio state poisoned");
        state.frequency_hz = midi_to_frequency(self.root_midi);
        state.gain = self.gain;
        state.cutoff_hz = self.cutoff_hz;
        state.waveform = self.waveform;
        state.noise_texture = self.noise_texture;
        state.noise_amount = self.noise_amount;
        state.gate_open = self.gate_open;
    }

    fn update_meters(&mut self, dt: f32) {
        self.meter_phase += dt * self.audio_frequency().min(900.0) * 0.012;
        let ambient_energy = if self.noise_texture == NoiseTexture::None {
            0.0
        } else {
            self.noise_amount * 0.45
        };
        let energy = if self.gate_open {
            self.gain.max(0.08)
        } else {
            0.025 + ambient_energy
        };

        for (index, meter) in self.meters.iter_mut().enumerate() {
            let x = index as f32 / 48.0;
            let wave = match self.waveform {
                Waveform::Sine => (self.meter_phase + x * 6.283).sin(),
                Waveform::Square => {
                    if (self.meter_phase + x * 6.283).sin() >= 0.0 {
                        1.0
                    } else {
                        -1.0
                    }
                }
                Waveform::Saw => ((self.meter_phase + x * 4.0).fract() * 2.0) - 1.0,
                Waveform::Triangle => {
                    let saw = (self.meter_phase + x * 4.0).fract();
                    4.0 * (saw - 0.5).abs() - 1.0
                }
            };
            *meter = (*meter * 0.72) + ((wave.abs() * energy) * 0.28);
        }
    }

    pub fn audio_frequency(&self) -> f32 {
        midi_to_frequency(self.root_midi)
    }

    pub fn texture_color_index(&self) -> usize {
        match self.noise_texture {
            NoiseTexture::None => self.waveform.next_color_index(),
            NoiseTexture::White => 0,
            NoiseTexture::Brown => 2,
            NoiseTexture::Rain => 3,
            NoiseTexture::Wind => 1,
        }
    }
}

fn midi_to_frequency(midi: i32) -> f32 {
    440.0 * 2.0_f32.powf((midi as f32 - 69.0) / 12.0)
}

fn midi_to_note_name(midi: i32) -> String {
    const NOTES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    format!("{}{}", NOTES[midi.rem_euclid(12) as usize], midi / 12 - 1)
}
