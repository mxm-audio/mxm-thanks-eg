# AGENTS.md — mxm-thanks-eg

DOX rail for this repository: what it is, the rules every part keeps, and the Child DOX Index.
The reasoning and the owner's rulings are in [NOTES.md](NOTES.md). The plan it is built from is
ops' `plans/plan-mxm-thanks-eg.md`, with its stages (T0 to T5) and their checks. The rules every
MXM plugin keeps are mxm-kit's `docs/plugin-conventions.md`; nothing here weakens them.

# Purpose

**MXM Thanks EG**: Plaits' twenty-four engines as a polyphonic MXM instrument, CLAP and VST3,
ported to Rust from Emilie Gillet's MIT code and named as a tribute to her (the owner,
2026-11-04). Eight voices, each a whole Plaits voice; three envelopes that are function
generators after Make Noise's MATHS too; three LFOs; sixteen route slots; 101 parameters.

Pre-alpha: T0 to T4 are done (2026-11-04): the port, all 24 engines and all 220 cases bit for bit;
the instrument; the editor, the factory sounds, the control map and the host tests. T5 is the
owner's: listening, then the freeze. Private until it is ready; after version 1.0 of the MXM
collection.

# Ownership

Root owns `AGENTS.md`, `NOTES.md`, `README.md`, `NOTICE.md`, `LICENSE`, `TRADEMARKS.md`,
`CONTRIBUTING.md`, `Cargo.toml`, `Cargo.lock`, `bundler.toml`, `.cargo/config.toml` (the `xtask`
alias), `.gitignore` and `.gitattributes`, and `xtask/` (the collection's shared `mxm-xtask`:
`bundle`), and `docs/briefs/mxm-thanks-eg.md` (the design system's §14 brief).
`crates/mxm-thanks-eg-engines` (the port), `crates/mxm-thanks-eg-dsp` (the instrument: voices,
envelopes, LFOs, the slots' routing, the resampler) and `plugins/mxm-thanks-eg` (the shell and
editor, with `host-tests/`, the slow tier) have their own `AGENTS.md`.

The reference harness, the C++ it builds and the research pages are in the private research
repository (`research:sources/mutable-instruments-plaits/`, its harness in `derived/harness/`;
`research:oscillators/`, `research:modulation/`), never here: this repository commits only the
harness's cases and its outputs' SHA-256s.

# Local Contracts

## The engines are a port, held to the C++

- **Rust only**: no C++ in the plugin and no FFI. Ported from `pichenettes/eurorack`'s `plaits/`
  and the parts of `stmlib` it uses, at the commits the research harness pins; each ported file
  keeps its original's copyright line, and `NOTICE.md` the MIT text.
- **Bit for bit against the C++ at 48 kHz**, Windows' bits, in blocks of 12 as the module, on **the
  16-bit frames the voice returns** (OUT and AUX, after Plaits' own output stage: ×−32767, +1,
  clipped). The one exception: a case whose engine calls a maths function where MSVC's library and
  Rust's differ passes at most one step of a frame apart, and names the call.
- **The one change from the C++ is the rate**: `kCorrectedSampleRate` takes 48 000 where the module
  takes its DAC's 47 872.34 Hz, which moves `a0` (every engine's pitch, 4.6 cents), the six-op
  engines' own pitch, envelopes and LFO, and the LPC speech's pitch to where the module meant them. The harness makes
  the same change; its README lists its two compiler workarounds, which change nothing.
- **Each voice has its own random stream**, starting where the C++'s one static `Random` starts
  (`0x21`), so a one-voice case matches.
- **Flush-to-zero is on**: the plugin takes nice-plug's `unsafe_flush_denormals`, and the harness
  sets the same bit, so the reference is what ships. The DSP adds no denormal guard of its own,
  which would change its bits.
- **The tables are Plaits' own values**, converted from `plaits/resources.cc` (and the speech
  words' file) by `crates/mxm-thanks-eg-engines/gen/tables.py`, bit for bit; never regenerated from
  the Python scripts that made them, whose numpy arithmetic a port could only approximate.
- **Plaits' data ships as Plaits has it**: the speech words, the six-op patches, the wavetables
  (the owner, 2026-11-04: *"The words and patches MUST be in"*). Their rights are the owner's,
  with legal; no change here waits on them.
- **The engines read no files**: the module's user data is not ported.

## The names

"Plaits", "Mutable Instruments", "MATHS" and "Make Noise" appear in `NOTICE.md`, the README, this
file, `NOTES.md` and the research, never in the product: not in a parameter, an engine's or a
control's name, a preset or the editor (Plaits' *Guidelines for derivative works*; ops'
`notes/synths.md`: maker and model names are research inputs, never product names).

## The instrument

- **The host holds what the panel can show at once** (the owner, 2026-11-04): the voice's 17
  parameters, the envelopes' 21, the LFOs' 15 and sixteen route slots
  `route<r>_{source,target,amount}`, 48; **101**. A source or target is a value of a slot, never a
  parameter of its own. The plan's §5 has the lists, the slots' rules and the envelopes' modes.
- **Every voice runs at 48 kHz**, and one resampler takes the sum to the host's rate with a fixed,
  reported latency; at 48 kHz nothing is resampled.
- The collection's contract, as every MXM instrument (the plan's §5.7).

# Work Guidance

- Build and test on Windows only, as far as `python ws.py reach` says a change goes; the other
  platforms in the owner's batch (the workspace's `AGENTS.md`).
- Kit work (the resampler, the slots if they move there) through `python ws.py link kit`, unlinked
  before any push; pushing mxm-kit's public `main` needs the owner's word.

# Verification

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test                 # the fast tier: the engines' cases bit for bit, the instrument, the plugin
cargo xtask bundle mxm-thanks-eg --release
cargo test -p mxm-thanks-eg-host-tests      # the slow tier: the release bundle in MXM Player
clap-validator validate target/bundled/mxm-thanks-eg.clap
```

`clap-validator` 0.4.1 on Windows fails `preset-discovery-crawl`, `-descriptor-consistency` and
`-load` on every MXM plugin with installed presets: the preset provider gives a `C:\…` path and
the validator takes only `/…` (its `src/plugin/preset_discovery/indexer.rs:178`, on `master` too),
a Unix-only check no absolute Windows path can pass (2026-11-04, the board's message 436). Every
other test passes, in debug and in release.

The reference's 220 cases and their digests (T1) are in `crates/mxm-thanks-eg-engines/tests/`.

# Child DOX Index

| Doc | Scope |
|---|---|
| [`crates/mxm-thanks-eg-engines/AGENTS.md`](crates/mxm-thanks-eg-engines/AGENTS.md) | The port: the porting contract, the bits, the arena, the reference cases |
| [`crates/mxm-thanks-eg-dsp/AGENTS.md`](crates/mxm-thanks-eg-dsp/AGENTS.md) | The instrument: the interface with the plugin, the voices, the envelopes with MATHS' functions, the LFOs, the routing, the resampler |
| [`plugins/mxm-thanks-eg/AGENTS.md`](plugins/mxm-thanks-eg/AGENTS.md) | The shell: the 101 parameters and the slots' lists, events and the callback, Init and the presets, the control map, and the editor: its twelve cards, the slots as rows on their targets' cards |
