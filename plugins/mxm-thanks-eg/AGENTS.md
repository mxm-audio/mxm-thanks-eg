# AGENTS.md — plugins/mxm-thanks-eg

Parent: [`../../AGENTS.md`](../../AGENTS.md)

# Purpose

The nice-plug shell, CLAP and VST3. **In T2 it is the bare plugin** of ops'
`plans/plan-mxm-thanks-eg.md` §7: one Plaits voice at 48 kHz, the module's controls as parameters,
no editor, so the owner can hear the engines as they are ported. T3 brings the eight voices, the
envelopes, the LFOs, the route slots and the resampler; T4 the editor, presets and control map.

# Ownership

`src/lib.rs` (the identity, the `Plugin` impl, MIDI into the module's V/OCT and TRIG, the frames
into floats), `src/params.rs` (the bare parameters), `Cargo.toml`.

# Local Contracts

- **The engines run at 48 kHz.** At 48 kHz a host sample is an engine frame, exactly; at any other
  rate the frames are read by linear interpolation, a stopgap until T3's resampler, so every rate
  plays in tune and `clap-validator`'s rates pass, but only 48 kHz is bit for bit.
- **MIDI is the module's jacks**: the newest held key is V/OCT plus Frequency, Fine and the bend
  (±2 semitones); TRIG is high while a key is held, and drops for one block when a key is pressed
  over another, so the voice sees a new edge. Releasing the newest key returns to the one held
  before it without an edge. Level is not patched, so the gate pings, as the module with only V/OCT
  and TRIG patched.
- **A frame is read back as the module's output stage does**: Plaits scales by −32767, so a sample
  is `−frame / 32767`. Aux fades the sum from OUT to AUX; Volume scales it.
- **Nothing allocates in `process` or `reset`**: the voice is made once, in `default`.
- **Flush-to-zero** through nice-plug's `unsafe_flush_denormals`, as the engines' reference was
  rendered (`crates/mxm-thanks-eg-engines/AGENTS.md`). A recorded deviation from the kit's *flush
  denormals in the DSP itself*: a flush in the ported code would change its bits.
- **The parameter ids are provisional** until the gate (T5); the engines' names are the
  instrument's own (`params::ENGINE_NAMES`), never the module's.

# Work Guidance

- `cargo xtask bundle mxm-thanks-eg --release` makes the `.clap` and `.vst3` and installs them,
  as every MXM plugin's bundle does.

# Verification

```bash
cargo test -p mxm-thanks-eg                  # identity: the VST3 class id, categories, bundle name
cargo xtask bundle mxm-thanks-eg --release
clap-validator validate target/bundled/mxm-thanks-eg.clap
```

# Child DOX Index

None.
