//! The instrument's tests, through its interface and, where a test needs to see inside a block,
//! the voices' own state.

use mxm_thanks_eg_engines::{Frame, Modulations, Patch, Voice as Engine};

use crate::voice::{BLOCK, Mix};
use crate::{
    EnvelopeMode, Instrument, LfoShape, Mode, SLOTS, Settings, Slot, Source, Target, VOICES,
};

const ENGINES: i32 = 24;

fn slots(list: &[(Source, Target, f32)]) -> [Slot; SLOTS] {
    let mut slots = [Slot::default(); SLOTS];
    for (slot, &(source, target, amount)) in slots.iter_mut().zip(list) {
        *slot = Slot {
            source: Some(source),
            target: Some(target),
            amount,
        };
    }
    slots
}

/// `n` host samples of both channels.
fn render(instrument: &mut Instrument, settings: &Settings, n: usize) -> (Vec<f32>, Vec<f32>) {
    let (mut l, mut r) = (vec![0.0; n], vec![0.0; n]);
    instrument.render(settings, &mut l, &mut r);
    (l, r)
}

/// One engine block at 48 kHz, so a test reads the voices between blocks.
fn block(instrument: &mut Instrument, settings: &Settings) {
    render(instrument, settings, BLOCK);
}

fn last(instrument: &Instrument, voice: usize) -> (Patch, Modulations, Mix) {
    instrument.voices.voices[voice].last
}

fn published(instrument: &Instrument, voice: usize, source: Source) -> f32 {
    instrument.voices.voices[voice].published(source)
}

/// The voice that took the last note.
fn newest(instrument: &Instrument) -> usize {
    (0..VOICES)
        .max_by_key(|&v| instrument.voices.voices[v].started)
        .unwrap()
}

fn sounding(instrument: &Instrument) -> usize {
    instrument.voices.voices.iter().filter(|v| v.active).count()
}

/// **The link to the engines' reference**: at 48 kHz, one voice and no slots, the left channel
/// is the engine voice's OUT frames read as `−frame / 32767`, times Volume, exactly, for every
/// engine; Spread at 0 and Aux at 0 put the same in the right. The engine voice is driven as the
/// module's jacks: V/OCT the key, TRIG high while it is held, LEVEL unpatched. Rendered in
/// stretches that do not fall on the blocks. Once the voice goes idle the instrument is exact
/// zero, and what it cut is the engine's residue: OUT within four steps of Plaits' silence
/// (−78 dBFS before Volume; the six-op engines' noise floor flickers there for seconds). AUX is
/// not mixed at Aux 0, so it is not read.
#[test]
fn one_voice_at_48_khz_is_the_engine_voice_exactly() {
    for model in 0..ENGINES {
        let settings = Settings {
            model,
            decay: 0.3,
            volume: 0.35,
            ..Settings::default()
        };
        let mut instrument = Instrument::new();
        let mut engine = Engine::new();
        let mut frames = [Frame::default(); BLOCK];
        let held = 24_000;
        let total = 960_000;
        let stretch = 100;
        let mut gone = false;
        instrument.note_on(60, 1.0, None);
        let mut done = 0;
        let mut position = BLOCK;
        let mut blocks = 0;
        while done < total {
            if done == held {
                instrument.note_off(60, None);
            }
            let n = if done < held {
                stretch.min(held - done)
            } else {
                stretch.min(total - done)
            };
            let (l, r) = render(&mut instrument, &settings, n);
            for i in 0..n {
                if position == BLOCK {
                    let patch = Patch {
                        note: settings.frequency_st + settings.fine_st,
                        harmonics: settings.harmonics,
                        timbre: settings.timbre,
                        morph: settings.morph,
                        frequency_modulation_amount: settings.fm_amount,
                        timbre_modulation_amount: settings.timbre_amount,
                        morph_modulation_amount: settings.morph_amount,
                        engine: settings.model,
                        decay: settings.decay,
                        lpg_colour: settings.colour,
                    };
                    let modulations = Modulations {
                        note: 60.0,
                        trigger: if blocks * BLOCK < held { 1.0 } else { 0.0 },
                        trigger_patched: true,
                        ..Modulations::default()
                    };
                    engine.render(&patch, &modulations, &mut frames);
                    blocks += 1;
                    position = 0;
                }
                let frame = frames[position];
                position += 1;
                let at = done + i;
                let want = -f32::from(frame.out) / 32767.0 * settings.volume;
                if !gone && l[i] != want {
                    // The voice went idle, after its release (a frame of 0 reads as 0.0, so the
                    // first differing sample may be a few after the block it stopped at).
                    assert!(at > held, "model {model}, sample {at}");
                    gone = true;
                }
                if gone {
                    assert_eq!((l[i], r[i]), (0.0, 0.0), "model {model}, sample {at}");
                    let step = |x: i16| (i32::from(x) - 1).abs() <= 4;
                    assert!(step(frame.out), "model {model}, sample {at}: {frame:?}");
                } else {
                    assert_eq!(r[i], want, "model {model}, sample {at}");
                }
            }
            done += n;
        }
        // The chiptune engine with its envelope at noon drones, as on the module, until the guard.
        if model != 7 {
            assert!(
                gone && sounding(&instrument) == 0,
                "model {model} never went idle"
            );
        }
    }
}

