//! MXM Thanks EG's instrument: eight voices of the ported engines (`mxm-thanks-eg-engines`), each
//! with its envelopes and LFOs, the sixteen route slots, the voices' mix, and the resampler from the
//! engines' 48 kHz to the host's rate. Framework-free: plain values in, samples out.
//!
//! **The interface below is the contract with the plugin** (`plugins/mxm-thanks-eg`): the plugin
//! fills [`Settings`] from its parameters once per call, sends notes and expressions as they come,
//! and calls [`Instrument::render`] for each stretch of host samples between events. Changing it is
//! a change to both crates. The rules behind each part are in this crate's `AGENTS.md` and ops'
//! `plans/plan-mxm-thanks-eg.md` §5.

#![forbid(unsafe_code)]

/// The engines' rate: every voice runs at it, as the module does.
pub const ENGINE_RATE: f32 = 48_000.0;
/// Voices in Poly mode.
pub const VOICES: usize = 8;
/// Route slots.
pub const SLOTS: usize = 16;
/// Envelopes, and LFOs, per voice.
pub const ENVELOPES: usize = 3;
pub const LFOS: usize = 3;

/// How the voices take notes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    /// Eight voices; a new note takes a free voice, or the oldest.
    #[default]
    Poly,
    /// One voice; every new note retriggers it.
    Mono,
    /// One voice; a note played over a held one glides to it without a new trigger.
    Legato,
}

/// What drives an envelope (the plan's §5.4).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EnvelopeMode {
    /// The note's gate: attack on note on, release on note off.
    #[default]
    Note,
    /// Only its Trigger target: rising starts the attack, it sustains while high, falling releases.
    Gate,
    /// No trigger: it follows its Input target, rising at its attack rate and falling at its
    /// release rate.
    Follow,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LfoShape {
    #[default]
    Sine,
    Triangle,
    SawUp,
    SawDown,
    Square,
    SampleAndHold,
    SmoothRandom,
}

/// A route's source: nineteen (the plan's §5.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Envelope1,
    Envelope2,
    Envelope3,
    /// High from the end of the attack until the envelope is back at rest or attacks again.
    Envelope1Rise,
    /// The complement of the end of rise: high at rest and through the attack.
    Envelope1Cycle,
    Envelope2Rise,
    Envelope2Cycle,
    Envelope3Rise,
    Envelope3Cycle,
    Lfo1,
    Lfo2,
    Lfo3,
    /// The voice's own decay envelope, the module's internal one.
    Decay,
    Key,
    Velocity,
    Wheel,
    Pressure,
    Bend,
    Random,
}

impl Source {
    pub const ALL: [Self; 19] = [
        Self::Envelope1,
        Self::Envelope2,
        Self::Envelope3,
        Self::Envelope1Rise,
        Self::Envelope1Cycle,
        Self::Envelope2Rise,
        Self::Envelope2Cycle,
        Self::Envelope3Rise,
        Self::Envelope3Cycle,
        Self::Lfo1,
        Self::Lfo2,
        Self::Lfo3,
        Self::Decay,
        Self::Key,
        Self::Velocity,
        Self::Wheel,
        Self::Pressure,
        Self::Bend,
        Self::Random,
    ];
}

/// A route's target: thirty-two (the plan's §5.3 and §5.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Pitch,
    Harmonics,
    Timbre,
    Morph,
    Model,
    /// The gate's LEVEL: a slot aimed here is LEVEL patched, and the gate follows it.
    Level,
    Amplitude,
    Pan,
    Aux,
    GateDecay,
    Colour,
    Lfo1Rate,
    Lfo2Rate,
    Lfo3Rate,
    Envelope1Rise,
    Envelope1Fall,
    Envelope1Time,
    Envelope1Cycle,
    Envelope1Trigger,
    Envelope1Input,
    Envelope2Rise,
    Envelope2Fall,
    Envelope2Time,
    Envelope2Cycle,
    Envelope2Trigger,
    Envelope2Input,
    Envelope3Rise,
    Envelope3Fall,
    Envelope3Time,
    Envelope3Cycle,
    Envelope3Trigger,
    Envelope3Input,
}

impl Target {
    pub const ALL: [Self; 32] = [
        Self::Pitch,
        Self::Harmonics,
        Self::Timbre,
        Self::Morph,
        Self::Model,
        Self::Level,
        Self::Amplitude,
        Self::Pan,
        Self::Aux,
        Self::GateDecay,
        Self::Colour,
        Self::Lfo1Rate,
        Self::Lfo2Rate,
        Self::Lfo3Rate,
        Self::Envelope1Rise,
        Self::Envelope1Fall,
        Self::Envelope1Time,
        Self::Envelope1Cycle,
        Self::Envelope1Trigger,
        Self::Envelope1Input,
        Self::Envelope2Rise,
        Self::Envelope2Fall,
        Self::Envelope2Time,
        Self::Envelope2Cycle,
        Self::Envelope2Trigger,
        Self::Envelope2Input,
        Self::Envelope3Rise,
        Self::Envelope3Fall,
        Self::Envelope3Time,
        Self::Envelope3Cycle,
        Self::Envelope3Trigger,
        Self::Envelope3Input,
    ];
}

