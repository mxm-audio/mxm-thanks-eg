# NOTES.md — crates/mxm-thanks-eg-dsp

The reasons behind [`AGENTS.md`](AGENTS.md) and what was measured (T3, 2026-11-04).

## The cost (the plan's §5.1)

`examples/cpu.rs`, release build, eight held notes of one engine with an envelope and an LFO
routed, 256-sample renders, on the owner's Windows machine: the heaviest is **engine 21 (the bass
drum), 18.2 % of a core at 48 kHz and 18.7 % at 44.1 kHz** with the rate converter; then the modal
resonator (15.6 %) and the snare (15.4 %). The pitched engines take 1 to 6.4 % (the additive
engine is the heaviest of them). Under the plan's quarter of a core, so eight voices stand.

## Idling: one silent block was not enough

The contract's first rule, idle after one block of Plaits' silence (+1), failed three ways on the
engines as ported:

- A decaying tone a step or two above silence has whole blocks of +1 at its zero crossings, so a
  voice stopped mid-tail and the engine kept sounding after it.
- The six-op engines settle into a one-step residue (frames 0 to 2, now and then 3) that never
  reaches +1, so they never stopped.
- The particle engine is sparse: silent for 128 ms between grains while its gate is still open. Its
  AUX is its raw grains, scaled by `1/density` (some 2·10⁵ at Timbre 0).

So a voice through the low-pass gate stops only once the gate's gain is under 10⁻⁶ (the engine
voice's `low_pass_gate_gain`, which `init` leaves at 1 for an engine that bypasses the gate); one
that bypasses it after 128 ms within a step of silence; and only the outputs the mix uses are
read. What idling cuts is under four steps (−78 dBFS before Volume), which the link test checks.

## The guard

The chiptune engine with TRIG patched bypasses the gate and takes its envelope from the Timbre
attenuverter: at noon there is none, so a released note drones, as the module does with nothing
after it. No slot can close it (Level is the bypassed gate; Amplitude is a trim, `1 + Σ`), so the
instrument ends any released note still sounding 30 s after its key went up, with a 10 ms fade. The
owner may prefer a different answer: a VCA of the instrument's own, or the attenuverter's noon
meaning a decay.

## Choices the contract left open, for the owner to hear

- **The curve**: `(e^{6r·x} − 1)/(e^{6r} − 1)` on the ramp, so Response +1 is down 26 dB half way
  through a fall; the times do not change with it.
- **Rise and Fall** are linear in seconds, as the contract says: an amount of 1 % adds 0.3 s.
- **Smooth random** glides by a raised cosine to a new random point each cycle.
- **Glide** is exponential, the Glide time reaching within 1 % of the interval.
- **Pan** is a balance (centre unity in both channels), so a voice at rest is its frames exactly.
- **Spread** crossfades the Aux mix in both channels to OUT left and AUX right; at 1, Aux does
  nothing.
- **Poly's glide** comes from the last note played; a stolen voice keeps its state, as a retrigger
  on the module.
- **Notes wait for the next render**: `note_on` before the first `render` (or beside a change of
  mode) used the last call's settings until they were queued.