/// **Silence in is exactly zero out once the voices are idle**, at 48 kHz and through the
/// resampler: every engine, four voices, released, then nothing but zeros, and nothing sounding.
#[test]
fn silence_in_is_exact_zero_once_the_voices_are_idle() {
    for rate in [48_000.0, 44_100.0] {
        for model in 0..ENGINES {
            let settings = Settings {
                model,
                decay: 0.3,
                ..Settings::default()
            };
            let mut instrument = Instrument::new();
            instrument.set_sample_rate(rate);
            for k in 0..4 {
                instrument.note_on(48 + 5 * k, 0.8, None);
            }
            render(&mut instrument, &settings, 4_800);
            assert_eq!(instrument.telemetry().voices_sounding, 4);
            instrument.release_all();
            let mut seconds = 0;
            while instrument.is_sounding() {
                render(&mut instrument, &settings, rate as usize);
                seconds += 1;
                assert!(seconds < 40, "model {model} at {rate}: still sounding");
            }
            assert_eq!(instrument.tail_samples(), 0);
            let (l, r) = render(&mut instrument, &settings, 9_600);
            assert!(
                l.iter().chain(&r).all(|&x| x == 0.0),
                "model {model} at {rate}"
            );
            assert_eq!(instrument.telemetry().peak, 0.0);
        }
    }
}

/// **A released note always ends**: the chiptune engine with its envelope at noon drones, as on
/// the module, so 30 s after its key went up the guard fades it over 10 ms and stops it; until
/// then it is the engine's frames exactly.
#[test]
fn a_drone_ends_thirty_seconds_after_its_release() {
    let settings = Settings {
        model: 7,
        ..Settings::default()
    };
    let mut instrument = Instrument::new();
    instrument.note_on(60, 1.0, None);
    render(&mut instrument, &settings, 4_800);
    instrument.note_off(60, None);
    let mut seconds = 0;
    while instrument.is_sounding() {
        let (l, _) = render(&mut instrument, &settings, 48_000);
        if seconds < 29 {
            assert!(
                l.iter().any(|&x| x.abs() > 0.01),
                "it drones at {seconds} s"
            );
        }
        seconds += 1;
        assert!(seconds <= 31, "still sounding");
    }
    assert_eq!(seconds, 31);
    let (l, r) = render(&mut instrument, &settings, 4_800);
    assert!(l.iter().chain(&r).all(|&x| x == 0.0));
}

