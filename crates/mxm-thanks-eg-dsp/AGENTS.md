# AGENTS.md — crates/mxm-thanks-eg-dsp

Parent: [`../../AGENTS.md`](../../AGENTS.md)

# Purpose

The instrument around the ported engines: eight voices, each a Plaits voice
(`mxm_macro_osc::Voice`) with three envelopes and three LFOs; the sixteen route slots on
the collection's routing (`mxm-modulation`); the voices' mix; and the resampler from 48 kHz to the
host's rate. Framework-free, no file IO; nothing allocated after `Instrument::new` and
`set_sample_rate`. The design is ops' `plans/plan-mxm-thanks-eg.md` §3 and §5; the MATHS behaviour
the envelopes take is `research:modulation/make-noise-maths.md`. The reasons and the measurements
are in [NOTES.md](NOTES.md).

# Ownership

`src/lib.rs` holds **the interface with the plugin** (`Settings`, `Slot`, `Source`, `Target`, the
modes and shapes, `reach`, `Target::offer`, `Instrument`'s methods): a change to it is a change to
`plugins/mxm-thanks-eg` in the same commit. Behind it: `voice.rs` (one voice: the jacks, the order,
the mix, idling), `voices.rs` (allocation, the held keys, note ids), `envelope.rs`, `lfo.rs`,
`routing.rs` (indices, laws, reaches, the grid), `resampler.rs`, `settings.rs` (the kept settings).
Tests: `src/tests.rs`, `src/conformance.rs`, `tests/realtime.rs`; `examples/cpu.rs` the cost.

# Local Contracts

## The voices

- **Eight in Poly**: a new note takes an idle voice, else the released one sounding longest, else
  the held one sounding longest. **Mono** and **Legato** use voice 0, last-note priority; Mono
  retriggers on every new key; Legato glides to a key played over a held one without a new trigger;
  a released key returns to the one held before it, without one. A change of mode releases every
  voice. Glide (time to within 1 %, exponential): in Poly from the last note played, in Mono from
  where the voice is, in Legato on a tie only.
- **Note events wait for the next `render`** (a fixed queue of 256), which takes in its settings
  first: an event acts at the next engine block either way, and so under the settings that block
  plays (mode, Retrigger, Fade, Glide). An event naming a note id acts on the voice playing it; one
  without, or on a voice without one, on its key. A non-finite velocity is 1; a non-finite tuning is
  dropped; a new note clears the tuning.
- **A voice is the module with the case around it.** Per engine block of 12 frames at 48 kHz: the
  routing is evaluated (below), the result filled into the engine voice's `Patch` and
  `Modulations`, and `Voice::render` makes 12 frames. The note is TRIG (`trigger_patched`, high
  while the voice's gate is; a retrigger drops it for one block when it was high). The knobs are
  `Patch::note` (`frequency + fine`), V/OCT `Modulations::note` (`key + glide + bend × range +
  tuning + Pitch`), which the module averages over two blocks.
- **The module's own modulation stays the module's**: the three attenuverters are
  `Patch::{frequency,timbre,morph}_modulation_amount`, the CV jacks stay unpatched, so the decay
  envelope reaches Pitch, Timbre and Morph as on the module. A slot's Harmonics, Timbre and Morph
  add to the patch's values before Plaits clamps them; Gate decay and Colour, which Plaits does not
  clamp, are clamped to 0…1; Model adds `Σ/23` to the model CV, which the module samples at each
  trigger and quantises. **A slot aimed at Level sets `level_patched`**, and the gate follows the
  sum (`ProcessLP`).
- **A frame becomes a sample as the module's output stage does**: `−frame / 32767`. Aux fades OUT
  to AUX; Spread crossfades that mix in both channels to OUT left and AUX right; Amplitude scales
  (`×0…×2`); Pan balances (centre is unity in both channels); summed over the voices; Volume. Aux,
  Spread and Volume are smoothed (5 ms); every gain ramps across its block.
- **An idle voice is not rendered** and adds exact zero. Idle when its key is up and its trigger
  through the engine's delay (8 blocks), the envelopes a slot reads at rest, and the outputs the
  mix uses silent, within one step of Plaits' +1: for a block once the low-pass gate's gain is
  below 10⁻⁶, or, an engine that bypasses the gate (its gain stays 1), for 128 ms. **A released
  note always ends**: one still sounding 30 s after its key went up fades over 10 ms and stops (the
  chiptune engine with its envelope at noon drones). The next note starts an idle voice as
  `Voice::init` would; `reset` clears every voice as `Voice::new` made it.
- Velocity (`v − 1`) and Random (`2u − 1`) are drawn per untied note; Key is the glided note on a
  60-semitone unit, middle C 0; Wheel and Pressure 0 to 1, Bend the lever −1 to 1
  (`mxm_modulation::standard`).

## The envelopes, MATHS' functions in an ADSR

As the plan's §5.4: Attack, Decay, Sustain, Release, one Response curve (−1 log to +1 exp, MATHS'
Vari-Response), Cycle, and Mode (Note, Gate, Follow). Once per block.

- **One slope core**: a ramp 0…1 at the segment's rate, output `curve(ramp)`; the curve is
  `(e^{6r·x} − 1)/(e^{6r} − 1)` (its mirror for log), defined against the level as the manual
  defines it, so a retrigger continues from the level it is at. A time is the whole 0-to-1 travel;
  the curve does not change it.
- **Times**: each from 0.5 ms to 30 s on its parameter; the Time target reaches ×/÷ 16 (4 octaves,
  exponential, positive shortens, as MATHS' Both); Rise and Fall reach their whole range, linear
  in seconds (an amount of 1 adds 30 s), positive slows, as MATHS' Rise and Fall CV; then clamped
  to the range, and Time after.
- **Cycling**, attack and decay repeat and Sustain is unused. In Note mode it starts at note on and
  releases at note off from where it is; in Gate mode it cycles while Cycle is on or the Cycle
  target is high, with or without a trigger; in Follow mode Cycle does nothing.
- **Retrigger, as MATHS**: a rising Trigger during the decay, sustain or release restarts the
  attack from where it is; one during the attack is ignored. In Note mode an attack the Trigger
  started completes whatever the note does (MATHS' rise ignores a gate's length). A new note on a
  sounding voice restarts the attack from where it is (Legato's tied notes don't). In Gate mode the
  Trigger is the gate: falling, even in the attack, releases.
- **Follow**: the output moves toward its Input (clamped 0…1) at the attack rate rising and the
  release rate falling (lag, portamento, attack–sustain–release).
- **The gates**: End of rise high from the end of the attack until rest or the next attack (a
  followed level reached counts as an end of rise); End of cycle its complement. A gate target is
  high while the slots aimed at it sum above one half.
- Output 0 to 1 (MATHS' 10 V); cycling, it turns at 0.8 (the manual's 8 V).

## The LFOs

Rate from 0.01 Hz to 50 Hz, or resolved from the tempo by the plugin (kept to 800 Hz); the Rate
target reaches ±4 octaves (the standard's). Shapes: sine, triangle and square from phase 0 as the
sine, saw up from −1, saw down from +1, sample-and-hold, and smooth random (a raised-cosine glide
to a new random point each cycle). **Retrigger on**: each voice's own LFO restarts at phase 0 with
each untied note; **off**: one LFO for all voices, free-running, whose Rate target reads only the
shared sources (Wheel, Pressure, Bend, the shared LFOs). Fade: the depth rises linearly from 0 over
the fade time after each note. Output −1 to 1.

## The routing

- **The slots fill `mxm-modulation`'s grid** (`SourceFrame`, `Compacted`), rebuilt when a slot
  changes, never per block. A slot is in use when its source and target are both set; two on one
  pair add; an amount is kept to ±1. **A pair the standard refuses is not filled**
  (`Target::offer`: a gesture into a Cycle or Trigger gate, Key or Random into Level), and a
  one-sided pair keeps its live half (Velocity into Level, positive only).
- **Each block, in order**: the performance sources, the LFOs, envelopes 1, 2, 3, then the voice's
  targets, the render, and the Decay source. A source read before it is computed this block reads
  last block's value, so any loop (an envelope triggering itself, a ring) is deterministic and one
  block, 0.25 ms, late. Every source is published every block, so nothing owes a `clear`.
- **Reaches** (amount ±1, [`reach`]): Pitch ±12 st; Harmonics, Timbre, Morph, Gate decay, Colour,
  Aux, Pan, Level the whole range; Model ±24 engines; Amplitude `×0…×2` (the standard's law); a
  Rate ±4 octaves; envelope times as above; gates and Input their value. Key routes take the
  standard's per-octave reach.

## The resampler

A fixed polyphase Kaiser-windowed sinc from 48 kHz to the host's rate, designed for 110 dB:
passband to 20 kHz, stopband from the lower Nyquist at least 100 dB down, history cleared on
`reset`; host rates 44.1 to 192 kHz (any rate works). Exact phases when the rate's ratio to 48 kHz
reduces to at most 1 024, else 512 interpolated. **Its latency is fixed**, `⌈half · rate / 48 000⌉`
host samples: 78 at 44.1 kHz, 88 at 96, 176 at 192. **At 48 kHz bypassed, with no latency**, so
each host sample is a sum of the voices' frames exactly. It is this crate's until a second
instrument needs it (MXM Step Sampler's plan wants one too); then it moves into mxm-kit, which
needs the owner's word to push.

## Realtime

No allocation, lock or panic in `render`, the note calls or `reset`, on any input; a NaN in a
setting is ignored (the last good value stays), never propagated. `#![forbid(unsafe_code)]` in the
library; `tests/realtime.rs` counts allocations with its own allocator.

# Work Guidance

- The engines are held bit for bit (mxm-macro-osc); this crate is not, and is tested as the
  collection's instruments are. Its one link to the engines' reference is
  `one_voice_at_48_khz_is_the_engine_voice_exactly`.
- The engine voice's accessors for this crate (`init`, `clear`, `decay_envelope`,
  `low_pass_gate_gain`) are listed in mxm-macro-osc's `AGENTS.md`; add one there only when
  nothing here can know it.

# Verification

```bash
cargo test -p mxm-thanks-eg-dsp
cargo clippy -p mxm-thanks-eg-dsp --all-targets -- -D warnings
cargo run --release -p mxm-thanks-eg-dsp --example cpu    # eight voices of each engine
```

The tests owed, all there: at 48 kHz one voice's output equals the engine voice's frames read as
samples (every engine); silence in is exactly zero out once the voices are idle (every engine, at
48 and 44.1 kHz); no NaN or infinity across every engine, extreme settings and every slot at ±1;
each envelope mode, the retrigger rule, Cycle and the gates; the LFO shapes, Retrigger and Fade;
voice allocation and stealing, Mono and Legato, note ids; each target's reach; the resampler's
passband, stopband and latency at 44.1, 96 and 192 kHz; the standard's three conformance checks,
each falsified once; no allocation; the CPU measure (the plan's §5.1: under a quarter of a core;
the figure is in NOTES.md).

# Child DOX Index

None.
