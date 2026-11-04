# AGENTS.md — plugins/mxm-thanks-eg

Parent: [`../../AGENTS.md`](../../AGENTS.md)

# Purpose

The nice-plug shell, CLAP and VST3, over the instrument in `crates/mxm-thanks-eg-dsp` (its
`src/lib.rs` is the interface): the 101 parameters, events, the realtime callback, state, the
preset system's wiring and the control map. Ops' `plans/plan-mxm-thanks-eg.md` §5 is the design.
**The editor is T4's** (`type Editor = ()` until then); everything it binds to is here.

# Ownership

`Cargo.toml`, `control-map.json`, `presets/` and `src/`: `lib.rs` (identity, the `Plugin` impl,
events, `process`), `params.rs` (the voice's 17, the envelopes' and LFOs' nested parameters, the
`Settings` fill), `routes.rs` (the sixteen slots and their lists), `preset.rs` (`mxm_preset::Instrument`
and the factory design), `telemetry.rs` (the atomics an editor reads). The DSP is the crate's.

# Local Contracts

## Identity and layouts

- `CLAP_ID` `dk.mxm.mxm-thanks-eg` from `plugin_name!`; `VST3_CLASS_ID` its hash, written out.
- Stereo, then mono (the two channels' average); no input of any kind.

## The 101 parameters

- **The ids** (`params::VOICE_IDS`, `ENVELOPE_IDS`, `LFO_IDS`, `routes::ROUTE_IDS`;
  `the_ids_are_unique_and_there_are_101`, `the_parameters_are_the_param_map_in_order`): the voice's
  `model frequency fine harmonics timbre morph fm_amount timbre_amount morph_amount decay colour
  aux spread mode glide bend_range volume`; `env<n>_{attack,decay,sustain,release,response,cycle,mode}`
  and `lfo<n>_{rate,shape,sync,retrigger,fade}`, `n` 1–3, nested by `id_prefix`;
  `route<r>_{source,target,amount}`, `r` 1–16. **Provisional until the gate (T5)**, then permanent,
  with the order of every list a state stores: Model's engines (`params::ENGINE_NAMES`), the voice
  and envelope modes, the LFO shapes, the slots' sources and targets.
- **The slots' lists are the DSP's**: `Off`, then `Source::ALL` or `Target::ALL` in order.
  `routes::SourceChoice` and `TargetChoice` carry the DSP's value and implement nice-plug's `Enum` by
  hand, their names from an exhaustive match, so a DSP addition fails to compile until named.
  Source names are short for a card (*Env 1*, *Env 1 rise end*, *LFO 1*, *Decay*, *Key* …).
- **Names are the instrument's own**, sentence case: never "Plaits", "Mutable Instruments",
  "MATHS" or "Make Noise" (`no_name_is_the_originals`). The decay envelope's attenuverters are
  *Pitch from decay*, *Timbre from decay*, *Morph from decay*; the gate's two *Gate decay* and
  *Colour*, as their targets are named. *Env n input* is the plan's and the DSP's word for what
  Follow follows, which the conventions' *Declaring the target list* names as one to avoid: the
  owner's to settle at T5.
- **Readings**: `mxm_params::text` where it has one; Model reads the engine's name, Frequency and
  Fine signed semitones, Volume (stored linear) decibels, a Rate hertz (a typed division picks its
  position), times by `params::time_text` (tenths of a millisecond, then hundredths of a second),
  a second copy of mxm-mono-pr1's `v2s_time`, due to move into `mxm_params::text`. Held by
  `every_parameter_text_survives_the_hosts_round_trip`.

## Init, smoothing, sync

- **Init**: every amount at zero (`params::AMOUNT_IDS`: the attenuverters, Spread, Glide, each Fade,
  every route amount), every slot `Off`, no sync, OUT alone on Virtual analogue; the rest
  configuration, set somewhere useful. Init is the CLAP defaults, generated, with no file
  (`init_is_every_amount_at_zero_and_nothing_routed`, `init_is_every_parameter_default_…`).
- **Smoothed**: what adds to or multiplies the sound (Fine, Harmonics, Timbre, Morph, the
  attenuverters, Colour, Aux, Spread, Volume, each Sustain, every route amount). Not: Model,
  Frequency, every time, rate, mode, shape and switch.
- **The patch, once per process call** (`ThanksEgParams::settings`), each LFO's Rate resolved
  there through `params::LFO_SYNC` (`Span::LFO`) at the call's tempo, free without one; **the
  smoothed signals step once per internal block** (`ThanksEgParams::advance`). The tempo in force
  is published in `telemetry.tempo` and forgotten at `activate`. No `TEMPO_SYNC_IDS`: the syncs
  were there from the first state.

## Route slots

- **mxm-drum-machine's scheme**: a slot is in use when neither end is `Off`; switching its source off
  keeps its target and amount; two on one pair add (the DSP's). The amount is −1…+1 of its
  target's reach, read as a percentage: the drums' **recorded deviation** from `routing_checks`'
  pair-by-pair readings, which need a parameter per pair.
- The slots stay here, a third copy of the shape after model-drums' and the drum machine's: moving
  them into `mxm-modulation-params` is kit work, the owner's to approve (the plan's §5.5).

## Events and the callback

- NoteOn with velocity and note id (velocity 0 is a NoteOff); NoteOff (a key-less one goes to every
  held key); Choke (`Key::Wildcard` is every note); `MidiPitchBend` (−1…1); CC 1 the wheel; channel
  pressure; `PolyTuning` by note (non-finite, or key-less, dropped); CC 120 chokes every note, CC
  123 lets every held key go (`held` counts them). Per-note pressure and the other expressions fall
  through.
- **Split at every event and capped at `MAX_BLOCK`**; events at the end, and in an empty buffer,
  apply before the next sample. Nothing allocates in `process` or `reset`; the mono scratch is a
  field.
- **The latency is `Instrument::latency_samples()`, reported in `activate`** through
  `ActivateContext::set_latency_samples` (the wrapper sends it as `activate` returns), after
  `set_sample_rate`; fixed for the activation.
- **`KeepAlive` while `Instrument::is_sounding()`, else `Normal`.** No `Tail(n)`: a cycling envelope
  in Gate mode or an LFO on Level sounds with no key held, so no tail length is honest.
- **Flush-to-zero** through nice-plug's `unsafe_flush_denormals`, as the engines' reference was
  rendered: a recorded deviation from the kit's *flush denormals in the DSP itself*, since a flush
  in the ported code would change its bits.

## Presets, map and telemetry

- `preset.rs` implements `mxm_preset::Instrument` (its test build reads this crate's `presets/`);
  `CLAP_PRESETS` and `preset_loader` let a host list and load the files. The design is
  `preset::tests::design`, overrides on Init in each parameter's own units; the ignored
  `write_the_factory_presets` writes the files and `order.txt`, and `the_shipped_files_are_the_design`
  fails when they differ. Each sound has a category; every bank of engines has one
  (`every_bank_has_a_sound`). **Nine drafts: the fifty, and their listening, are owed (T4).**
- `control-map.json` claims only matching roles (its `_comment` says why each; held by
  `control_map::the_map_holds_to_the_standard_and_names_real_parameters`). No route amount is
  claimed: a slot moves with its routing.
- `telemetry::Telemetry` (shared through `MxmThanksEg::telemetry`): the peak meter, voices
  sounding, the tempo, and the developer channel's requests (CC 116–119 with `MXM_DEV_CC` set).

# Work Guidance

- Change `params.rs`, `routes.rs` or the design, then regenerate:
  `cargo test -p mxm-thanks-eg --lib write_the_factory_presets -- --ignored`.
- The DSP is a stub until its branch merges: no test here depends on sound.
- `cargo xtask bundle mxm-thanks-eg --release` makes the `.clap` and `.vst3` and installs them;
  `MXM_CLAP_DIR` and `MXM_VST3_DIR` send a debug bundle elsewhere.

# Verification

```bash
cargo test -p mxm-thanks-eg
cargo clippy -p mxm-thanks-eg --all-targets -- -D warnings
cargo xtask bundle mxm-thanks-eg --release
clap-validator validate target/bundled/mxm-thanks-eg.clap
```

`clap-validator` 0.4.1 fails `preset-discovery-crawl`, `-descriptor-consistency` and `-load` once the
factory folder is installed: the nice-plug fork's provider declares the folder as a Windows path
(`C:\…`), and the validator takes only one starting with `/`. Every plugin with installed presets
fails the same three (mxm-mono-pr1 reproduced, 2026-11-04); the fix is the fork's. The other tests
pass in debug and release.

# Child DOX Index

None.