/// Every slot of a set, alternating ±1.
fn every_slot(set: usize, sign: f32) -> [Slot; SLOTS] {
    let mut slots = [Slot::default(); SLOTS];
    for (i, slot) in slots.iter_mut().enumerate() {
        *slot = Slot {
            source: Some(Source::ALL[(i + 7 * set) % Source::ALL.len()]),
            target: Some(Target::ALL[(i + SLOTS * set) % Target::ALL.len()]),
            amount: if i % 2 == 0 { sign } else { -sign },
        };
    }
    slots
}

/// **No NaN or infinity**, on every engine, at extreme settings and with every slot at ±1: every
/// source against every target over the sets, cycling envelopes in each mode, the LFOs at their
/// fastest, every gesture at its end; then settings that are NaN and infinite, which are ignored.
#[test]
fn no_nan_or_infinity_anywhere() {
    for model in 0..ENGINES {
        for (n, extreme) in [0.0f32, 1.0].into_iter().enumerate() {
            let mut settings = Settings {
                model,
                frequency_st: if n == 0 { -48.0 } else { 48.0 },
                fine_st: 1.0,
                harmonics: extreme,
                timbre: extreme,
                morph: 1.0 - extreme,
                fm_amount: 1.0 - 2.0 * extreme,
                timbre_amount: 2.0 * extreme - 1.0,
                morph_amount: 1.0,
                decay: extreme,
                colour: 1.0 - extreme,
                aux: extreme,
                spread: 1.0 - extreme,
                mode: [Mode::Poly, Mode::Legato][n],
                glide_s: 0.05,
                bend_range_st: 48.0,
                volume: 1.0,
                ..Settings::default()
            };
            for (e, env) in settings.envelopes.iter_mut().enumerate() {
                env.attack_s = if n == 0 { 0.0005 } else { 30.0 };
                env.decay_s = 0.0005;
                env.release_s = if n == 0 { 30.0 } else { 0.0005 };
                env.response = 2.0 * extreme - 1.0;
                env.cycle = true;
                env.mode = [EnvelopeMode::Note, EnvelopeMode::Gate, EnvelopeMode::Follow][e];
            }
            for (l, lfo) in settings.lfos.iter_mut().enumerate() {
                lfo.rate_hz = 50.0;
                lfo.shape = [
                    LfoShape::Square,
                    LfoShape::SampleAndHold,
                    LfoShape::SmoothRandom,
                ][l];
                lfo.retrigger = l != 1;
                lfo.fade_s = 0.01;
            }
            for set in 0..3 {
                for sign in [1.0, -1.0] {
                    settings.slots = every_slot(set, sign);
                    let mut instrument = Instrument::new();
                    instrument.set_sample_rate(44_100.0);
                    instrument.set_bend(sign);
                    instrument.set_wheel(1.0);
                    instrument.set_pressure(1.0);
                    for k in [0u8, 40, 80, 127] {
                        instrument.note_on(k, 1.0 - extreme, Some(i32::from(k)));
                        instrument.set_tuning(k, Some(i32::from(k)), 24.0 * sign);
                    }
                    let (l, r) = render(&mut instrument, &settings, 4_410);
                    assert!(
                        l.iter().chain(&r).all(|x| x.is_finite()),
                        "model {model}, extreme {extreme}, set {set}, sign {sign}"
                    );
                    instrument.release_all();
                    let mut bad = settings;
                    bad.timbre = f32::NAN;
                    bad.volume = f32::INFINITY;
                    bad.envelopes[0].attack_s = f32::NAN;
                    bad.lfos[2].rate_hz = f32::NEG_INFINITY;
                    bad.slots[3].amount = f32::NAN;
                    instrument.set_bend(f32::NAN);
                    instrument.set_tuning(127, None, f32::INFINITY);
                    instrument.note_on(64, f32::NAN, None);
                    let (l, r) = render(&mut instrument, &bad, 2_205);
                    assert!(
                        l.iter().chain(&r).all(|x| x.is_finite()),
                        "model {model}, NaN settings"
                    );
                    assert!(instrument.telemetry().peak.is_finite());
                }
            }
        }
    }
}

