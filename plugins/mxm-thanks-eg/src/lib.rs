//! MXM Thanks EG, T2's bare plugin: one Plaits voice at 48 kHz, the module's controls as
//! parameters, no editor, for the owner to hear the engines as they are ported
//! (ops' `plans/plan-mxm-thanks-eg.md` §7). The voices, the modulation, the resampler and the
//! editor come in T3 and T4.
//!
//! The module's jacks, played from MIDI: the note is V/OCT, and the gate is TRIG, held while a key
//! is held, as the six-op engines and the string machine read it. A new key while one is held
//! drops the trigger for one block, so the voice sees a new edge, as a gate into the module would.
//! The voice renders a block of 12 samples at a time, as the module does; an event takes effect at
//! the next block, at most 0.25 ms late.
//!
//! The engines run at 48 kHz. At 48 kHz a host sample is an engine frame; at any other rate the
//! frames are read by linear interpolation, a stopgap until T3's resampler, so the engines are in
//! tune at every rate but held bit for bit only at 48 kHz.

pub mod params;

use mxm_thanks_eg_engines::{Frame, Modulations, Patch, Voice};
use nice_plug::prelude::*;
use params::ThanksEgParams;
use std::sync::Arc;

/// The plugin's name, and the only place it is written in this crate.
macro_rules! plugin_name {
    () => {
        "mxm-thanks-eg"
    };
}

/// What the host displays.
pub const NAME: &str = plugin_name!();

/// The CLAP identifier, built from the name. Permanent from the gate (T5).
pub const CLAP_ID: &str = concat!("dk.mxm.", plugin_name!());

/// The engines' rate, the module's.
pub const SAMPLE_RATE: f32 = 48_000.0;

/// Plaits' block: the voice renders this many frames per call.
const BLOCK: usize = 12;

/// Keys held, newest last; enough for a hand, and nothing is allocated.
const HELD: usize = 16;

pub struct MxmThanksEg {
    params: Arc<ThanksEgParams>,
    voice: Box<Voice>,
    frames: [Frame; BLOCK],
    /// The next frame of `frames` to play; `BLOCK` when a block is due.
    position: usize,
    held: [u8; HELD],
    held_count: usize,
    /// One block of trigger low before the next, so a key pressed while another is held is a new
    /// edge.
    retrigger: bool,
    /// The pitch bend, in semitones, ±2.
    bend: f32,
    /// The key the voice plays: the newest held, or after the last release the one it kept.
    sounding: u8,
    /// Engine frames per host sample: 1 at 48 kHz.
    step: f64,
    /// How far the host has read between `previous` and `next`, in engine frames.
    phase: f64,
    previous: f32,
    next: f32,
}

impl Default for MxmThanksEg {
    fn default() -> Self {
        Self {
            params: Arc::new(ThanksEgParams::default()),
            voice: Voice::new(),
            frames: [Frame::default(); BLOCK],
            position: BLOCK,
            held: [0; HELD],
            held_count: 0,
            retrigger: false,
            bend: 0.0,
            sounding: 60,
            step: 1.0,
            phase: 0.0,
            previous: 0.0,
            next: 0.0,
        }
    }
}

impl MxmThanksEg {
    fn note_on(&mut self, note: u8) {
        self.remove_held(note);
        if self.held_count == HELD {
            self.held.copy_within(1.., 0);
            self.held_count -= 1;
        }
        if self.held_count > 0 {
            self.retrigger = true;
        }
        self.held[self.held_count] = note;
        self.held_count += 1;
    }

    /// Releasing the sounding key returns the voice to the key held before it, without a new edge;
    /// releasing the last key drops the gate.
    fn note_off(&mut self, note: u8) {
        self.remove_held(note);
    }

    fn remove_held(&mut self, note: u8) {
        if let Some(i) = self.held[..self.held_count].iter().position(|&n| n == note) {
            self.held.copy_within(i + 1..self.held_count, i);
            self.held_count -= 1;
        }
    }

    fn handle_event(&mut self, event: NoteEvent<()>) {
        match event {
            NoteEvent::NoteOn { key, .. } => {
                if let Some(note) = key.number() {
                    self.note_on(note);
                }
            }
            // A key with no number is every key, as a host's panic sends it.
            NoteEvent::NoteOff { key, .. } | NoteEvent::Choke { key, .. } => match key.number() {
                Some(note) => self.note_off(note),
                None => self.held_count = 0,
            },
            NoteEvent::MidiPitchBend { value, .. } => self.bend = (value * 2.0 - 1.0) * 2.0,
            // All sound off and all notes off.
            NoteEvent::MidiCC { cc: 120 | 123, .. } => self.held_count = 0,
            _ => {}
        }
    }