/// One route slot. In use when both its source and target are set; the amount is a signed
/// fraction of the target's reach. Two slots on one source and target add.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Slot {
    pub source: Option<Source>,
    pub target: Option<Target>,
    pub amount: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnvelopeSettings {
    pub attack_s: f32,
    pub decay_s: f32,
    pub sustain: f32,
    pub release_s: f32,
    /// −1 logarithmic, 0 linear, +1 exponential: one curve for every segment.
    pub response: f32,
    pub cycle: bool,
    pub mode: EnvelopeMode,
}

impl Default for EnvelopeSettings {
    fn default() -> Self {
        Self {
            attack_s: 0.005,
            decay_s: 0.3,
            sustain: 0.7,
            release_s: 0.3,
            response: 0.0,
            cycle: false,
            mode: EnvelopeMode::Note,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LfoSettings {
    /// Already resolved by the plugin, tempo sync included.
    pub rate_hz: f32,
    pub shape: LfoShape,
    /// Restart at each note, per voice; otherwise one LFO for all voices, free-running.
    pub retrigger: bool,
    pub fade_s: f32,
}

impl Default for LfoSettings {
    fn default() -> Self {
        Self {
            rate_hz: 5.0,
            shape: LfoShape::Sine,
            retrigger: false,
            fade_s: 0.0,
        }
    }
}

/// Every setting, as plain values; the plugin fills it from its parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    /// 0–23, in Plaits' registration order.
    pub model: i32,
    pub frequency_st: f32,
    pub fine_st: f32,
    pub harmonics: f32,
    pub timbre: f32,
    pub morph: f32,
    /// The module's three attenuverters: how far its decay envelope moves Pitch, Timbre and Morph.
    pub fm_amount: f32,
    pub timbre_amount: f32,
    pub morph_amount: f32,
    /// The gate's decay and colour.
    pub decay: f32,
    pub colour: f32,
    /// 0: OUT; 1: AUX.
    pub aux: f32,
    /// 0: OUT and AUX in both channels; 1: OUT left, AUX right.
    pub spread: f32,
    pub mode: Mode,
    pub glide_s: f32,
    pub bend_range_st: f32,
    /// Linear gain.
    pub volume: f32,
    pub envelopes: [EnvelopeSettings; ENVELOPES],
    pub lfos: [LfoSettings; LFOS],
    pub slots: [Slot; SLOTS],
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            model: 8,
            frequency_st: 0.0,
            fine_st: 0.0,
            harmonics: 0.5,
            timbre: 0.5,
            morph: 0.5,
            fm_amount: 0.0,
            timbre_amount: 0.0,
            morph_amount: 0.0,
            decay: 0.5,
            colour: 0.5,
            aux: 0.0,
            spread: 0.0,
            mode: Mode::Poly,
            glide_s: 0.0,
            bend_range_st: 2.0,
            volume: 0.35,
            envelopes: [EnvelopeSettings::default(); ENVELOPES],
            lfos: [LfoSettings::default(); LFOS],
            slots: [Slot::default(); SLOTS],
        }
    }
}

/// What the editor shows of the sound, read after each render.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Telemetry {
    pub peak: f32,
    pub voices_sounding: usize,
}

/// The instrument: everything is allocated in [`Instrument::new`]; nothing after.
pub struct Instrument {
    host_rate: f32,
    telemetry: Telemetry,
}

impl Instrument {
    pub fn new() -> Box<Self> {
        Box::new(Self {
            host_rate: ENGINE_RATE,
            telemetry: Telemetry::default(),
        })
    }

    /// From the plugin's `activate`: the host's rate. Prepares the resampler; may allocate.
    pub fn set_sample_rate(&mut self, host_rate: f32) {
        self.host_rate = host_rate;
    }

    /// The resampler's fixed latency at the current rate, in host samples; 0 at 48 kHz.
    pub fn latency_samples(&self) -> u32 {
        0
    }

    /// Every voice silent and every state cleared; allocates nothing.
    pub fn reset(&mut self) {}

    pub fn note_on(&mut self, _note: u8, _velocity: f32, _note_id: Option<i32>) {}
    pub fn note_off(&mut self, _note: u8, _note_id: Option<i32>) {}
    /// `None`: every note.
    pub fn choke(&mut self, _note: Option<u8>) {}
    /// −1 to 1, the lever.
    pub fn set_bend(&mut self, _bend: f32) {}
    /// 0 to 1.
    pub fn set_wheel(&mut self, _wheel: f32) {}
    /// 0 to 1, channel pressure.
    pub fn set_pressure(&mut self, _pressure: f32) {}
    /// A per-note pitch expression, in semitones (the conventions' *Per-note pitch expression*).
    pub fn set_tuning(&mut self, _note: u8, _note_id: Option<i32>, _semitones: f32) {}

    /// Renders `left.len()` host samples (`right` the same length).
    pub fn render(&mut self, _settings: &Settings, left: &mut [f32], right: &mut [f32]) {
        left.fill(0.0);
        right.fill(0.0);
    }

    pub fn telemetry(&self) -> Telemetry {
        self.telemetry
    }

    /// Whether any voice still sounds: the plugin's tail.
    pub fn is_sounding(&self) -> bool {
        false
    }
}