/// **Poly**: a note takes an idle voice; with all eight held, a ninth takes the one sounding
/// longest; a released voice is taken before a held one.
#[test]
fn poly_takes_free_voices_then_the_longest_sounding() {
    let settings = Settings::default();
    let mut instrument = Instrument::new();
    for k in 0..VOICES as u8 {
        instrument.note_on(60 + k, 1.0, None);
        block(&mut instrument, &settings);
    }
    let notes: Vec<u8> = instrument.voices.voices.iter().map(|v| v.note).collect();
    assert_eq!(notes, (60..68).collect::<Vec<u8>>(), "each its own voice");
    assert_eq!(instrument.telemetry().voices_sounding, VOICES);
    // All held: the ninth steals the first.
    instrument.note_on(80, 1.0, None);
    instrument.apply();
    assert_eq!(instrument.voices.voices[0].note, 80);
    // A released one goes before a held one, though younger.
    instrument.note_off(65, None);
    instrument.note_on(81, 1.0, None);
    instrument.apply();
    assert_eq!(instrument.voices.voices[5].note, 81);
    // The stolen voice drops TRIG for a block, then raises it: a new edge.
    block(&mut instrument, &settings);
    assert_eq!(last(&instrument, 0).1.trigger, 0.0);
    block(&mut instrument, &settings);
    assert_eq!(last(&instrument, 0).1.trigger, 1.0);
}

/// An idle voice is not rendered: one note, one voice sounding; after its release and tail, none.
#[test]
fn idle_voices_are_not_rendered() {
    let settings = Settings {
        decay: 0.0,
        ..Settings::default()
    };
    let mut instrument = Instrument::new();
    render(&mut instrument, &settings, 480);
    assert_eq!(instrument.telemetry().voices_sounding, 0);
    assert!(!instrument.is_sounding());
    instrument.note_on(60, 1.0, None);
    render(&mut instrument, &settings, 4_800);
    assert_eq!(instrument.telemetry().voices_sounding, 1);
    assert_eq!(instrument.tail_samples(), u32::MAX, "held");
    instrument.note_off(60, None);
    assert_eq!(
        instrument.tail_samples(),
        u32::MAX,
        "the note off waits for the render"
    );
    render(&mut instrument, &settings, 12);
    let tail = instrument.tail_samples();
    assert!(tail > 0 && tail < 48_000, "{tail}");
    let mut taken = 0;
    while instrument.is_sounding() {
        render(&mut instrument, &settings, 480);
        taken += 480;
    }
    assert!(taken <= tail as usize + 480, "{taken} against {tail}");
}

/// **Mono**: one voice; every new key retriggers it (TRIG low for a block); a released key
/// returns to the one held before it without a trigger.
#[test]
fn mono_retriggers_and_returns_to_the_held_key() {
    let settings = Settings {
        mode: Mode::Mono,
        ..Settings::default()
    };
    let mut instrument = Instrument::new();
    block(&mut instrument, &settings);
    instrument.note_on(60, 1.0, None);
    block(&mut instrument, &settings);
    assert_eq!(last(&instrument, 0).1.trigger, 1.0);
    instrument.note_on(64, 1.0, None);
    block(&mut instrument, &settings);
    assert_eq!(last(&instrument, 0).1.trigger, 0.0, "retriggered");
    assert_eq!(last(&instrument, 0).1.note, 64.0);
    block(&mut instrument, &settings);
    assert_eq!(last(&instrument, 0).1.trigger, 1.0);
    assert_eq!(sounding(&instrument), 1);
    instrument.note_off(64, None);
    block(&mut instrument, &settings);
    let (_, m, _) = last(&instrument, 0);
    assert_eq!((m.note, m.trigger), (60.0, 1.0), "back to 60, no trigger");
    assert_eq!(instrument.voices.held(), vec![60]);
    instrument.note_off(60, None);
    block(&mut instrument, &settings);
    assert_eq!(last(&instrument, 0).1.trigger, 0.0, "released");
}

