//! The instrument's side of the collection's preset system (`mxm-preset`): its id, version,
//! parameters and identity slot. Init is generated from the parameter defaults and has no file;
//! the factory set is the files in this crate's `presets/`, in `order.txt`'s order, which
//! `cargo xtask bundle` checks and installs for the plugin's version, where the plugin reads them.
//!
//! The factory sounds are designed in the test module's [`tests::design`], as overrides on Init in
//! each parameter's own units, and written to `presets/` by the ignored
//! `write_the_factory_presets`; `the_shipped_files_are_the_design` catches a forgotten regenerate,
//! and `tests/factory_levels.rs` holds the set's loudness.

use crate::params::ThanksEgParams;
use std::sync::RwLock;

pub use mxm_preset::{Category, INIT_NAME, Preset, PresetIdentity, Value};

impl mxm_preset::Instrument for ThanksEgParams {
    fn clap_id(&self) -> &'static str {
        crate::CLAP_ID
    }

    /// The plugin's version, its `Plugin::VERSION`: its factory presets are installed for it.
    fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    fn parameters(&self) -> Vec<(&'static str, &dyn mxm_params::ErasedParam)> {
        self.all_parameters()
    }

    fn identity(&self) -> &RwLock<PresetIdentity> {
        &self.preset
    }

    /// The installed set, but in a unit test the repository's, never the machine's.
    #[cfg(test)]
    fn factory_files(&self) -> Vec<(String, String)> {
        mxm_preset::read_factory_dir(&factory_folder())
    }
}