    fn render_block(&mut self) {
        let p = &self.params;
        if self.held_count > 0 {
            self.sounding = self.held[self.held_count - 1];
        }
        let key = f32::from(self.sounding);
        let patch = Patch {
            note: key + p.frequency.value() + p.fine.value() + self.bend,
            harmonics: p.harmonics.value(),
            timbre: p.timbre.value(),
            morph: p.morph.value(),
            frequency_modulation_amount: p.fm.value(),
            timbre_modulation_amount: p.timbre_mod.value(),
            morph_modulation_amount: p.morph_mod.value(),
            engine: p.model.value(),
            decay: p.decay.value(),
            lpg_colour: p.colour.value(),
        };
        let gate = self.held_count > 0 && !self.retrigger;
        self.retrigger = false;
        let modulations = Modulations {
            trigger: if gate { 1.0 } else { 0.0 },
            trigger_patched: true,
            ..Modulations::default()
        };
        self.voice.render(&patch, &modulations, &mut self.frames);
    }

    /// The next engine frame, mixed and scaled: Plaits' output stage inverts (×−32767), so a frame
    /// is read back as its DAC's output stage does.
    fn engine_sample(&mut self) -> f32 {
        if self.position == BLOCK {
            self.render_block();
            self.position = 0;
        }
        let frame = self.frames[self.position];
        self.position += 1;
        let out = -f32::from(frame.out) / 32767.0;
        let aux = -f32::from(frame.aux) / 32767.0;
        (out + (aux - out) * self.params.aux.value()) * self.params.volume.value()
    }
}

impl Plugin for MxmThanksEg {
    const NAME: &'static str = crate::NAME;
    const VENDOR: &'static str = "mxm";
    const URL: &'static str = "https://mxm.dk";
    const EMAIL: &'static str = "plugins@mxm.dk";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    /// An instrument: no main input; the voice is mono, its sum on both channels.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: None,
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: None,
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    /// `MidiCCs`: pitch bend, CC 120 and CC 123, and a host's panic clears a held key.
    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const SAMPLE_ACCURATE_AUTOMATION: bool = false;

    type Editor = ();
    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn activate(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl ActivateContext<Self>,
    ) -> bool {
        let rate = buffer_config.sample_rate;
        if !rate.is_finite() || rate <= 0.0 {
            return false;
        }
        self.step = f64::from(SAMPLE_RATE) / f64::from(rate);
        true
    }

    /// The host's reset can come on the audio thread, so nothing is allocated here: the keys are
    /// let go and the voice decays on its own. (T3's voices get a reset that clears their state.)
    fn reset(&mut self) {
        self.position = BLOCK;
        self.held_count = 0;
        self.retrigger = false;
        self.bend = 0.0;
        self.phase = 0.0;
        self.previous = 0.0;
        self.next = 0.0;
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let samples = buffer.samples();
        let output = buffer.as_slice();
        let mut next_event = context.next_event();
        for i in 0..samples {
            while let Some(event) = next_event {
                if event.timing() as usize > i {
                    break;
                }
                self.handle_event(event);
                next_event = context.next_event();
            }
            let sample = if self.step == 1.0 {
                self.engine_sample()
            } else {
                self.phase += self.step;
                while self.phase >= 1.0 {
                    self.phase -= 1.0;
                    self.previous = self.next;
                    self.next = self.engine_sample();
                }
                self.previous + (self.next - self.previous) * self.phase as f32
            };
            for channel in output.iter_mut() {
                channel[i] = sample;
            }
        }
        // The engines keep sounding after a key is released (a gate's tail, a resonator's ring),
        // and Plaits' output stage never returns exact zero.
        ProcessStatus::KeepAlive
    }
}

impl ClapPlugin for MxmThanksEg {
    const CLAP_ID: &'static str = CLAP_ID;
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("A macro-oscillator instrument: twenty-four engines, each played from four controls");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
        ClapFeature::Mono,
    ];
}

impl Vst3Plugin for MxmThanksEg {
    /// The CLAP id's hash written out, held to it by
    /// `identity::the_vst3_class_id_is_the_clap_ids_hash`.
    const VST3_CLASS_ID: [u8; 16] = [
        0xfd, 0x89, 0xa2, 0x15, 0x63, 0x52, 0xf0, 0xdf, 0x77, 0x68, 0x9a, 0x4e, 0xd8, 0xba, 0xb4,
        0xf1,
    ];
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}

nice_export_clap!(MxmThanksEg);
nice_export_vst3!(MxmThanksEg);

#[cfg(test)]
mod identity {
    use super::{CLAP_ID, NAME};
    use mxm_plugin_test::formats;

    #[test]
    fn the_vst3_class_id_is_the_clap_ids_hash() {
        formats::the_vst3_class_id_is_the_clap_ids_hash::<super::MxmThanksEg>();
    }

    #[test]
    fn the_vst3_categories_follow_the_clap_features() {
        formats::the_vst3_categories_follow_the_clap_features::<super::MxmThanksEg>();
    }

    #[test]
    fn the_id_is_the_name_under_the_project_domain() {
        assert_eq!(CLAP_ID, format!("dk.mxm.{NAME}"));
    }

    #[test]
    fn the_bundle_is_named_after_this_plugin() {
        mxm_plugin_test::bundle::is_named(env!("CARGO_MANIFEST_DIR"), env!("CARGO_PKG_NAME"), NAME);
    }
}