/// **Legato**: a key over a held one glides to it without a trigger; a key after every key is up
/// triggers and starts where it is played.
#[test]
fn legato_glides_without_a_trigger() {
    let settings = Settings {
        mode: Mode::Legato,
        glide_s: 0.1,
        ..Settings::default()
    };
    let mut instrument = Instrument::new();
    instrument.note_on(60, 1.0, None);
    for _ in 0..40 {
        block(&mut instrument, &settings);
    }
    instrument.note_on(72, 0.3, None);
    let mut pitches = Vec::new();
    for _ in 0..1_100 {
        block(&mut instrument, &settings);
        let (_, m, _) = last(&instrument, 0);
        assert_eq!(m.trigger, 1.0, "tied: no new trigger");
        pitches.push(m.note);
    }
    assert!(
        pitches[0] < 61.0,
        "it starts from the held key: {}",
        pitches[0]
    );
    assert!(pitches.windows(2).all(|w| w[1] >= w[0]));
    // Within 1 % of the interval at the glide time (400 blocks), there by the end.
    assert!((pitches[399] - 72.0).abs() < 0.13, "{}", pitches[399]);
    assert_eq!(*pitches.last().unwrap(), 72.0);
    // The tie keeps the first press's velocity.
    assert_eq!(published(&instrument, 0, Source::Velocity), 0.0);
    instrument.note_off(72, None);
    instrument.note_off(60, None);
    block(&mut instrument, &settings);
    instrument.note_on(48, 1.0, None);
    block(&mut instrument, &settings);
    let (_, m, _) = last(&instrument, 0);
    assert_eq!((m.note, m.trigger), (48.0, 1.0), "untied: no glide");
}

/// A note id picks its voice when both have one; with none, the key does. The per-id forms act
/// on the voice playing the id, or on nothing.
#[test]
fn note_ids_pick_their_voice() {
    let settings = Settings::default();
    let mut instrument = Instrument::new();
    instrument.note_on(60, 1.0, Some(1));
    instrument.note_on(60, 1.0, Some(2));
    instrument.note_on(62, 1.0, None);
    instrument.note_off(60, Some(2));
    instrument.apply();
    let gates: Vec<bool> = instrument.voices.voices[..3]
        .iter()
        .map(|v| v.gate)
        .collect();
    assert_eq!(gates, [true, false, true]);
    instrument.set_tuning(60, Some(1), 0.5);
    instrument.set_tuning_id(2, -0.25);
    instrument.set_tuning_id(99, 3.0);
    block(&mut instrument, &settings);
    assert_eq!(last(&instrument, 0).1.note, 60.5);
    assert_eq!(last(&instrument, 1).1.note, 59.75);
    assert_eq!(last(&instrument, 2).1.note, 62.0);
    instrument.note_off_id(99);
    instrument.apply();
    assert!(instrument.voices.voices[0].gate);
    instrument.note_off_id(1);
    instrument.apply();
    assert!(!instrument.voices.voices[0].gate);
    instrument.choke_id(2);
    instrument.apply();
    assert!(!instrument.voices.voices[1].active);
    instrument.note_off(62, Some(7));
    instrument.apply();
    assert!(
        !instrument.voices.voices[2].gate,
        "a voice with no id answers to its key"
    );
    instrument.choke(None);
    instrument.apply();
    assert_eq!(sounding(&instrument), 0);
    // A new note clears the tuning.
    instrument.note_on(60, 1.0, Some(1));
    block(&mut instrument, &settings);
    assert_eq!(last(&instrument, newest(&instrument)).1.note, 60.0);
}

