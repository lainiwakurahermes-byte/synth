use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui, EguiState};
use std::sync::Arc;

mod oscillator;

use oscillator::Oscillator;

/// Fixed output level. A full-scale sawtooth is painfully loud.
const LEVEL: f32 = 0.2;

pub struct Synth {
    params: Arc<SynthParams>,
    sample_rate: f32,

    oscillator: Oscillator,
    /// The MIDI note that is held down, if any. One note at a time for now.
    note: Option<u8>,
}

#[derive(Params)]
struct SynthParams {
    #[persist = "editor-state"]
    editor_state: Arc<EguiState>,
}

impl Default for Synth {
    fn default() -> Self {
        Self {
            params: Arc::new(SynthParams {
                editor_state: EguiState::from_size(360, 160),
            }),
            sample_rate: 1.0,
            oscillator: Oscillator::default(),
            note: None,
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

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        create_egui_editor(
            self.params.editor_state.clone(),
            (),
            |_, _| {},
            |egui_ctx, _setter, _state| {
                egui::CentralPanel::default().show(egui_ctx, |ui| {
                    ui.heading("Synth");
                    ui.label("Step 1: one sawtooth oscillator, one note at a time.");
                    ui.label("Play the keyboard. Close this window to quit.");
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
        self.note = None;
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let mut next_event = context.next_event();

        for (sample_index, channel_samples) in buffer.iter_samples().enumerate() {
            // Events carry the sample offset inside this buffer at which they happen.
            while let Some(event) = next_event {
                if event.timing() > sample_index as u32 {
                    break;
                }
                match event {
                    NoteEvent::NoteOn { note, .. } => self.note = Some(note),
                    // Releasing an older key must not silence the key played after it.
                    NoteEvent::NoteOff { note, .. } if self.note == Some(note) => self.note = None,
                    _ => (),
                }
                next_event = context.next_event();
            }

            let sample = match self.note {
                Some(note) => {
                    let frequency = util::midi_note_to_freq(note);
                    self.oscillator.next(frequency, self.sample_rate) * LEVEL
                }
                None => 0.0,
            };

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
