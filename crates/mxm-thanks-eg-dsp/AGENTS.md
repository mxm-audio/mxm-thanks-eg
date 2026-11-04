# AGENTS.md — crates/mxm-thanks-eg-dsp

Parent: [`../../AGENTS.md`](../../AGENTS.md)

# Purpose

The instrument around the ported engines: eight voices, each a Plaits voice
(`mxm_thanks_eg_engines::Voice`) with three envelopes and three LFOs; the sixteen route slots on
the collection's routing (`mxm-modulation`); the voices' mix; and the resampler from 48 kHz to the
host's rate. Framework-free, no file IO; nothing allocated after `Instrument::new` and
`set_sample_rate`. The design is ops' `plans/plan-mxm-thanks-eg.md` §3 and §5; the MATHS behaviour
the envelopes take is `research:modulation/make-noise-maths.md`.

# Ownership

`src/lib.rs` holds **the interface with the plugin** (`Settings`, `Slot`, `Source`, `Target`, the
modes and shapes, `Instrument`'s methods): a change to it is a change to `plugins/mxm-thanks-eg` in
the same commit. Behind it, a module per part: the voices and their allocation, the envelopes, the
LFOs, the routing, the resampler.

# Local Contracts

## The voices

- **Eight in Poly**: a new note takes a free voice, else the one sounding longest. **Mono** and
  **Legato** use one voice, last-note priority; Legato glides to a note played over a held one
  without a new trigger; a released key returns to the one held before it.
- **A voice is the module with the case around it.** Per engine block of 12 frames at 48 kHz: the
  routing is evaluated (below), the result filled into the engine voice's `Patch` and
  `Modulations`, and `Voice::render` makes 12 frames. The note is TRIG (`trigger_patched`, high
  while the voice's gate is; a retrigger drops it for one block), and V/OCT is
  `key + frequency + fine + bend × range + glide + tuning + Pitch`.
- **The module's own modulation stays the module's**: the three attenuverters are
  `Patch::{frequency,timbre,morph}_modulation_amount`, the CV jacks stay unpatched, so the decay
  envelope reaches Pitch, Timbre and Morph as on the module. A slot's Harmonics, Timbre, Morph,
  Gate decay and Colour add to the patch's values before Plaits clamps them; Model adds to the
  model, rounded to an engine. **A slot aimed at Level sets `level_patched`**, and the gate follows
  the sum (`ProcessLP`).
- **A frame becomes a sample as the module's output stage does**: `−frame / 32767`; OUT and AUX are
  mixed by Aux, placed by Spread (0: both in both channels; 1: OUT left, AUX right), scaled by
  Amplitude (`×0…×2`), panned, summed over the voices and scaled by Volume.
- **An idle voice is not rendered** and adds exact zero: idle when its gate is low, its envelopes
  are at rest and its frames have been Plaits' silence (+1) for a block; the next note starts it
  as `Voice::init` would. So silence in is exactly zero out, and an idle voice costs nothing.
- Velocity and Random (`2u − 1`) are drawn per note; Key is the glided note (12 st per octave from
  C4 as 0, the standard's), Wheel and Pressure 0 to 1, Bend the lever −1 to 1
  (`mxm_modulation::standard`).

## The envelopes, MATHS' functions in an ADSR

As the plan's §5.4: Attack, Decay, Sustain, Release, one Response curve (−1 log to +1 exp, MATHS'
Vari-Response), Cycle, and Mode (Note, Gate, Follow).

- **Times**: each from 0.5 ms to 30 s on its parameter; the Time target reaches ×/÷ 16 (4 octaves,
  exponential, positive shortens, as MATHS' Both); Rise and Fall reach their whole range, linear,
  positive slows, as MATHS' Rise and Fall CV.
- **Cycling**, attack and decay repeat and Sustain is unused. In Note mode it starts at note on and
  releases at note off from where it is; in Gate mode it cycles while Cycle is on or the Cycle
  target is high, with or without a trigger; in Follow mode Cycle does nothing.
- **Retrigger, as MATHS**: a rising Trigger during the decay or release restarts the attack from
  where it is; one during the attack is ignored. A new note on a sounding voice restarts the
  attack from where it is (Legato's tied notes don't).
- **Follow**: the output moves toward its Input at the attack rate rising and the release rate
  falling (lag, portamento, attack–sustain–release).
- **The gates**: End of rise high from the end of the attack until rest or the next attack; End of
  cycle its complement. A gate target is high while the slots aimed at it sum above one half.
- Output 0 to 1 (MATHS' 10 V); cycling, it reaches 0.8 (the manual's 8 V).

## The LFOs

Rate from 0.01 Hz to 50 Hz, or resolved from the tempo by the plugin; the Rate target reaches ±4
octaves (the standard's). Shapes: sine, triangle, saw up, saw down, square, sample-and-hold,
smooth random. **Retrigger on**: each voice's own LFO restarts at phase 0 with each note; **off**:
one LFO for all voices, free-running. Fade: the depth rises from 0 over the fade time after each
note. Output −1 to 1.

## The routing

- **The slots fill `mxm-modulation`'s grid** (`SourceFrame`, `Compacted`), rebuilt when a slot
  changes, never per sample. A slot is in use when its source and target are both set; two on one
  pair add.
- **Each block, in order**: the performance sources, the LFOs, envelopes 1, 2, 3, then the voice's
  targets. A source read before it is computed this block reads last block's value, so any loop
  (an envelope triggering itself, a ring) is deterministic and one block, 0.25 ms, late.
- **Reaches** (amount ±1): Pitch ±12 st; Harmonics, Timbre, Morph, Gate decay, Colour, Aux, Pan,
  Level the whole range; Model ±24 engines; Amplitude `×0…×2` (the standard's amplitude law); a
  Rate ±4 octaves; envelope times as above; gates and Input their value.

## The resampler

A fixed polyphase windowed-sinc from 48 kHz to the host's rate: passband to 20 kHz, stopband at
least 100 dB down, its latency a fixed number of host samples reported to the plugin, history
cleared on `reset`; host rates 44.1 to 192 kHz; **at 48 kHz bypassed, with no latency**, so each
host sample is a sum of the voices' frames exactly. It is this crate's until a second instrument
needs it (MXM Step Sampler's plan wants one too); then it moves into mxm-kit, which needs the
owner's word to push.

## Realtime

No allocation, lock or panic in `render`, the note calls or `reset`, on any input; a NaN in a
setting is ignored, never propagated. `#![forbid(unsafe_code)]`.

# Work Guidance

- The engines are held bit for bit (their crate); this crate is not, and is tested as the
  collection's instruments are.

# Verification

```bash
cargo test -p mxm-thanks-eg-dsp
cargo clippy -p mxm-thanks-eg-dsp --all-targets -- -D warnings
```

The tests owed: at 48 kHz one voice's output equals the engine voice's frames read as samples
(the link to the engines' reference); silence in is exactly zero out once the voices are idle;
no NaN or infinity across every engine, extreme settings and every slot at ±1; each envelope mode,
the retrigger rule, Cycle and the gates; the LFO shapes, Retrigger and Fade; voice allocation and
stealing, Mono and Legato; each target's reach; the resampler's passband, stopband and latency at
44.1, 96 and 192 kHz; a CPU measure of eight voices of the heaviest engine (the plan's §5.1: under a
quarter of a core).

# Child DOX Index

None.