/// **Each target's reach**, from one slot at full with the wheel at the top, read where the voice
/// hands it on: the module's knobs and jacks, the mix.
#[test]
fn each_voice_target_reaches_what_it_says() {
    let read = |target: Target, amount: f32| {
        let settings = Settings {
            harmonics: 0.25,
            timbre: 0.25,
            morph: 0.25,
            decay: 0.25,
            colour: 0.25,
            model: 8,
            slots: slots(&[(Source::Wheel, target, amount)]),
            ..Settings::default()
        };
        let mut instrument = Instrument::new();
        instrument.set_wheel(1.0);
        instrument.note_on(60, 1.0, None);
        block(&mut instrument, &settings);
        last(&instrument, 0)
    };
    assert_eq!(read(Target::Pitch, 1.0).1.note, 72.0);
    assert_eq!(read(Target::Pitch, -0.5).1.note, 54.0);
    assert_eq!(
        read(Target::Harmonics, 1.0).0.harmonics,
        1.25,
        "before Plaits clamps"
    );
    assert_eq!(read(Target::Timbre, -1.0).0.timbre, -0.75);
    assert_eq!(read(Target::Morph, 0.5).0.morph, 0.75);
    assert_eq!(read(Target::GateDecay, 0.5).0.decay, 0.75);
    assert_eq!(read(Target::Colour, 1.0).0.lpg_colour, 1.0);
    assert!((read(Target::Model, 1.0).1.engine * 23.0 - 24.0).abs() < 1e-5);
    let (_, m, _) = read(Target::Level, 0.5);
    assert!(m.level_patched);
    assert_eq!(m.level, 0.5);
    assert!(!read(Target::Pitch, 1.0).1.level_patched);
    assert_eq!(read(Target::Amplitude, -1.0).2.out_left, 0.0, "silence");
    assert_eq!(read(Target::Amplitude, 1.0).2.out_left, 2.0, "double");
    let pan = read(Target::Pan, 1.0).2;
    assert_eq!((pan.out_left, pan.out_right), (0.0, 1.0));
    let aux = read(Target::Aux, 1.0).2;
    assert_eq!((aux.out_left, aux.aux_left), (0.0, 1.0));
}

/// An LFO's Rate reaches ±4 octaves.
#[test]
fn a_rate_reaches_four_octaves() {
    let period = |amount: f32| {
        let mut settings = Settings {
            slots: slots(&[(Source::Wheel, Target::Lfo1Rate, amount)]),
            ..Settings::default()
        };
        settings.lfos[0] = crate::LfoSettings {
            rate_hz: 2.0,
            shape: LfoShape::SawUp,
            retrigger: true,
            fade_s: 0.0,
        };
        let mut instrument = Instrument::new();
        instrument.set_wheel(1.0);
        instrument.note_on(60, 1.0, None);
        let mut was = -2.0;
        let mut wraps = Vec::new();
        for b in 0..40_000 {
            block(&mut instrument, &settings);
            let v = published(&instrument, 0, Source::Lfo1);
            if v < was {
                wraps.push(b);
            }
            was = v;
            if wraps.len() == 3 {
                break;
            }
        }
        (wraps[2] - wraps[1]) as f32 * BLOCK as f32 / 48_000.0
    };
    assert!((period(0.0) - 0.5).abs() < 0.001);
    assert!((period(1.0) - 0.5 / 16.0).abs() < 0.001);
    assert!((period(-0.5) - 0.5 * 4.0).abs() < 0.001);
}

/// **A delayed envelope** is one in Gate mode whose Trigger is another's End of rise, as on
/// MATHS: envelope 2 starts when envelope 1's attack ends, and End of cycle is its complement.
#[test]
fn a_delayed_envelope_is_gate_mode_on_end_of_rise() {
    let mut settings = Settings {
        slots: slots(&[(Source::Envelope1Rise, Target::Envelope2Trigger, 1.0)]),
        ..Settings::default()
    };
    settings.envelopes[0].attack_s = 0.02;
    settings.envelopes[1].mode = EnvelopeMode::Gate;
    let mut instrument = Instrument::new();
    instrument.note_on(60, 1.0, None);
    let mut started = None;
    let mut risen = None;
    for b in 0..400 {
        block(&mut instrument, &settings);
        let rise = published(&instrument, 0, Source::Envelope1Rise);
        assert_eq!(
            rise + published(&instrument, 0, Source::Envelope1Cycle),
            1.0
        );
        if risen.is_none() && rise == 1.0 {
            risen = Some(b);
        }
        if started.is_none() && published(&instrument, 0, Source::Envelope2) > 0.0 {
            started = Some(b);
        }
    }
    let (risen, started) = (risen.unwrap(), started.unwrap());
    assert!((79..=80).contains(&risen), "20 ms is 80 blocks: {risen}");
    assert_eq!(
        started, risen,
        "the same block: envelope 1 is computed before 2"
    );
}