/// This crate's `presets/`: what the bundle installs, and what the tests read.
#[must_use]
pub fn factory_folder() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("presets")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::{
        BANKS, ENGINE_NAMES, ENVELOPE_IDS, EnvelopeModeChoice, INIT_MODEL, LFO_IDS, LFO_SYNC,
        LfoShapeChoice, VoiceMode,
    };
    use crate::routes::{ROUTE_IDS, SourceChoice, TargetChoice};
    use mxm_preset::Instrument as _;
    use mxm_thanks_eg_dsp::{SLOTS, Source, Target};
    use nice_plug::prelude::{Enum as _, Param as _, ParamSetter, Params as _};

    /// One factory sound: its name, its category, and what it changes from Init, each value in its
    /// parameter's own units (seconds, hertz, a fraction, an engine's or a list's index).
    type Design = (&'static str, Category, Vec<(&'static str, f32)>);

    fn source(source: Source) -> f32 {
        SourceChoice(Some(source)).to_index() as f32
    }

    fn target(target: Target) -> f32 {
        TargetChoice(Some(target)).to_index() as f32
    }

    /// Harmonics at the middle of chord `k` of the string machine's, the chiptune's and the chord
    /// engine's eleven (octaves, fifth, sus4, minor, m7, m9, m11, 6/9, maj9, maj7, major): their
    /// quantiser reads `11 × 1.02 × Harmonics`.
    fn chord(k: u8) -> f32 {
        (f32::from(k) + 0.5) / (11.0 * 1.02)
    }

    /// Harmonics at the middle of patch `k` of a six-op engine's 32: its quantiser reads
    /// `32 × 1.02 × Harmonics`.
    fn patch(k: u8) -> f32 {
        (f32::from(k) + 0.5) / (32.0 * 1.02)
    }

    /// A factory sound as it is written: Init with each change made, its routes filling the slots
    /// from the first.
    struct Sound {
        name: &'static str,
        category: Category,
        set: Vec<(&'static str, f32)>,
        routes: usize,
    }

    impl Sound {
        fn new(name: &'static str, category: Category, model: usize) -> Self {
            Self {
                name,
                category,
                set: vec![("model", model as f32)],
                routes: 0,
            }
        }

        fn set(mut self, id: &'static str, plain: f32) -> Self {
            self.set.push((id, plain));
            self
        }

        /// Harmonics, Timbre and Morph: the engine's three controls.
        fn knobs(self, harmonics: f32, timbre: f32, morph: f32) -> Self {
            self.set("harmonics", harmonics)
                .set("timbre", timbre)
                .set("morph", morph)
        }

        fn voice(self, mode: VoiceMode) -> Self {
            self.set("mode", mode.to_index() as f32)
        }

        /// Envelope `n`'s attack, decay, sustain and release.
        fn env(self, n: usize, attack: f32, decay: f32, sustain: f32, release: f32) -> Self {
            let [a, d, s, r, ..] = ENVELOPE_IDS[n - 1];
            self.set(a, attack)
                .set(d, decay)
                .set(s, sustain)
                .set(r, release)
        }

        fn response(self, n: usize, response: f32) -> Self {
            self.set(ENVELOPE_IDS[n - 1][4], response)
        }

        /// Envelope `n` cycles: attack and decay repeat, an LFO.
        fn cycling(self, n: usize) -> Self {
            self.set(ENVELOPE_IDS[n - 1][5], 1.0)
        }

        fn env_mode(self, n: usize, mode: EnvelopeModeChoice) -> Self {
            self.set(ENVELOPE_IDS[n - 1][6], mode.to_index() as f32)
        }

        fn lfo(self, n: usize, rate_hz: f32, shape: LfoShapeChoice) -> Self {
            let [rate, form, ..] = LFO_IDS[n - 1];
            self.set(rate, rate_hz).set(form, shape.to_index() as f32)
        }

        /// LFO `n` follows the host's tempo at `division`; free, without a tempo, at the rate whose
        /// position picks it.
        fn synced(self, n: usize, division: mxm_tempo::Division) -> Self {
            let [rate, _, sync, ..] = LFO_IDS[n - 1];
            let position = LFO_SYNC.position(division);
            let hz = ThanksEgParams::default().lfo1.rate.preview_plain(position);
            self.set(rate, hz).set(sync, 1.0)
        }

        /// LFO `n` restarts with each note, each voice its own.
        fn retriggered(self, n: usize) -> Self {
            self.set(LFO_IDS[n - 1][3], 1.0)
        }

        fn fade(self, n: usize, seconds: f32) -> Self {
            self.set(LFO_IDS[n - 1][4], seconds)
        }

        /// A route in the next free slot.
        fn route(mut self, from: Source, to: Target, amount: f32) -> Self {
            assert!(
                self.routes < SLOTS,
                "{} has more routes than slots",
                self.name
            );
            let [s, t, a] = ROUTE_IDS[self.routes];
            self.routes += 1;
            self.set(s, source(from)).set(t, target(to)).set(a, amount)
        }

        /// **The strike follows velocity**: Envelope 1 held open into Level at `amount`, and
        /// Velocity into Level at `amount`, so an engine that envelopes itself (the six-op, the
        /// strings, the drums) takes its accent from the note, `amount × velocity` through the
        /// module's compression of Level.
        fn struck(self, amount: f32) -> Self {
            self.env(1, 0.0005, 0.3, 1.0, 0.05)
                .route(Envelope1, Level, amount)
                .route(Velocity, Level, amount)
        }

        fn designed(self) -> Design {
            (self.name, self.category, self.set)
        }
    }

    use Category::{Bass, Brass, Drone, Fx, Keys, Lead, Pad, Percussion, Pluck, Sequence, Strings};
    use EnvelopeModeChoice::{Follow, Gate};
    use LfoShapeChoice::{SampleAndHold, SmoothRandom, Square, Triangle};
    use Source::{
        Envelope1, Envelope1Cycle, Envelope1Rise, Envelope2, Key, Lfo1, Lfo2, Lfo3, Pressure,
        Random, Velocity, Wheel,
    };
    use Target::{
        Amplitude, Envelope1Trigger, Envelope2Input, Envelope2Trigger, Harmonics, Level, Model,
        Morph, Pan, Pitch, Timbre,
    };

    /// **The factory sounds, readable**, in the browser's order: a first impression, then by
    /// category. Every engine has one sound that shows what it does best and the strong ones two;
    /// the modulation the instrument adds to the module is shown off along the way, each such sound
    /// saying what it shows. Each one's loudness is held by `tests/factory_levels.rs`, and is set
    /// in the sound, never by Volume.
    pub(super) fn design() -> Vec<Design> {
        let sounds = vec![
            // The first impression: the second six-op bank's electric piano, velocity striking its
            // tines and setting its level, panned by a shared LFO.
            Sound::new("Tine keys", Keys, 3)
                .knobs(patch(0), 0.5, 0.5)
                .lfo(1, 3.5, Triangle)
                .struck(0.25)
                .route(Velocity, Amplitude, 1.0)
                .route(Lfo1, Pan, 0.35),
            // Bass.
            Sound::new("Filter bass", Bass, 0)
                .knobs(0.3, 0.55, 0.3)
                .set("timbre_amount", 0.45)
                .set("colour", 0.5)
                .voice(VoiceMode::Mono)
                .env(1, 0.0005, 0.4, 0.6, 0.08)
                .route(Envelope1, Level, 1.0)
                .route(Velocity, Timbre, 0.3),
            // Mono with glide: a resonant line whose envelope sweeps the cutoff.
            Sound::new("Rubber acid", Bass, 0)
                .knobs(0.05, 0.22, 0.4)
                .set("colour", 0.6)
                .voice(VoiceMode::Mono)
                .set("glide", 0.06)
                .env(1, 0.0005, 0.3, 0.8, 0.06)
                .env(2, 0.0005, 0.25, 0.0, 0.1)
                .route(Envelope1, Level, 1.0)
                .route(Envelope2, Timbre, 0.45)
                .route(Velocity, Timbre, 0.4),
            Sound::new("Hard FM bass", Bass, 2)
                .knobs(patch(10), 0.5, 0.5)
                .route(Velocity, Amplitude, 0.6),
            // The chiptune's triangle bass alone, an octave under the key, so up an octave.
            Sound::new("Triangle bass", Bass, 7)
                .knobs(chord(0), 0.0, 0.5)
                .set("frequency", 12.0)
                .set("aux", 1.0)
                .set("timbre_amount", 0.35)
                .voice(VoiceMode::Mono),
            // An envelope cycling as an LFO, from each note, on the wavefolder.
            Sound::new("Wobble fold bass", Bass, 9)
                .knobs(0.3, 0.2, 0.3)
                .voice(VoiceMode::Mono)
                .env(1, 0.0005, 0.3, 1.0, 0.08)
                .env(2, 0.06, 0.1, 0.7, 0.1)
                .cycling(2)
                .route(Envelope1, Level, 1.0)
                .route(Envelope2, Timbre, 0.4),
            // Lead.
            // Legato with glide: tied notes slide without a new attack.
            Sound::new("Legato lead", Lead, 9)
                .knobs(0.5, 0.55, 0.35)
                .voice(VoiceMode::Legato)
                .set("glide", 0.08)
                .env(1, 0.005, 0.3, 1.0, 0.15)
                .fade(1, 0.4)
                .route(Envelope1, Level, 0.4)
                .route(Lfo1, Pitch, 0.025)
                .route(Wheel, Morph, 0.4),
            // The phase distortion's amount swept by an envelope cycling as an LFO.
            Sound::new("Cycling sweep", Lead, 1)
                .knobs(0.5, 0.3, 0.5)
                .set("colour", 1.0)
                .voice(VoiceMode::Mono)
                .set("glide", 0.03)
                .env(1, 0.01, 0.3, 1.0, 0.2)
                .env(2, 0.12, 0.12, 0.7, 0.2)
                .cycling(2)
                .route(Envelope1, Level, 0.15)
                .route(Envelope2, Timbre, 0.4),
            // Hard sync swept down by an envelope, more of it on the wheel.
            Sound::new("Sync lead", Lead, 8)
                .knobs(0.5, 0.75, 0.8)
                .set("aux", 0.3)
                .voice(VoiceMode::Mono)
                .env(1, 0.005, 0.3, 1.0, 0.15)
                .env(2, 0.0005, 0.4, 0.2, 0.2)
                .fade(1, 0.4)
                .route(Envelope1, Level, 0.6)
                .route(Envelope2, Timbre, 0.25)
                .route(Wheel, Timbre, 0.25)
                .route(Lfo1, Pitch, 0.01),
            // Follow as a lag on Key: the formant climbs after the melody, slowly; Legato glides.
            // Envelope 1's end of cycle, high once it is back at rest, takes the input away, so
            // the follower comes to rest after the note.
            Sound::new("Formant glide", Lead, 11)
                .knobs(0.55, 0.4, 0.5)
                .voice(VoiceMode::Legato)
                .set("glide", 0.12)
                .env(1, 0.02, 0.3, 1.0, 0.2)
                .env(2, 0.35, 0.3, 0.7, 0.35)
                .env_mode(2, Follow)
                .route(Envelope1, Level, 1.0)
                .route(Key, Envelope2Input, 1.0)
                .route(Envelope1Cycle, Envelope2Input, -1.0)
                .route(Envelope2, Timbre, 1.0),
            // Noise through a resonance high enough to pitch it, breathier under pressure.
            Sound::new("Noise flute", Lead, 17)
                .knobs(0.5, 0.9, 0.92)
                .env(1, 0.06, 0.3, 0.9, 0.2)
                .fade(1, 0.5)
                .route(Envelope1, Level, 0.4)
                .route(Lfo1, Pitch, 0.01)
                .route(Pressure, Morph, -0.1),
            // Pad.
            Sound::new("Soft saw pad", Pad, 8)
                .knobs(0.45, 0.4, 0.5)
                .set("colour", 1.0)
                .set("spread", 0.5)
                .env(1, 0.4, 0.3, 0.9, 1.0)
                .route(Envelope1, Level, 0.22)
                .route(Lfo2, Timbre, 0.15),
            // A delayed envelope: Envelope 2, in Gate mode, fires when Envelope 1's attack ends,
            // and opens the terrain's path after the swell.
            Sound::new("Delayed bloom", Pad, 5)
                .knobs(0.6, 0.25, 0.4)
                .set("colour", 1.0)
                .set("spread", 0.4)
                .env(1, 0.6, 0.5, 1.0, 1.2)
                .env(2, 1.5, 1.0, 1.0, 0.5)
                .env_mode(2, Gate)
                .route(Envelope1, Level, 0.12)
                .route(Envelope1Rise, Envelope2Trigger, 1.0)
                .route(Envelope2, Timbre, 0.4)
                .route(Envelope2, Morph, -0.2),
            // The terrain's path wandering under two slow LFOs.
            Sound::new("Terrain drift", Pad, 5)
                .knobs(0.3, 0.4, 0.5)
                .set("colour", 1.0)
                .set("spread", 0.5)
                .env(1, 0.8, 1.0, 0.9, 1.5)
                .lfo(2, 0.13, Triangle)
                .lfo(3, 0.07, SmoothRandom)
                .route(Envelope1, Level, 0.086)
                .route(Lfo2, Morph, 0.25)
                .route(Lfo3, Timbre, 0.15),
            // Two slow LFOs scanning the wavetable's grid.
            Sound::new("Wavetable scan", Pad, 13)
                .knobs(0.25, 0.3, 0.5)
                .set("colour", 1.0)
                .env(1, 0.7, 1.0, 1.0, 1.5)
                .lfo(2, 0.11, Triangle)
                .lfo(3, 0.07, Triangle)
                .route(Envelope1, Level, 0.22)
                .route(Lfo2, Timbre, 0.3)
                .route(Lfo3, Morph, 0.3),
            Sound::new("Formant choir", Pad, 11)
                .knobs(0.6, 0.45, 0.3)
                .set("colour", 1.0)
                .set("spread", 0.5)
                .env(1, 0.5, 0.5, 1.0, 1.0)
                .lfo(2, 0.3, Triangle)
                .fade(1, 1.0)
                .route(Envelope1, Level, 0.25)
                .route(Lfo2, Timbre, 0.08)
                .route(Lfo3, Harmonics, 0.1)
                .route(Lfo1, Pitch, 0.008),
            // The additive engine's peaks moved by an LFO and a slow envelope.
            Sound::new("Harmonic sweep", Pad, 12)
                .knobs(0.2, 0.3, 0.5)
                .set("colour", 1.0)
                .env(1, 1.0, 1.0, 1.0, 1.5)
                .env(2, 3.0, 2.0, 0.5, 1.5)
                .lfo(2, 0.15, Triangle)
                .route(Envelope1, Level, 0.071)
                .route(Lfo2, Timbre, 0.3)
                .route(Envelope2, Harmonics, 0.3),
            // One key, a major ninth; the chord engine's wavetable half, each voice crossfading at
            // its own point as an LFO moves it.
            Sound::new("Glass chord pad", Pad, 14)
                .knobs(chord(8), 0.5, 0.75)
                .env(1, 0.6, 1.0, 1.0, 1.5)
                .lfo(2, 0.2, Triangle)
                .route(Envelope1, Level, 1.0)
                .route(Lfo2, Morph, 0.1),
            Sound::new("Vowel pad", Pad, 15)
                .knobs(0.08, 0.4, 0.3)
                .set("colour", 1.0)
                .env(1, 0.4, 0.5, 1.0, 1.0)
                .lfo(2, 0.25, Triangle)
                .fade(1, 1.0)
                .route(Envelope1, Level, 0.4)
                .route(Lfo2, Morph, 0.3)
                .route(Lfo1, Pitch, 0.006),
            // A vocal patch of the first six-op bank, struck softly.
            Sound::new("Vocal glass", Pad, 2)
                .knobs(patch(26), 0.3, 0.5)
                .struck(0.25)
                .route(Velocity, Amplitude, 1.0)
                .route(Lfo2, Timbre, 0.1),
            // Keys.
            Sound::new("Bright keys", Keys, 1)
                .knobs(0.35, 0.65, 0.3)
                .set("decay", 0.55)
                .set("colour", 0.6)
                .route(Velocity, Timbre, 0.3),
            // The string machine's octaves as an organ, its ensemble on the wheel.
            Sound::new("Combo organ", Keys, 6)
                .knobs(chord(0), 0.5, 0.6)
                .set("decay", 0.2)
                .set("colour", 1.0)
                .env(1, 0.003, 0.1, 1.0, 0.06)
                .route(Envelope1, Level, 0.6)
                .route(Wheel, Timbre, 0.4),
            // The additive engine's organ harmonics, on AUX.
            Sound::new("Drawbar organ", Keys, 12)
                .knobs(0.3, 0.4, 0.6)
                .set("aux", 1.0)
                .set("decay", 0.2)
                .set("colour", 1.0)
                .env(1, 0.003, 0.1, 1.0, 0.05)
                .lfo(1, 6.5, Triangle)
                .route(Envelope1, Level, 0.087)
                .route(Lfo1, Pitch, 0.005)
                .route(Wheel, Timbre, 0.3),
            Sound::new("Chord stabs", Keys, 14)
                .knobs(chord(4), 0.5, 0.4)
                .env(1, 0.0005, 0.35, 0.25, 0.15)
                .route(Envelope1, Level, 1.0),
            // The two-op FM's index falling with the decay envelope, at an inharmonic ratio, and
            // the bell's own fall an exponential envelope through the gate.
            Sound::new("Cold bell", Keys, 10)
                .knobs(0.75, 0.45, 0.5)
                .set("timbre_amount", 0.5)
                .set("decay", 0.75)
                .set("colour", 1.0)
                .env(1, 0.0005, 3.0, 0.0, 2.0)
                .response(1, 0.6)
                .route(Envelope1, Level, 0.08),
            // Pluck.
            // A damper: the string rings on its own whatever the key does, so Envelope 1 lifts
            // the damping (Morph) while the key is down and lets it fall when the key goes up.
            Sound::new("Nylon pluck", Pluck, 19)
                .knobs(0.25, 0.65, 0.5)
                .env(1, 0.0005, 0.3, 1.0, 0.8)
                .route(Envelope1, Morph, 0.25),
            // The string's curved bridge, buzzing, damped as the nylon is.
            Sound::new("Bridge buzz", Pluck, 19)
                .knobs(0.05, 0.7, 0.5)
                .env(1, 0.0005, 0.3, 1.0, 0.8)
                .route(Envelope1, Morph, 0.25),
            Sound::new("FM pluck", Pluck, 10)
                .knobs(0.5, 0.35, 0.55)
                .set("timbre_amount", 0.6)
                .set("decay", 0.45),
            // The wavetable's five-bit AUX, its grid stepped.
            Sound::new("Five-bit pluck", Pluck, 13)
                .knobs(0.7, 0.4, 0.4)
                .set("aux", 1.0)
                .set("decay", 0.55)
                .set("colour", 0.9)
                .env(2, 0.0005, 0.3, 0.0, 0.1)
                .route(Envelope2, Morph, 0.3),
            // Random on Model: every note a different engine, from Virtual analogue to Chords.
            Sound::new("Engine roulette", Pluck, 11)
                .knobs(0.5, 0.5, 0.5)
                .set("decay", 0.5)
                .set("colour", 0.6)
                .set("spread", 0.3)
                .route(Random, Model, 0.125),
            // Brass.
            Sound::new("Analogue brass", Brass, 0)
                .knobs(0.7, 0.3, 0.3)
                .set("colour", 0.4)
                .env(1, 0.06, 0.4, 0.75, 0.25)
                .env(2, 0.08, 0.5, 0.4, 0.3)
                .fade(1, 0.6)
                .route(Envelope1, Level, 0.5)
                .route(Envelope2, Timbre, 0.35)
                .route(Lfo1, Pitch, 0.008)
                .route(Wheel, Timbre, 0.3),
            Sound::new("Bright horns", Brass, 4)
                .knobs(patch(29), 0.55, 0.5)
                .fade(1, 0.5)
                .route(Velocity, Amplitude, 1.0)
                .route(Lfo1, Pitch, 0.01)
                .route(Wheel, Timbre, 0.3),
            // Strings.
            Sound::new("String ensemble", Strings, 6)
                .knobs(chord(0), 0.75, 0.3)
                .set("spread", 0.6)
                .env(1, 0.5, 0.3, 1.0, 1.2)
                .route(Envelope1, Level, 1.0)
                .route(Lfo2, Morph, 0.05),
            Sound::new("Silk strings", Strings, 4)
                .knobs(patch(25), 0.5, 0.5)
                .route(Velocity, Amplitude, 0.5)
                .route(Lfo3, Timbre, 0.05),
            // Percussion.
            Sound::new("Struck bar", Percussion, 20).knobs(0.6, 0.8, 0.6),
            Sound::new("Singing bowl", Percussion, 20).knobs(0.85, 0.3, 0.66),
            // The vibraphone of the second six-op bank, its motor an LFO on Amplitude.
            Sound::new("Motor mallets", Percussion, 3)
                .knobs(patch(18), 0.5, 0.5)
                .lfo(1, 5.0, Triangle)
                .route(Lfo1, Amplitude, 0.25),
            Sound::new("Deep kick", Percussion, 21)
                .knobs(0.4, 0.3, 0.7)
                .set("frequency", -12.0)
                .struck(1.0),
            Sound::new("Driven kick", Percussion, 21)
                .knobs(0.8, 0.5, 0.45)
                .set("frequency", -12.0),
            Sound::new("Tight snare", Percussion, 22)
                .knobs(0.6, 0.5, 0.5)
                .set("frequency", 12.0)
                .struck(1.0),
            Sound::new("Synth snare", Percussion, 22)
                .knobs(0.5, 0.8, 0.65)
                .set("frequency", 12.0)
                .set("aux", 1.0)
                .struck(1.0),
            Sound::new("Closed hat", Percussion, 23)
                .knobs(0.2, 0.6, 0.5)
                .set("frequency", 24.0)
                .struck(1.0),
            Sound::new("Open hat", Percussion, 23)
                .knobs(0.6, 0.65, 0.8)
                .set("frequency", 24.0)
                .set("aux", 0.3)
                .struck(1.0),
            // Fx.
            Sound::new("Noise wind", Fx, 17)
                .knobs(0.5, 0.4, 0.7)
                .env(1, 1.5, 0.3, 1.0, 2.0)
                .lfo(2, 0.3, LfoShapeChoice::Sine)
                .route(Envelope1, Level, 1.0)
                .route(Lfo2, Timbre, 0.3)
                .route(Lfo3, Harmonics, 0.2),
            // Random on Morph: each note says a different one of the synthesizer words.
            Sound::new("Robot words", Fx, 15)
                .knobs(0.92, 0.5, 0.5)
                .route(Random, Morph, 0.5),
            Sound::new("Glissando swarm", Fx, 16)
                .knobs(0.5, 0.6, 0.85)
                .env(1, 1.0, 1.0, 1.0, 2.0)
                .route(Envelope1, Level, 1.0),
            Sound::new("Rain on glass", Fx, 18)
                .knobs(0.6, 0.85, 0.8)
                .set("colour", 0.7)
                .env(1, 0.5, 1.0, 1.0, 2.0)
                .route(Envelope1, Level, 1.0),
            // Sparse particles through a resonance near its peak: pitched glints, held open.
            Sound::new("Glass dust", Fx, 18)
                .knobs(0.8, 0.4, 0.85)
                .env(1, 0.01, 0.3, 1.0, 1.5)
                .route(Envelope1, Level, 1.0),
            // Sequence.
            // In Mono each key steps the chiptune's arpeggio on: play one key again and again.
            Sound::new("Chip arpeggio", Sequence, 7)
                .knobs(chord(4), 0.1, 0.5)
                .set("aux", 0.2)
                .set("timbre_amount", 0.35)
                .voice(VoiceMode::Mono),
            // A held key plays sixteenths: a square LFO on the tempo gates Envelope 1 in Gate
            // mode, and a sample-and-hold on the same division steps the FM index. The key is in
            // the gate too: Envelope 2 follows it, and only the two together pass one half.
            Sound::new("Clocked bleeps", Sequence, 10)
                .knobs(0.6, 0.4, 0.5)
                .set("decay", 0.1)
                .set("colour", 1.0)
                .synced(1, mxm_tempo::Division::Sixteenth)
                .synced(2, mxm_tempo::Division::Sixteenth)
                .set(LFO_IDS[0][1], Square.to_index() as f32)
                .set(LFO_IDS[1][1], SampleAndHold.to_index() as f32)
                .retriggered(1)
                .retriggered(2)
                .env(1, 0.001, 0.08, 0.0, 0.05)
                .env_mode(1, Gate)
                .env(2, 0.0005, 0.3, 1.0, 0.0005)
                .route(Lfo1, Envelope1Trigger, 0.45)
                .route(Envelope2, Envelope1Trigger, 0.45)
                .route(Envelope1, Level, 0.35)
                .route(Lfo2, Timbre, 0.3),
            // Drone.
            Sound::new("Grain drone", Drone, 16)
                .knobs(0.3, 0.6, 0.5)
                .set("spread", 0.6)
                .env(1, 1.5, 1.0, 1.0, 3.0)
                .route(Envelope1, Level, 1.0)
                .route(Lfo3, Morph, 0.1)
                .route(Lfo2, Harmonics, 0.1),
            // Pressure opens the filter.
            Sound::new("Resonant drone", Drone, 0)
                .knobs(0.15, 0.2, 0.9)
                .set("frequency", -12.0)
                .env(1, 2.0, 1.0, 1.0, 3.0)
                .route(Envelope1, Level, 1.0)
                .route(Lfo3, Timbre, 0.15)
                .route(Pressure, Timbre, 0.3),
        ];
        sounds.into_iter().map(Sound::designed).collect()
    }

    /// The file a sound is written to.
    fn file_name(name: &str) -> String {
        format!("{}.json", name.to_lowercase().replace(' ', "-"))
    }

    /// A design as the preset it is: Init, with each override converted to its parameter's
    /// normalised value and every text the plugin's own, read in the preset's state.
    fn generated(params: &ThanksEgParams, (name, category, overrides): &Design) -> Preset {
        let map = params.param_map();
        let mut preset = Preset::init(params);
        preset.name = (*name).to_owned();
        preset.category = *category;
        for (id, plain) in overrides {
            let (_, ptr, _) = map
                .iter()
                .find(|(candidate, ..)| candidate == id)
                .unwrap_or_else(|| panic!("{name:?} names unknown parameter `{id}`"));
            // SAFETY: `params` owns the parameter and outlives the call.
            let normalised = unsafe { ptr.preview_normalized(*plain) };
            preset
                .params
                .get_mut(*id)
                .unwrap_or_else(|| panic!("{id} is not in Init"))
                .v = normalised;
        }
        mxm_preset::refresh_texts(params, &mut preset);
        preset
    }

    fn shipped() -> Vec<(String, String)> {
        ThanksEgParams::default().factory_files()
    }

    fn model_of(overrides: &[(&str, f32)]) -> usize {
        overrides
            .iter()
            .find(|(id, _)| *id == "model")
            .map_or(INIT_MODEL, |(_, v)| *v as i32) as usize
    }

    #[test]
    #[ignore = "writes the factory preset files"]
    fn write_the_factory_presets() {
        let params = ThanksEgParams::default();
        let folder = factory_folder();
        std::fs::create_dir_all(&folder).unwrap();
        let mut order = String::new();
        for design in design() {
            let file = file_name(design.0);
            std::fs::write(folder.join(&file), generated(&params, &design).to_json()).unwrap();
            order.push_str(&file);
            order.push('\n');
        }
        std::fs::write(folder.join(mxm_preset::ORDER_FILE), order).unwrap();
    }

    /// **The shipped files are the design**, in its order: a design changed without regenerating
    /// fails here.
    #[test]
    fn the_shipped_files_are_the_design() {
        let params = ThanksEgParams::default();
        let files = shipped();
        let designs = design();
        assert_eq!(
            files
                .iter()
                .map(|(file, _)| file.clone())
                .collect::<Vec<_>>(),
            designs.iter().map(|d| file_name(d.0)).collect::<Vec<_>>(),
            "regenerate: cargo test -p mxm-thanks-eg --lib write_the_factory_presets -- --ignored"
        );
        for ((file, json), design) in files.iter().zip(&designs) {
            assert_eq!(
                Preset::parse(json, crate::CLAP_ID).unwrap(),
                generated(&params, design),
                "regenerate {file}"
            );
        }
    }

    /// **Every stored `text` is this plugin's own formatting of the `v` beside it**
    /// (`mxm_preset::stale_factory_texts`).
    #[test]
    fn every_factory_text_is_this_plugins_formatting_of_its_value() {
        let stale = mxm_preset::stale_factory_texts(&ThanksEgParams::default());
        assert!(stale.is_empty(), "{}", stale.join("\n"));
    }

    /// Every file is this plugin's, names every parameter, is categorised and is a sound of its
    /// own.
    #[test]
    fn every_file_is_complete_distinct_categorised_and_owned() {
        let params = ThanksEgParams::default();
        let mut seen = Vec::new();
        assert!(!shipped().is_empty(), "no factory files were read");
        for (file, json) in shipped() {
            let preset =
                Preset::parse(&json, crate::CLAP_ID).unwrap_or_else(|e| panic!("{file}: {e}"));
            assert_ne!(preset.category, Category::Uncategorised, "{file}");
            let (_, problems) = preset.resolve(&params);
            assert!(problems.is_empty(), "{file}: {problems:?}");
            assert_eq!(preset.params.len(), params.all_parameters().len(), "{file}");
            assert!(!seen.contains(&preset.params), "{file} repeats a sound");
            seen.push(preset.params);
        }
    }

    /// **Every file loads as a host loads it** (`mxm_preset::clap::load_file`, the CLAP preset
    /// loader's path), and marks itself loaded by its name.
    #[test]
    fn every_file_loads_through_the_hosts_loader() {
        let params = ThanksEgParams::default();
        let host = mxm_plugin_test::NoHost;
        let setter = ParamSetter::new(&host);
        for (file, json) in shipped() {
            let path = factory_folder().join(&file);
            assert_eq!(
                mxm_preset::clap::load_file(&params, &path, &setter),
                Ok(()),
                "{file}"
            );
            let name = Preset::parse(&json, crate::CLAP_ID).unwrap().name;
            assert_eq!(mxm_preset::loaded(&params).name(), Some(name.as_str()));
        }
    }

    /// `order.txt` names every file once, and nothing else.
    #[test]
    fn the_order_names_every_file_once() {
        let folder = factory_folder();
        let order = std::fs::read_to_string(folder.join(mxm_preset::ORDER_FILE)).unwrap();
        let mut listed: Vec<String> = order.lines().map(str::to_owned).collect();
        let mut files: Vec<String> = std::fs::read_dir(&folder)
            .unwrap()
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|name| name.ends_with(".json"))
            .collect();
        listed.sort();
        files.sort();
        assert_eq!(listed, files);
    }

    /// **Fifty sounds is the floor** (mxm-kit's `docs/plugin-conventions.md`, *A factory set is
    /// content*).
    #[test]
    fn the_set_is_at_least_fifty_sounds() {
        assert!(design().len() >= 50, "{} sounds", design().len());
    }

    /// **Every engine has a sound**, and so every bank: the browser opens on each part of the
    /// instrument.
    #[test]
    fn every_engine_and_bank_has_a_sound() {
        let mut engines = [false; ENGINE_NAMES.len()];
        for (_, _, overrides) in design() {
            engines[model_of(&overrides)] = true;
        }
        let missing: Vec<&str> = ENGINE_NAMES
            .iter()
            .zip(engines)
            .filter(|(_, has)| !has)
            .map(|(name, _)| *name)
            .collect();
        assert!(missing.is_empty(), "no sound plays {missing:?}");
        let banks: Vec<bool> = engines
            .chunks(ENGINE_NAMES.len() / BANKS)
            .map(|b| b.iter().any(|&e| e))
            .collect();
        assert_eq!(banks, [true; BANKS]);
    }

    /// **The names are the set's own**: sentence case, short, and naming neither the module, its
    /// maker, the function generator it takes after nor its maker (root `AGENTS.md`, *The names*).
    #[test]
    fn every_name_is_its_own_short_and_in_sentence_case() {
        for (name, ..) in design() {
            let lower = name.to_lowercase();
            for word in ["plaits", "mutable", "maths", "make noise"] {
                assert!(!lower.contains(word), "{name:?} names {word:?}");
            }
            assert!(name.len() <= 20, "{name:?} is long");
            let mut chars = name.chars();
            assert!(chars.next().is_some_and(char::is_uppercase), "{name:?}");
            // A later word starts in lower case, unless it is an initialism (FM).
            let title_cased = |word: &str| {
                word.chars().next().is_some_and(char::is_uppercase)
                    && word.chars().any(char::is_lowercase)
            };
            assert!(
                !name.split(' ').skip(1).any(title_cased),
                "{name:?} is not sentence case"
            );
        }
    }

    /// **Init is every parameter's default, and the browser's first row**: the generated Init and
    /// the CLAP defaults are one set of numbers.
    #[test]
    fn init_is_every_parameter_default_and_factory_begins_with_it() {
        let params = ThanksEgParams::default();
        let init = Preset::init(&params);
        assert_eq!(init.params.len(), params.all_parameters().len());
        for (id, parameter) in params.all_parameters() {
            assert_eq!(init.params[id].v, parameter.default_normalised(), "{id}");
        }
        let all = mxm_preset::factory(&params);
        assert_eq!(all[0].name, INIT_NAME);
        assert_eq!(all.len(), 1 + shipped().len());
    }

    #[test]
    fn factory_content_does_not_move_volume() {
        let params = ThanksEgParams::default();
        let default = params.volume.default_normalised();
        for (file, json) in shipped() {
            let preset = Preset::parse(&json, crate::CLAP_ID).unwrap();
            assert_eq!(preset.params["volume"].v, default, "{file}");
        }
    }

    #[test]
    fn user_files_are_namespaced_to_the_permanent_id() {
        if let Some(root) = mxm_preset::user_root(crate::CLAP_ID) {
            assert!(root.to_string_lossy().contains(crate::CLAP_ID));
        }
    }

    use mxm_params::ErasedParam as _;
}
