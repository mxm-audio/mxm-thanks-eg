//! MXM Thanks EG's instrument: eight voices of the ported engines (`mxm-macro-osc`), each
//! with its envelopes and LFOs, the sixteen route slots, the voices' mix, and the resampler from the
//! engines' 48 kHz to the host's rate. Framework-free: plain values in, samples out.
//!
//! **The interface below is the contract with the plugin** (`plugins/mxm-thanks-eg`): the plugin
//! fills [`Settings`] from its parameters once per call, sends notes and expressions as they come,
//! and calls [`Instrument::render`] for each stretch of host samples between events. Changing it is
//! a change to both crates. The rules behind each part are in this crate's `AGENTS.md` and ops'
//! `plans/plan-mxm-thanks-eg.md` §5.

#![forbid(unsafe_code)]

mod envelope;
mod lfo;
mod resampler;
mod routing;
mod settings;
mod voice;
mod voices;

#[cfg(test)]
mod conformance;
#[cfg(test)]
mod tests;

pub use mxm_modulation::standard::Offer;

use resampler::Resampler;
use routing::Routing;
use voice::{BLOCK, DT, Shared};
use voices::Voices;

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

/// What a slot's amount of ±1 moves its target by, in the target's own unit: semitones on Pitch
/// (12), engines on Model (24), octaves on an LFO's Rate and an envelope's Time (4), the factor's
/// swing on Amplitude (×0…×2: 1), and 1 on every other target, its whole range (a control, Pan,
/// Level, an envelope's Rise and Fall as a fraction of their 0.5 ms–30 s range, a gate, an Input).
/// A Key slot reaches per octave of keyboard instead (the collection's standard). Which pairs a
/// slot moves at all is [`Target::offer`].
pub fn reach(target: Target) -> f32 {
    target.reach()
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

/// Aux, Spread and Volume after their smoothing: a one-pole of 5 ms a block, and a ramp across
/// each block.
#[derive(Clone, Copy, Debug)]
struct Smoothed {
    volume: f32,
    aux: f32,
    spread: f32,
}

/// The smoothing's step a block: `1 − e^(−0.25 ms / 5 ms)`.
const SMOOTHING: f32 = 0.048_770_58;

/// A note event, kept until the next render has taken in its settings: an event acts at the
/// next engine block either way, and so it acts under the settings that block plays with (the
/// mode, an LFO's Retrigger and Fade, Glide), not the last call's.
#[derive(Clone, Copy, Debug)]
enum Event {
    On(u8, f32, Option<i32>),
    Off(u8, Option<i32>),
    OffId(i32),
    ReleaseAll,
    Choke(Option<u8>),
    ChokeId(i32),
    Tuning(u8, Option<i32>, f32),
    TuningId(i32, f32),
}

/// Events kept between two renders; past it the oldest are applied at once.
const EVENTS: usize = 256;

/// The instrument: everything is allocated in [`Instrument::new`]; nothing after.
///
/// A note event (an on, an off, a choke, a tuning) takes effect at the next [`Instrument::render`],
/// under the settings that call brings; the expressions (bend, wheel, pressure) at once.
pub struct Instrument {
    host_rate: f32,
    telemetry: Telemetry,
    /// The last settings, every value finite and in range.
    settings: Settings,
    routing: Routing,
    voices: Voices,
    events: [Event; EVENTS],
    events_len: usize,
    /// The LFOs with Retrigger off: one each, for every voice.
    lfos: [lfo::Lfo; LFOS],
    /// The sources every voice shares, which the shared LFOs' Rate targets read.
    global: routing::Frame,
    bend: f32,
    wheel: f32,
    pressure: f32,
    /// The current engine block, mixed, and the next frame of it to play.
    block: [[f32; 2]; BLOCK],
    position: usize,
    /// `None` until the first block, which starts at its targets.
    smoothed: Option<Smoothed>,
    /// `None` at 48 kHz.
    resampler: Option<Resampler>,
}

impl Instrument {
    pub fn new() -> Box<Self> {
        Box::new(Self {
            host_rate: ENGINE_RATE,
            telemetry: Telemetry::default(),
            settings: Settings::default(),
            routing: Routing::new(),
            voices: Voices::new(),
            events: [Event::ReleaseAll; EVENTS],
            events_len: 0,
            lfos: std::array::from_fn(|l| lfo::Lfo::new(0x85eb_ca6b ^ (l as u32 + 1))),
            global: routing::Frame::new(),
            bend: 0.0,
            wheel: 0.0,
            pressure: 0.0,
            block: [[0.0; 2]; BLOCK],
            position: BLOCK,
            smoothed: None,
            resampler: None,
        })
    }

    /// From the plugin's `activate`: the host's rate. Prepares the resampler; may allocate.
    pub fn set_sample_rate(&mut self, host_rate: f32) {
        if !host_rate.is_finite() || host_rate <= 0.0 {
            return;
        }
        self.host_rate = host_rate;
        let rate = host_rate.round().min(u32::MAX as f32) as u32;
        self.resampler = (rate != ENGINE_RATE as u32).then(|| Resampler::new(rate));
    }

    /// The resampler's fixed latency at the current rate, in host samples; 0 at 48 kHz.
    pub fn latency_samples(&self) -> u32 {
        self.resampler.as_ref().map_or(0, Resampler::latency)
    }

    /// Every voice silent and every state cleared; allocates nothing.
    pub fn reset(&mut self) {
        self.voices.clear();
        self.events_len = 0;
        for l in &mut self.lfos {
            l.reset();
        }
        self.global.reset();
        self.bend = 0.0;
        self.wheel = 0.0;
        self.pressure = 0.0;
        self.block = [[0.0; 2]; BLOCK];
        self.position = BLOCK;
        self.smoothed = None;
        if let Some(r) = &mut self.resampler {
            r.reset();
        }
        self.telemetry = Telemetry::default();
    }

    pub fn note_on(&mut self, note: u8, velocity: f32, note_id: Option<i32>) {
        self.push(Event::On(note, velocity, note_id));
    }
    pub fn note_off(&mut self, note: u8, note_id: Option<i32>) {
        self.push(Event::Off(note, note_id));
    }
    /// `None`: every note.
    pub fn choke(&mut self, note: Option<u8>) {
        self.push(Event::Choke(note));
    }
    /// −1 to 1, the lever.
    pub fn set_bend(&mut self, bend: f32) {
        if bend.is_finite() {
            self.bend = mxm_modulation::standard::bend(bend);
        }
    }
    /// 0 to 1.
    pub fn set_wheel(&mut self, wheel: f32) {
        if wheel.is_finite() {
            self.wheel = mxm_modulation::standard::wheel(wheel);
        }
    }
    /// 0 to 1, channel pressure.
    pub fn set_pressure(&mut self, pressure: f32) {
        if pressure.is_finite() {
            self.pressure = mxm_modulation::standard::pressure(pressure);
        }
    }
    /// A per-note pitch expression, in semitones (the conventions' *Per-note pitch expression*).
    pub fn set_tuning(&mut self, note: u8, note_id: Option<i32>, semitones: f32) {
        self.push(Event::Tuning(note, note_id, semitones));
    }

    /// Every voice's gate off, as the note offs of every held key (CC 123).
    pub fn release_all(&mut self) {
        self.push(Event::ReleaseAll);
    }
    /// A note off that names only a note id: the voice playing it, or nothing.
    pub fn note_off_id(&mut self, note_id: i32) {
        self.push(Event::OffId(note_id));
    }
    /// A choke that names only a note id: the voice playing it, or nothing.
    pub fn choke_id(&mut self, note_id: i32) {
        self.push(Event::ChokeId(note_id));
    }
    /// A pitch expression that names only a note id: the voice playing it, or nothing.
    pub fn set_tuning_id(&mut self, note_id: i32, semitones: f32) {
        self.push(Event::TuningId(note_id, semitones));
    }

    fn push(&mut self, event: Event) {
        if self.events_len == EVENTS {
            self.apply_events();
        }
        self.events[self.events_len] = event;
        self.events_len += 1;
    }

    /// The kept events, in order, under the settings taken in.
    fn apply_events(&mut self) {
        let s = &self.settings;
        let v = &mut self.voices;
        for &event in &self.events[..self.events_len] {
            match event {
                Event::On(note, velocity, id) => v.note_on(s, note, velocity, id),
                Event::Off(note, id) => v.note_off(s, note, id),
                Event::OffId(id) => v.note_off_id(s, id),
                Event::ReleaseAll => v.release_all(),
                Event::Choke(note) => v.choke(note),
                Event::ChokeId(id) => v.choke_id(id),
                Event::Tuning(note, id, semitones) => v.set_tuning(note, id, semitones),
                Event::TuningId(id, semitones) => v.set_tuning_id(id, semitones),
            }
        }
        self.events_len = 0;
    }

    /// The kept events applied now, for a test that reads the voices without rendering.
    #[cfg(test)]
    pub(crate) fn apply(&mut self) {
        self.apply_events();
    }

    /// Renders `left.len()` host samples (`right` the same length).
    pub fn render(&mut self, settings: &Settings, left: &mut [f32], right: &mut [f32]) {
        self.absorb(settings);
        self.apply_events();
        let n = left.len().min(right.len());
        let mut peak = 0.0f32;
        if let Some(mut resampler) = self.resampler.take() {
            for (l, r) in left[..n].iter_mut().zip(&mut right[..n]) {
                while resampler.needs_input() {
                    let [a, b] = self.next_frame();
                    resampler.push(a, b);
                }
                (*l, *r) = resampler.output();
                peak = peak.max(l.abs()).max(r.abs());
            }
            self.resampler = Some(resampler);
        } else {
            for (l, r) in left[..n].iter_mut().zip(&mut right[..n]) {
                [*l, *r] = self.next_frame();
                peak = peak.max(l.abs()).max(r.abs());
            }
        }
        left[n..].fill(0.0);
        right[n..].fill(0.0);
        self.telemetry = Telemetry {
            peak,
            voices_sounding: self.voices.voices.iter().filter(|v| v.active).count(),
        };
    }

    pub fn telemetry(&self) -> Telemetry {
        self.telemetry
    }

    /// Whether any voice still sounds: the plugin's tail. A note event not yet rendered counts.
    pub fn is_sounding(&self) -> bool {
        self.events_len > 0
            || self.voices.voices.iter().any(|v| v.active)
            || self.resampler.as_ref().is_some_and(Resampler::ringing)
    }

    /// Host samples until every voice is idle, as best the voices can say: each released voice's
    /// slowest routed envelope to rest, then the gate's tail, then the resampler's. 0 when nothing
    /// sounds; `u32::MAX` while a key is held, an envelope cycles in Gate mode, or a note event
    /// waits for the next render.
    pub fn tail_samples(&self) -> u32 {
        if !self.is_sounding() {
            return 0;
        }
        if self.events_len > 0 {
            return u32::MAX;
        }
        let mut seconds = 0.0f32;
        for v in &self.voices.voices {
            match v.tail_seconds(&self.settings, &self.routing) {
                Some(s) => seconds = seconds.max(s),
                None => return u32::MAX,
            }
        }
        let samples = (seconds * self.host_rate).ceil() + 2.0 * self.latency_samples() as f32;
        samples.min(u32::MAX as f32) as u32
    }

    /// The settings taken in: a non-finite value ignored, a change of mode releasing every voice,
    /// the slots' grid rebuilt if a slot changed.
    fn absorb(&mut self, new: &Settings) {
        let mode = self.settings.mode;
        settings::absorb(&mut self.settings, new);
        if self.settings.mode != mode {
            self.voices.release_all();
        }
        self.routing.update(&self.settings.slots);
    }

    /// The next 48 kHz frame, mixed: a new block when the last is played.
    #[inline]
    fn next_frame(&mut self) -> [f32; 2] {
        if self.position == BLOCK {
            self.render_block();
        }
        let frame = self.block[self.position];
        self.position += 1;
        frame
    }

    /// One engine block: the shared sources and LFOs, every sounding voice, Volume.
    fn render_block(&mut self) {
        let s = &self.settings;
        let (from, to) = match self.smoothed {
            None => {
                let snap = Smoothed {
                    volume: s.volume,
                    aux: s.aux,
                    spread: s.spread,
                };
                (snap, snap)
            }
            Some(was) => (
                was,
                Smoothed {
                    volume: was.volume + (s.volume - was.volume) * SMOOTHING,
                    aux: was.aux + (s.aux - was.aux) * SMOOTHING,
                    spread: was.spread + (s.spread - was.spread) * SMOOTHING,
                },
            ),
        };
        self.smoothed = Some(to);

        let g = &mut self.global;
        g.begin_sample();
        g.write(Source::Wheel.index(), self.wheel);
        g.write(Source::Pressure.index(), self.pressure);
        g.write(Source::Bend.index(), self.bend);
        let mut lfos = [0.0; LFOS];
        for (l, value) in lfos.iter_mut().enumerate() {
            let ls = &s.lfos[l];
            if !ls.retrigger {
                let octaves = self.routing.sum(Target::lfo_rate(l), g);
                *value = self.lfos[l].advance(ls.shape, ls.rate_hz * octaves.exp2(), DT);
                g.write(Source::lfo(l).index(), *value);
            }
        }

        let shared = Shared {
            settings: &self.settings,
            routing: &self.routing,
            wheel: self.wheel,
            pressure: self.pressure,
            bend: self.bend,
            lfos,
            aux: to.aux,
            spread: to.spread,
        };
        let mut out = [[0.0f32; 2]; BLOCK];
        for v in &mut self.voices.voices {
            if v.active {
                v.process(&shared, &mut out);
            }
        }
        for (i, o) in out.iter_mut().enumerate() {
            let t = (i + 1) as f32 / BLOCK as f32;
            let volume = from.volume + (to.volume - from.volume) * t;
            o[0] *= volume;
            o[1] *= volume;
        }
        self.block = out;
        self.position = 0;
    }
}