/// An envelope's Trigger read from itself is one block late, never a hang: an envelope
/// triggering itself from its End of cycle is a deterministic loop.
#[test]
fn feedback_is_one_block_late_and_deterministic() {
    let mut settings = Settings {
        slots: slots(&[
            (Source::Envelope1Cycle, Target::Envelope1Trigger, 1.0),
            (Source::Envelope1, Target::Envelope1Fall, 0.01),
        ]),
        ..Settings::default()
    };
    settings.envelopes[0] = crate::EnvelopeSettings {
        attack_s: 0.01,
        decay_s: 0.02,
        sustain: 0.0,
        release_s: 0.02,
        response: 0.3,
        cycle: false,
        mode: EnvelopeMode::Gate,
    };
    let run = || {
        let mut instrument = Instrument::new();
        instrument.note_on(60, 1.0, None);
        (0..2_000)
            .map(|_| {
                block(&mut instrument, &settings);
                published(&instrument, 0, Source::Envelope1)
            })
            .collect::<Vec<f32>>()
    };
    let a = run();
    assert_eq!(a, run());
    assert!(a.iter().any(|&v| v > 0.99) && a.iter().all(|v| v.is_finite()));
}

/// **LFO Retrigger on**: each voice's own LFO starts at phase 0 with its note. **Off**: one LFO
/// for every voice. **Fade**: the depth rises from 0 after each note.
#[test]
fn the_lfos_retrigger_share_and_fade() {
    let mut settings = Settings::default();
    settings.lfos[0] = crate::LfoSettings {
        rate_hz: 3.0,
        shape: LfoShape::SawUp,
        retrigger: true,
        fade_s: 0.0,
    };
    settings.lfos[1] = crate::LfoSettings {
        rate_hz: 3.0,
        shape: LfoShape::Sine,
        retrigger: false,
        fade_s: 0.0,
    };
    settings.lfos[2] = crate::LfoSettings {
        rate_hz: 1.0,
        shape: LfoShape::Square,
        retrigger: true,
        fade_s: 0.1,
    };
    let mut instrument = Instrument::new();
    instrument.note_on(60, 1.0, None);
    block(&mut instrument, &settings);
    assert_eq!(published(&instrument, 0, Source::Lfo1), -1.0, "phase 0");
    assert_eq!(published(&instrument, 0, Source::Lfo3), 0.0, "faded out");
    for _ in 0..77 {
        block(&mut instrument, &settings);
    }
    instrument.note_on(64, 1.0, None);
    block(&mut instrument, &settings);
    assert_eq!(
        published(&instrument, 1, Source::Lfo1),
        -1.0,
        "its own, from phase 0"
    );
    assert!(published(&instrument, 0, Source::Lfo1) > -0.9);
    assert_eq!(
        published(&instrument, 0, Source::Lfo2),
        published(&instrument, 1, Source::Lfo2),
        "shared"
    );
    assert!(published(&instrument, 0, Source::Lfo2).abs() > 0.1);
    let fade0 = published(&instrument, 0, Source::Lfo3);
    assert!((fade0 - 78.0 / 400.0).abs() < 1e-3, "{fade0}");
    for _ in 0..410 {
        block(&mut instrument, &settings);
    }
    assert_eq!(published(&instrument, 1, Source::Lfo3), 1.0, "faded in");
}

