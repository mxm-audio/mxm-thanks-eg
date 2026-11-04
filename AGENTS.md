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

Pre-alpha: T0 (this repository) is done and T1 (the research and the reference harness) is under
way; no code yet. Private until it is ready; after version 1.0 of the MXM collection.

# Ownership

Root owns `AGENTS.md`, `NOTES.md`, `README.md`, `NOTICE.md`, `LICENSE`, `TRADEMARKS.md`,
`CONTRIBUTING.md`, `.gitignore` and `.gitattributes`. From T2: `crates/mxm-thanks-eg-engines` (the
port), `crates/mxm-thanks-eg-dsp` (voices, envelopes, LFOs, the slots' routing),
`plugins/mxm-thanks-eg` and `xtask`, each with its own `AGENTS.md`.

The reference harness, the C++ it builds and the research pages are in the private research
repository (`research:plaits-engine/`, `research:oscillators/`, `research:modulation/`), never
here: this repository commits only the harness's cases and its outputs' SHA-256s.

# Local Contracts

## The engines are a port, held to the C++

- **Rust only**: no C++ in the plugin and no FFI. Ported from `pichenettes/eurorack`'s `plaits/`
  and the parts of `stmlib` it uses, at the commits the research harness pins; each ported file
  keeps its original's copyright line, and `NOTICE.md` the MIT text.
- **Bit for bit against the C++ at 48 kHz**, Windows' bits, in blocks of 12 as the module. The one
  exception: a case whose engine calls a maths function where MSVC's library and Rust's differ
  passes at most 10⁻⁶ apart, and names the call.
- **The one change from the C++ is the tuning**: `a0` takes 48 000 where the module takes its DAC's
  47 872.34 Hz, or every note would be 4.6 cents flat. The harness makes the same change.
- **Each voice has its own random stream**, seeded as the C++ seeds its one static `Random`, so a
  one-voice case matches.
- **The tables are made by a Rust generator** ported from `plaits/resources/*.py`, each equal to
  `plaits/resources.cc`'s.
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

None yet: T1 builds the harness; T2's port is checked against it.

# Child DOX Index

None yet.
