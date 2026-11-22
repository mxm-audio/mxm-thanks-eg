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
the instrument; the editor, the factory sounds, the control map and the host tests. T5 is done
(2026-11-05): the parameters are frozen and permanent, and the owner heard the instrument and its
sounds (*"Thanks EG sounded fine"*). In version 1.0 of the MXM collection (the owner,
2026-11-17), and public since 2026-11-22, out to the beta testers (the owner: *"If they are out in beta, their repos should also be published"*).

# Ownership

Root owns `AGENTS.md`, `NOTES.md`, `README.md`, `NOTICE.md`, `LICENSE`, `TRADEMARKS.md`,
`CONTRIBUTING.md`, `Cargo.toml`, `Cargo.lock`, `bundler.toml`, `.cargo/config.toml` (the `xtask`
alias), `.gitignore` and `.gitattributes`, and `xtask/` (the collection's shared `mxm-xtask`:
`bundle`), and `docs/briefs/mxm-thanks-eg.md` (the design system's §14 brief).
The port is mxm-macro-osc, its own repository (below). `crates/mxm-thanks-eg-dsp` (the
instrument: voices, envelopes, LFOs, the slots' routing, the rate converter) and `plugins/mxm-thanks-eg`
(the shell and editor, with `host-tests/`, the slow tier) have their own `AGENTS.md`.

The reference harness, the C++ it builds and the research pages are in the private research
repository (`research:sources/mutable-instruments-plaits/`, its harness in `derived/harness/`;
`research:oscillators/`, `research:modulation/`), never here nor in mxm-macro-osc, which commits
only the harness's cases and its outputs' SHA-256s. The harness makes the port's one change, the
rate; its README lists its two compiler workarounds, which change nothing.

# Local Contracts

## The engines: mxm-macro-osc

- **The port is its own repository**, `mxm-audio/mxm-macro-osc` (`libs/mxm-macro-osc` in the
  workspace), public and MIT since 2026-11-05, taken by its `main`, locked. It holds Plaits' voice
  bit for bit to the C++ at 48 kHz, in blocks of 12, on the 16-bit frames the voice returns, and
  its `AGENTS.md` has the porting rules. A change to the engines is made there, keeping its 220
  cases bit for bit, then relocked here.
- **Rust only**: no C++ in the plugin and no FFI.
- **Flush-to-zero is on**: the plugin takes nice-plug's `unsafe_flush_denormals`, as the reference
  was rendered, so what ships is the reference's bits.
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
- **Every voice runs at 48 kHz**, and one rate converter (mxm-kit's `mxm_dsp::rate_converter`)
  takes the sum to the host's rate with a fixed, reported latency; at 48 kHz nothing is converted.
- The collection's contract, as every MXM instrument (the plan's §5.7).

# Work Guidance

- Build and test on Windows only, as far as `python ws.py reach` says a change goes; the other
  platforms in the owner's batch (the workspace's `AGENTS.md`).
- Engine work in mxm-macro-osc, built here first through `python ws.py link mxm-macro-osc`, as
  kit work (the rate converter's) through `python ws.py link kit`, unlinked
  before any push; pushing mxm-kit's public `main` needs the owner's word.

# Verification

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test                 # the fast tier: the instrument and the plugin
cargo xtask bundle mxm-thanks-eg --release
cargo test -p mxm-thanks-eg-host-tests      # the slow tier: the release bundle in MXM Player
clap-validator validate target/bundled/mxm-thanks-eg.clap
```

`clap-validator` 0.4.1 on Windows fails `preset-discovery-crawl`, `-descriptor-consistency` and
`-load` on every MXM plugin with installed presets: the preset provider gives a `C:\…` path and
the validator takes only `/…` (its `src/plugin/preset_discovery/indexer.rs:178`, on `master` too),
a Unix-only check no absolute Windows path can pass (2026-11-04, the board's message 436). Every
other test passes, in debug and in release.

The reference's 220 cases and their digests (T1) are mxm-macro-osc's, in its `tests/`, and its
`cargo test` checks them; here the instrument's tests check its link to the engines.

# Child DOX Index

| Doc | Scope |
|---|---|
| [`crates/mxm-thanks-eg-dsp/AGENTS.md`](crates/mxm-thanks-eg-dsp/AGENTS.md) | The instrument: the interface with the plugin, the voices, the envelopes with MATHS' functions, the LFOs, the routing, the rate converter |
| [`plugins/mxm-thanks-eg/AGENTS.md`](plugins/mxm-thanks-eg/AGENTS.md) | The shell: the 101 parameters and the slots' lists, events and the callback, Init and the presets, the control map, and the editor: its twelve cards, the slots as rows on their targets' cards |