/// **The host's rate is the 48 kHz stream resampled**, with the reported latency; at 48 kHz none.
#[test]
fn the_host_rate_is_the_48_khz_stream_resampled() {
    let settings = Settings {
        model: 13,
        ..Settings::default()
    };
    let mut plain = Instrument::new();
    assert_eq!(plain.latency_samples(), 0);
    plain.set_sample_rate(48_000.0);
    assert_eq!(plain.latency_samples(), 0);
    let mut resampled = Instrument::new();
    for (rate, latency) in [(44_100.0, 78), (96_000.0, 88), (192_000.0, 176)] {
        resampled.set_sample_rate(rate);
        assert_eq!(resampled.latency_samples(), latency, "{rate}");
    }
    resampled.set_sample_rate(96_000.0);
    plain.note_on(57, 1.0, None);
    resampled.note_on(57, 1.0, None);
    let (input, _) = render(&mut plain, &settings, 4_800);
    let (output, _) = render(&mut resampled, &settings, 9_600);
    let mut r = crate::resampler::Resampler::new(96_000);
    let mut n = 0;
    for &y in &output {
        while r.needs_input() {
            r.push(input[n], input[n]);
            n += 1;
        }
        assert_eq!(r.output().0, y);
    }
}

/// **Reset** leaves no tail and every state as made: the same notes after it render what a new
/// instrument renders, on engines that keep memory between blocks (the six-op, the modal
/// resonator, the granular cloud).
#[test]
fn reset_clears_every_state() {
    for model in [2, 16, 20] {
        let settings = Settings {
            model,
            slots: slots(&[(Source::Lfo1, Target::Timbre, 0.3)]),
            ..Settings::default()
        };
        let play = |instrument: &mut Instrument| {
            instrument.note_on(50, 0.7, None);
            instrument.note_on(57, 0.9, None);
            render(instrument, &settings, 9_000)
        };
        let fresh = play(&mut Instrument::new());
        let mut used = Instrument::new();
        used.set_sample_rate(44_100.0);
        used.note_on(70, 1.0, None);
        render(
            &mut used,
            &Settings {
                model: 5,
                ..settings
            },
            3_000,
        );
        used.reset();
        used.set_sample_rate(48_000.0);
        assert!(!used.is_sounding());
        let (l, _) = render(&mut used, &settings, 480);
        assert!(l.iter().all(|&x| x == 0.0), "no tail");
        used.reset();
        assert_eq!(play(&mut used), fresh, "model {model}");
    }
}

/// Velocity and Random are drawn per note; Key is the glided note.
#[test]
fn key_is_the_glided_note() {
    let settings = Settings {
        glide_s: 0.05,
        ..Settings::default()
    };
    let mut instrument = Instrument::new();
    instrument.note_on(60, 1.0, None);
    block(&mut instrument, &settings);
    assert_eq!(published(&instrument, 0, Source::Key), 0.0);
    instrument.note_on(72, 0.5, None);
    block(&mut instrument, &settings);
    let key = published(&instrument, 1, Source::Key);
    assert!(key < 0.01, "it glides from 60: {key}");
    for _ in 0..800 {
        block(&mut instrument, &settings);
    }
    assert_eq!(published(&instrument, 1, Source::Key), 0.2);
    assert_eq!(published(&instrument, 1, Source::Velocity), -0.5);
    assert_eq!(published(&instrument, 0, Source::Velocity), 0.0);
}

/// The peak is the loudest host sample of the last render.
#[test]
fn the_peak_is_the_loudest_sample() {
    let settings = Settings::default();
    let mut instrument = Instrument::new();
    instrument.note_on(60, 1.0, None);
    let (l, r) = render(&mut instrument, &settings, 4_800);
    let peak = l.iter().chain(&r).fold(0.0f32, |p, x| p.max(x.abs()));
    assert!(peak > 0.0);
    assert_eq!(instrument.telemetry().peak, peak);
}
