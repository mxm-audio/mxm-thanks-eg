//! One of the instrument's voices: an engine voice (the module, `mxm_macro_osc::Voice`)
//! with the case around it: the note, glide and tuning on V/OCT, the gate on TRIG, three
//! envelopes, three LFOs (or the shared ones), the slots' targets, and the mix of OUT and AUX.
//!
//! Per engine block of 12 frames, in order (the crate's `AGENTS.md`, *The routing*): the
//! performance sources, the LFOs, envelopes 1, 2 and 3, then the voice's targets, the engine's
//! render, and last the engine's own decay envelope as the *Decay* source, which every reader so
//! reads one block late.

use mxm_macro_osc::dsp::K_BLOCK_SIZE;
use mxm_macro_osc::{Frame as EngineFrame, Modulations, Patch, Voice as Engine};
use mxm_modulation::standard;

use crate::envelope::{Envelope, Inputs};
use crate::lfo::{Fade, Lfo};
use crate::routing::{self, KEY_UNIT, Routing};
use crate::{ENGINE_RATE, ENVELOPES, LFOS, Settings, Source, Target};

/// The engines' block.
pub(crate) const BLOCK: usize = K_BLOCK_SIZE;
/// A block's length, seconds: the modulation's step.
pub(crate) const DT: f32 = BLOCK as f32 / ENGINE_RATE;
/// Blocks a voice's gate stays low before it may go idle: the engine's trigger reaches it five
/// blocks late (`voice.cc`'s delay line), so a short note's edge is still on its way until then.
const QUIET_BLOCKS: u32 = 8;
/// Plaits' silence is the frame +1, what its output stage makes of zero; a frame within one step
/// of it (−90 dBFS) counts as silent, since the six-op engines settle into a one-step residue
/// that never reaches +1.
#[inline]
fn near_silence(x: i16) -> bool {
    (i32::from(x) - 1).abs() <= 1
}
/// The low-pass gate is closed for good below this gain: an engine's output times it is under
/// half a step of the 16-bit frame, and with the key up nothing opens it again.
const GATE_CLOSED: f32 = 1e-6;
/// Silent blocks an engine that envelopes itself (the gate bypassed) must give before it is
/// idle: 128 ms, longer than half a period of the lowest key and than a word's pause, so a tail
/// of a step or two is not cut at its zero crossings. (One silent block, the first rule, cut
/// those tails.)
const SILENT_BLOCKS: u32 = 512;
/// A released voice still sounding this long after its key went up, its envelopes at rest, is
/// faded out and stopped, so a released note always ends: only an engine that never ends on its
/// own gets there (the chiptune engine with its envelope at noon, which on the module drones).
/// Thirty seconds, the longest time an envelope or the gate sets.
const GUARD_BLOCKS: u32 = (30.0 / DT) as u32;
/// That fade, 10 ms.
const GUARD_FADE: f32 = 0.01;
/// The engines' quantiser scales the model's CV by the engines less one.
const MODEL_CV_SCALE: f32 = 23.0;
/// Glide: the time to within 1 % of the interval is the Glide setting.
const GLIDE_TIME_CONSTANTS: f32 = 4.605_17;

/// What the instrument hands every voice for a block.
pub(crate) struct Shared<'a> {
    pub settings: &'a Settings,
    pub routing: &'a Routing,
    pub wheel: f32,
    pub pressure: f32,
    pub bend: f32,
    /// The shared LFOs' outputs this block (an LFO with Retrigger off).
    pub lfos: [f32; LFOS],
    /// Aux and Spread, smoothed.
    pub aux: f32,
    pub spread: f32,
}

/// What OUT and AUX each give the left and right channels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Mix {
    pub out_left: f32,
    pub aux_left: f32,
    pub out_right: f32,
    pub aux_right: f32,
}

