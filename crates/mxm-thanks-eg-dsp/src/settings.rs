//! The settings as the instrument keeps them: each value finite and in its range. A value that
//! is not finite is ignored, and the last good one stays (the crate's `AGENTS.md`, *Realtime*).

use crate::envelope::{MAX_TIME, MIN_TIME};
use crate::lfo::MAX_RATE;
use crate::{EnvelopeSettings, LfoSettings, Settings, Slot};

/// The engines, in Plaits' registration order.
pub(crate) const ENGINES: i32 = 24;

#[inline]
fn take(kept: &mut f32, new: f32, low: f32, high: f32) {
    if new.is_finite() {
        *kept = new.clamp(low, high);
    }
}

/// `new` into `kept`, value by value.
pub(crate) fn absorb(kept: &mut Settings, new: &Settings) {
    kept.model = new.model.clamp(0, ENGINES - 1);
    take(&mut kept.frequency_st, new.frequency_st, -120.0, 120.0);
    take(&mut kept.fine_st, new.fine_st, -120.0, 120.0);
    take(&mut kept.harmonics, new.harmonics, 0.0, 1.0);
    take(&mut kept.timbre, new.timbre, 0.0, 1.0);
    take(&mut kept.morph, new.morph, 0.0, 1.0);
    take(&mut kept.fm_amount, new.fm_amount, -1.0, 1.0);
    take(&mut kept.timbre_amount, new.timbre_amount, -1.0, 1.0);
    take(&mut kept.morph_amount, new.morph_amount, -1.0, 1.0);
    take(&mut kept.decay, new.decay, 0.0, 1.0);
    take(&mut kept.colour, new.colour, 0.0, 1.0);
    take(&mut kept.aux, new.aux, 0.0, 1.0);
    take(&mut kept.spread, new.spread, 0.0, 1.0);
    kept.mode = new.mode;
    take(&mut kept.glide_s, new.glide_s, 0.0, MAX_TIME);
    take(&mut kept.bend_range_st, new.bend_range_st, 0.0, 96.0);
    take(&mut kept.volume, new.volume, 0.0, 16.0);
    for (k, n) in kept.envelopes.iter_mut().zip(&new.envelopes) {
        envelope(k, n);
    }
    for (k, n) in kept.lfos.iter_mut().zip(&new.lfos) {
        lfo(k, n);
    }
    for (k, n) in kept.slots.iter_mut().zip(&new.slots) {
        slot(k, n);
    }
}

fn envelope(kept: &mut EnvelopeSettings, new: &EnvelopeSettings) {
    take(&mut kept.attack_s, new.attack_s, MIN_TIME, MAX_TIME);
    take(&mut kept.decay_s, new.decay_s, MIN_TIME, MAX_TIME);
    take(&mut kept.sustain, new.sustain, 0.0, 1.0);
    take(&mut kept.release_s, new.release_s, MIN_TIME, MAX_TIME);
    take(&mut kept.response, new.response, -1.0, 1.0);
    kept.cycle = new.cycle;
    kept.mode = new.mode;
}

fn lfo(kept: &mut LfoSettings, new: &LfoSettings) {
    take(&mut kept.rate_hz, new.rate_hz, 0.0, MAX_RATE);
    take(&mut kept.fade_s, new.fade_s, 0.0, 60.0);
    kept.shape = new.shape;
    kept.retrigger = new.retrigger;
}

fn slot(kept: &mut Slot, new: &Slot) {
    kept.source = new.source;
    kept.target = new.target;
    take(&mut kept.amount, new.amount, -1.0, 1.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_non_finite_value_keeps_the_last_good_one() {
        let mut kept = Settings::default();
        let mut new = Settings {
            timbre: 0.25,
            ..Settings::default()
        };
        new.envelopes[1].attack_s = 2.0;
        absorb(&mut kept, &new);
        assert_eq!(kept.timbre, 0.25);
        new.timbre = f32::NAN;
        new.envelopes[1].attack_s = f32::INFINITY;
        new.slots[4].amount = f32::NAN;
        new.volume = -3.0;
        new.model = 99;
        absorb(&mut kept, &new);
        assert_eq!(kept.timbre, 0.25);
        assert_eq!(kept.envelopes[1].attack_s, 2.0);
        assert_eq!(kept.slots[4].amount, 0.0);
        assert_eq!(kept.volume, 0.0);
        assert_eq!(kept.model, 23);
    }

    #[test]
    fn the_defaults_are_kept_as_they_are() {
        let mut kept = Settings::default();
        absorb(&mut kept, &Settings::default());
        assert_eq!(kept, Settings::default());
    }
}
