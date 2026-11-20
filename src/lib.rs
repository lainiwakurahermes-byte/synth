use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui, widgets::ParamSlider, EguiState};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

mod envelope;
mod oscillator;

use envelope::{Envelope, EnvelopeSettings};
use oscillator::Oscillator;

/// Fixed output level. A full-scale sawtooth is painfully loud.
const LEVEL: f32 = 0.2;

// Keyboard knobs. Provisional: the first four pots of a Novation Launchkey in its default mode.
const CC_ATTACK: u8 = 21;
const CC_DECAY: u8 = 22;
const CC_SUSTAIN: u8 = 23;
const CC_RELEASE: u8 = 24;

/// Marks a slot in `MidiCcs::pending` that holds no new value.
const NO_VALUE: u8 = u8::MAX;

/// Carries MIDI CC values from the audio thread to the window.
///
/// Only the window may change a parameter, so the audio thread leaves the values here.
struct MidiCcs {
    /// The newest raw value (0-127) per CC number, or `NO_VALUE` once the window has taken it.
    pending: [AtomicU8; 128],
    /// The CC number that arrived last, for the monitor line in the window.
    last_cc: AtomicU8,
    last_value: AtomicU8,
}

pub struct Synth {
    params: Arc<SynthParams>,
    midi_ccs: Arc<MidiCcs>,
    sample_rate: f32,

    oscillator: Oscillator,
    envelope: Envelope,
    /// The key that is held down, if any. One note at a time for now.
    held_note: Option<u8>,
    /// Outlives the key: the note keeps its pitch while the envelope releases.
    frequency: f32,
}

#[derive(Params)]
struct SynthParams {
    #[persist = "editor-state"]
    editor_state: Arc<EguiState>,

    #[id = "attack"]
    attack: FloatParam,
    #[id = "decay"]
    decay: FloatParam,
    #[id = "sustain"]
    sustain: FloatParam,
    #[id = "release"]
    release: FloatParam,
}

/// An envelope time in milliseconds. The skew gives short times most of the knob's travel.
fn time_param(name: &str, default_ms: f32) -> FloatParam {
    FloatParam::new(
        name,
        default_ms,
        FloatRange::Skewed {
            min: 1.0,
            max: 5_000.0,
            factor: FloatRange::skew_factor(-2.0),
        },
    )
    .with_unit(" ms")
    .with_value_to_string(formatters::v2s_f32_rounded(0))
}

impl Default for Synth {
    fn default() -> Self {
        Self {
            params: Arc::new(SynthParams {
                editor_state: EguiState::from_size(360, 240),
                attack: time_param("Attack", 10.0),
                decay: time_param("Decay", 200.0),
                sustain: FloatParam::new("Sustain", 0.7, FloatRange::Linear { min: 0.0, max: 1.0 })
                    .with_value_to_string(formatters::v2s_f32_rounded(2)),
                release: time_param("Release", 300.0),
            }),
            midi_ccs: Arc::new(MidiCcs {
                pending: std::array::from_fn(|_| AtomicU8::new(NO_VALUE)),
                last_cc: AtomicU8::new(NO_VALUE),
                last_value: AtomicU8::new(0),
            }),
            sample_rate: 1.0,
            oscillator: Oscillator::default(),
            envelope: Envelope::default(),
            held_note: None,
            frequency: 0.0,
        }
    }
}

impl Plugin for Synth {
    const NAME: &'static str = "Synth";
    const VENDOR: &'static str = "Learning Synth";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let params = self.params.clone();
        let midi_ccs = self.midi_ccs.clone();

        create_egui_editor(
            self.params.editor_state.clone(),
            (),
            |_, _| {},
            move |egui_ctx, setter, _state| {
                let knobs = [
                    (CC_ATTACK, &params.attack),
                    (CC_DECAY, &params.decay),
                    (CC_SUSTAIN, &params.sustain),
                    (CC_RELEASE, &params.release),
                ];

                // Apply the knob moves that arrived since the last frame.
                for (cc, param) in knobs {
                    let value = midi_ccs.pending[cc as usize].swap(NO_VALUE, Ordering::Relaxed);
                    if value != NO_VALUE {
                        setter.begin_set_parameter(param);
                        setter.set_parameter_normalized(param, value as f32 / 127.0);
                        setter.end_set_parameter(param);
                    }
                }

                egui::CentralPanel::default().show(egui_ctx, |ui| {
                    ui.heading("Synth");
                    ui.label("Step 2: amplitude envelope.");
                    ui.add_space(8.0);

                    egui::Grid::new("envelope").show(ui, |ui| {
                        for (cc, param) in knobs {
                            ui.label(param.name());
                            ui.add(ParamSlider::for_param(param, setter));
                            ui.label(format!("CC {cc}"));
                            ui.end_row();
                        }
                    });

                    ui.add_space(8.0);
                    let last_cc = midi_ccs.last_cc.load(Ordering::Relaxed);
                    if last_cc == NO_VALUE {
                        ui.label("Last MIDI CC: none yet. Turn a knob on the keyboard.");
                    } else {
                        let last_value = midi_ccs.last_value.load(Ordering::Relaxed);
                        ui.label(format!("Last MIDI CC: {last_cc} = {last_value}"));
                    }
                });
            },
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        true
    }

    fn reset(&mut self) {
        self.oscillator.reset();
        self.envelope.reset();
        self.held_note = None;
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let settings = EnvelopeSettings {
            attack: self.params.attack.value() / 1_000.0,
            decay: self.params.decay.value() / 1_000.0,
            sustain: self.params.sustain.value(),
            release: self.params.release.value() / 1_000.0,
        };

        let mut next_event = context.next_event();

        for (sample_index, channel_samples) in buffer.iter_samples().enumerate() {
            // Events carry the sample offset inside this buffer at which they happen.
            while let Some(event) = next_event {
                if event.timing() > sample_index as u32 {
                    break;
                }
                match event {
                    NoteEvent::NoteOn { note, .. } => {
                        self.held_note = Some(note);
                        self.frequency = util::midi_note_to_freq(note);
                        self.envelope.gate_on();
                    }
                    // Releasing an older key must not silence the key played after it.
                    NoteEvent::NoteOff { note, .. } if self.held_note == Some(note) => {
                        self.held_note = None;
                        self.envelope.gate_off();
                    }
                    NoteEvent::MidiCC { cc, value, .. } => {
                        let raw = (value * 127.0).round() as u8;
                        self.midi_ccs.pending[cc as usize].store(raw, Ordering::Relaxed);
                        self.midi_ccs.last_cc.store(cc, Ordering::Relaxed);
                        self.midi_ccs.last_value.store(raw, Ordering::Relaxed);
                    }
                    _ => (),
                }
                next_event = context.next_event();
            }

            let gain = self.envelope.next(&settings, self.sample_rate);
            let sample = self.oscillator.next(self.frequency, self.sample_rate) * gain * LEVEL;

            for channel_sample in channel_samples {
                *channel_sample = sample;
            }
        }

        ProcessStatus::KeepAlive
    }
}

impl ClapPlugin for Synth {
    const CLAP_ID: &'static str = "local.learning.synth";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A learning polysynth");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
    ];
}

impl Vst3Plugin for Synth {
    const VST3_CLASS_ID: [u8; 16] = *b"LearningSynth001";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}

nih_export_clap!(Synth);
nih_export_vst3!(Synth);