impl Mix {
    /// Aux fades OUT to AUX; Spread then places OUT left and AUX right (0: the mix in both
    /// channels; 1: OUT left, AUX right); Amplitude scales; Pan balances (centre is unity in both
    /// channels, so a voice at rest is its frames exactly).
    pub(crate) fn new(aux: f32, spread: f32, amplitude: f32, pan: f32) -> Self {
        let (out, aux) = (1.0 - aux, aux);
        let keep = 1.0 - spread;
        let left = (1.0 - pan.max(0.0)) * amplitude;
        let right = (1.0 + pan.min(0.0)) * amplitude;
        Self {
            out_left: (keep * out + spread) * left,
            aux_left: keep * aux * left,
            out_right: keep * out * right,
            aux_right: (keep * aux + spread) * right,
        }
    }

    fn scaled(self, gain: f32) -> Self {
        Self {
            out_left: self.out_left * gain,
            aux_left: self.aux_left * gain,
            out_right: self.out_right * gain,
            aux_right: self.aux_right * gain,
        }
    }

    #[inline]
    fn lerp(self, to: Self, t: f32) -> Self {
        Self {
            out_left: self.out_left + (to.out_left - self.out_left) * t,
            aux_left: self.aux_left + (to.aux_left - self.aux_left) * t,
            out_right: self.out_right + (to.out_right - self.out_right) * t,
            aux_right: self.aux_right + (to.aux_right - self.aux_right) * t,
        }
    }
}

pub(crate) struct Voice {
    engine: Box<Engine>,
    /// The engine is as `init` left it: nothing rendered since.
    fresh: bool,
    /// Rendered; an idle voice is not, and adds nothing.
    pub active: bool,
    /// The key is held.
    pub gate: bool,
    pub note: u8,
    pub id: Option<i32>,
    /// Which note-on this voice took, counting from one.
    pub started: u64,
    /// The published Velocity (`v − 1`) and Random (`2u − 1`), drawn per note.
    velocity: f32,
    random: f32,
    /// Per-note pitch expression, semitones.
    pub tuning: f32,
    /// The glide's offset from the note, semitones, decaying to zero.
    glide: f32,
    /// TRIG low for the next block, so the engine sees a new edge.
    retrigger: bool,
    /// TRIG was high in the last block rendered.
    trigger_high: bool,
    /// A new note for the envelopes in the next block.
    note_on: bool,
    /// Blocks rendered with the gate low.
    quiet: u32,
    /// Blocks in a row whose frames were all Plaits' silence.
    silent: u32,
    /// The guard's fade, once it has begun.
    ending: Option<f32>,
    envelopes: [Envelope; ENVELOPES],
    lfos: [Lfo; LFOS],
    fades: [Fade; LFOS],
    frame: routing::Frame,
    frames: [EngineFrame; BLOCK],
    /// The last block's mix; `None` before the first, which then starts at its own.
    mix: Option<Mix>,
    #[cfg(test)]
    pub(crate) last: (Patch, Modulations, Mix),
    /// Notes the guard ended.
    #[cfg(test)]
    pub(crate) guarded: u32,
}

impl Voice {
    /// Voice `index`: its engine and its LFOs' random streams, each its own.
    pub(crate) fn new(index: usize) -> Self {
        Self {
            engine: Engine::new(),
            fresh: true,
            active: false,
            gate: false,
            note: 60,
            id: None,
            started: 0,
            velocity: 0.0,
            random: 0.0,
            tuning: 0.0,
            glide: 0.0,
            retrigger: false,
            trigger_high: false,
            note_on: false,
            quiet: 0,
            silent: 0,
            ending: None,
            envelopes: [Envelope::new(); ENVELOPES],
            lfos: std::array::from_fn(|l| Lfo::new(lfo_seed(index, l))),
            fades: [Fade::default(); LFOS],
            frame: routing::Frame::new(),
            frames: [EngineFrame::default(); BLOCK],
            mix: None,
            #[cfg(test)]
            last: (Patch::default(), Modulations::default(), Mix::default()),
            #[cfg(test)]
            guarded: 0,
        }
    }

    /// Every state as `new` made it; allocates nothing.
    pub(crate) fn clear(&mut self) {
        self.engine.clear();
        self.fresh = true;
        self.active = false;
        self.gate = false;
        self.note = 60;
        self.id = None;
        self.started = 0;
        self.velocity = 0.0;
        self.random = 0.0;
        self.tuning = 0.0;
        self.glide = 0.0;
        self.retrigger = false;
        self.trigger_high = false;
        self.note_on = false;
        self.quiet = 0;
        self.silent = 0;
        self.ending = None;
        for e in &mut self.envelopes {
            e.reset();
        }
        for l in &mut self.lfos {
            l.reset();
        }
        self.fades = [Fade::default(); LFOS];
        self.frame.reset();
        self.mix = None;
    }

    /// The glided pitch, semitones.
    pub(crate) fn pitch(&self) -> f32 {
        f32::from(self.note) + self.glide
    }

    /// A new note: an idle voice starts as the engine's `init` leaves it; a sounding one keeps its
    /// state and restarts its envelopes' attacks from where they are, with TRIG dropped for a block
    /// if it was high. `glide_from` is the pitch it glides from, if it glides.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn start(
        &mut self,
        settings: &Settings,
        note: u8,
        id: Option<i32>,
        velocity: f32,
        random: f32,
        order: u64,
        glide_from: Option<f32>,
    ) {
        if !self.active {
            if !self.fresh {
                self.engine.init();
                self.fresh = true;
            }
            for e in &mut self.envelopes {
                e.reset();
            }
            self.frame.reset();
            self.mix = None;
            self.trigger_high = false;
        }
        self.retrigger = self.trigger_high;
        self.active = true;
        self.gate = true;
        self.note_on = true;
        self.quiet = 0;
        self.silent = 0;
        self.ending = None;
        self.glide = glide_from.map_or(0.0, |from| from - f32::from(note));
        self.note = note;
        self.id = id;
        self.started = order;
        self.velocity = velocity;
        self.random = random;
        self.tuning = 0.0;
        for (l, s) in settings.lfos.iter().enumerate() {
            if s.retrigger {
                self.lfos[l].restart();
            }
            self.fades[l].restart(s.fade_s);
        }
    }

    /// The held note moves to another without a new trigger (Legato's tie, or a release
    /// returning to the key held before it), gliding if Glide is set.
    pub(crate) fn tie(&mut self, settings: &Settings, note: u8, id: Option<i32>) {
        self.glide = if settings.glide_s > 0.0 {
            self.pitch() - f32::from(note)
        } else {
            0.0
        };
        self.note = note;
        self.id = id;
        self.tuning = 0.0;
    }

    pub(crate) fn release(&mut self) {
        self.gate = false;
    }

    /// Silent at once: idle, and the next note starts it afresh.
    pub(crate) fn choke(&mut self) {
        self.gate = false;
        self.active = false;
        self.trigger_high = false;
    }

    /// Whether the voice answers to a note: by its id when both have one, else by its key.
    pub(crate) fn answers(&self, note: u8, id: Option<i32>) -> bool {
        match (id, self.id) {
            (Some(a), Some(b)) => a == b,
            _ => self.note == note,
        }
    }

    /// The source's value in the voice's frame, as a slot would read it now.
    #[cfg(test)]
    pub(crate) fn published(&self, source: Source) -> f32 {
        self.frame.read(source.index())
    }

    /// One block: the modulation, the engine's 12 frames, and their mix added into `out`.
    pub(crate) fn process(&mut self, shared: &Shared, out: &mut [[f32; 2]; BLOCK]) {
        let s = shared.settings;
        let r = shared.routing;
        let key = self.pitch();
        if s.glide_s > 0.0 && self.glide != 0.0 {
            self.glide *= (-DT * GLIDE_TIME_CONSTANTS / s.glide_s).exp();
            if self.glide.abs() < 1e-4 {
                self.glide = 0.0;
            }
        } else {
            self.glide = 0.0;
        }

        // The performance sources.
        let f = &mut self.frame;
        f.begin_sample();
        f.write(Source::Key.index(), standard::key(key, KEY_UNIT));
        f.write(Source::Velocity.index(), self.velocity);
        f.write(Source::Wheel.index(), shared.wheel);
        f.write(Source::Pressure.index(), shared.pressure);
        f.write(Source::Bend.index(), shared.bend);
        f.write(Source::Random.index(), self.random);

        // The LFOs: a voice's own with Retrigger on, else the shared one; each faded in.
        for l in 0..LFOS {
            let ls = &s.lfos[l];
            let value = if ls.retrigger {
                let octaves = r.sum(Target::lfo_rate(l), f);
                self.lfos[l].advance(ls.shape, ls.rate_hz * octaves.exp2(), DT)
            } else {
                shared.lfos[l]
            };
            let depth = self.fades[l].advance(ls.fade_s, DT);
            f.write(Source::lfo(l).index(), value * depth);
        }

        // Envelopes 1, 2 and 3.
        for e in 0..ENVELOPES {
            let t = Target::envelope(e);
            let inputs = Inputs {
                note_gate: self.gate,
                note_on: self.note_on,
                rise: r.sum(t.rise, f),
                fall: r.sum(t.fall, f),
                time: r.sum(t.time, f),
                cycle: r.sum(t.cycle, f),
                trigger: r.sum(t.trigger, f),
                input: r.sum(t.input, f),
            };
            let env = &mut self.envelopes[e];
            env.process(&s.envelopes[e], &inputs, DT);
            let [level, rise, cycle] = Source::envelope(e);
            let eor = if env.end_of_rise() { 1.0 } else { 0.0 };
            f.write(level.index(), env.value());
            f.write(rise.index(), eor);
            f.write(cycle.index(), 1.0 - eor);
        }
        self.note_on = false;

        // The voice's targets, into the module's knobs and jacks.
        let patch = Patch {
            note: s.frequency_st + s.fine_st,
            harmonics: s.harmonics + r.sum(Target::Harmonics, f),
            timbre: s.timbre + r.sum(Target::Timbre, f),
            morph: s.morph + r.sum(Target::Morph, f),
            frequency_modulation_amount: s.fm_amount,
            timbre_modulation_amount: s.timbre_amount,
            morph_modulation_amount: s.morph_amount,
            engine: s.model,
            decay: (s.decay + r.sum(Target::GateDecay, f)).clamp(0.0, 1.0),
            lpg_colour: (s.colour + r.sum(Target::Colour, f)).clamp(0.0, 1.0),
        };
        let trigger = self.gate && !self.retrigger;
        self.retrigger = false;
        self.trigger_high = trigger;
        let level_patched = r.patched(Target::Level);
        let modulations = Modulations {
            engine: r.sum(Target::Model, f) / MODEL_CV_SCALE,
            note: key + shared.bend * s.bend_range_st + self.tuning + r.sum(Target::Pitch, f),
            trigger: if trigger { 1.0 } else { 0.0 },
            level: if level_patched {
                r.sum(Target::Level, f)
            } else {
                0.0
            },
            trigger_patched: true,
            level_patched,
            ..Modulations::default()
        };
        let mix = Mix::new(
            (shared.aux + r.sum(Target::Aux, f)).clamp(0.0, 1.0),
            shared.spread,
            standard::amplitude_factor(r.sum(Target::Amplitude, f)),
            r.sum(Target::Pan, f).clamp(-1.0, 1.0),
        );
        let mix = match self.ending {
            Some(gain) => mix.scaled(gain),
            None => mix,
        };

        self.engine.render(&patch, &modulations, &mut self.frames);
        self.fresh = false;
        self.frame
            .write(Source::Decay.index(), self.engine.decay_envelope());

        // The frames as the module's output stage reads them, mixed.
        let from = self.mix.unwrap_or(mix);
        let heard = |m: &Mix| {
            (
                m.out_left != 0.0 || m.out_right != 0.0,
                m.aux_left != 0.0 || m.aux_right != 0.0,
            )
        };
        let (out_heard, aux_heard) = {
            let (a, b) = (heard(&from), heard(&mix));
            (a.0 || b.0, a.1 || b.1)
        };
        for (i, (frame, o)) in self.frames.iter().zip(out.iter_mut()).enumerate() {
            let c = from.lerp(mix, (i + 1) as f32 / BLOCK as f32);
            let main = -f32::from(frame.out) / 32767.0;
            let aux = -f32::from(frame.aux) / 32767.0;
            o[0] += c.out_left * main + c.aux_left * aux;
            o[1] += c.out_right * main + c.aux_right * aux;
        }
        self.mix = Some(mix);
        #[cfg(test)]
        {
            self.last = (patch, modulations, mix);
        }

        // Idle once the key is up, its trigger through, its envelopes at rest (those a slot
        // reads) and its frames silent: for a block once the gate has closed, or, with the gate
        // bypassed, for 128 ms. Else, after 30 s, the guard.
        if self.gate {
            self.quiet = 0;
        } else {
            self.quiet = self.quiet.saturating_add(1);
        }
        // An output the mix does not use adds nothing, so only those it uses must be silent.
        if self
            .frames
            .iter()
            .all(|fr| (!out_heard || near_silence(fr.out)) && (!aux_heard || near_silence(fr.aux)))
        {
            self.silent = self.silent.saturating_add(1);
        } else {
            self.silent = 0;
        }
        // Through the gate, the gate must have closed: a sparse engine (the particles) is silent
        // between its grains while the gate is still open. Bypassed, its gain stays at 1.
        let gain = self.engine.low_pass_gate_gain();
        let done = if gain >= 1.0 {
            self.silent >= SILENT_BLOCKS
        } else {
            gain < GATE_CLOSED && self.silent >= 1
        };
        if !self.gate && self.quiet >= QUIET_BLOCKS && done && self.envelopes_at_rest(r) {
            self.active = false;
        } else if let Some(gain) = self.ending {
            let gain = gain - DT / GUARD_FADE;
            if gain <= 0.0 {
                self.active = false;
                self.ending = None;
                #[cfg(test)]
                {
                    self.guarded += 1;
                }
            } else {
                self.ending = Some(gain);
            }
        } else if !self.gate && self.quiet >= GUARD_BLOCKS && self.envelopes_at_rest(r) {
            self.ending = Some(1.0 - DT / GUARD_FADE);
        }
    }

    /// Seconds until the voice is idle, as best it can say: its slowest routed envelope's time to
    /// rest, then the gate's tail (the vactrol's slowest fall from full to below one step of the
    /// 16-bit frame, at the Decay and Colour set). `None` while its key is held or an envelope
    /// cycles for ever.
    pub(crate) fn tail_seconds(&self, s: &Settings, r: &Routing) -> Option<f32> {
        if !self.active {
            return Some(0.0);
        }
        if self.gate {
            return None;
        }
        let mut envelopes = 0.0f32;
        for (e, env) in self.envelopes.iter().enumerate() {
            if Source::envelope(e).iter().any(|&src| r.reads(src)) {
                envelopes = envelopes.max(env.remaining(&s.envelopes[e])?);
            }
        }
        // `voice.cc`: per block the vactrol closes by `0.005 · 2^((−72·decay + 12·colour) / 12)`
        // at its slowest; ln(65 534) such steps take it from full to a step of the frame.
        let per_block = 0.005 * ((-72.0 * s.decay + 12.0 * s.colour) / 12.0).exp2();
        let gate = 11.09 / per_block * DT;
        Some(envelopes + gate + (QUIET_BLOCKS + SILENT_BLOCKS) as f32 * DT)
    }

    /// Every envelope a slot reads is still: at rest, or in Follow mode holding its input's level
    /// (one following Key never rests, and its voice must still be freed once silent).
    fn envelopes_at_rest(&self, r: &Routing) -> bool {
        self.envelopes
            .iter()
            .enumerate()
            .all(|(e, env)| env.still() || !Source::envelope(e).iter().any(|&s| r.reads(s)))
    }
}

/// Each voice's LFO has its own random stream.
fn lfo_seed(voice: usize, lfo: usize) -> u32 {
    0x2545_f491_u32.wrapping_mul((voice * LFOS + lfo + 1) as u32)
}
