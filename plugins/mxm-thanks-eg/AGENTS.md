# AGENTS.md — plugins/mxm-thanks-eg

Parent: [`../../AGENTS.md`](../../AGENTS.md)

# Purpose

The nice-plug shell, CLAP and VST3, over the instrument in `crates/mxm-thanks-eg-dsp` (its
`src/lib.rs` is the interface): the 101 parameters, events, the realtime callback, state, the
preset system's wiring, the control map and the editor. Ops' `plans/plan-mxm-thanks-eg.md` §5 is
the design.

# Ownership

`Cargo.toml`, `control-map.json`, `presets/`, `tests/` (the factory set's levels), `host-tests/`
(the slow tier) and `src/`: `lib.rs` (identity, the `Plugin` impl, events, `process`), `params.rs`
(the voice's 17, the envelopes' and LFOs' nested parameters, the `Settings` fill), `routes.rs`
(the sixteen slots and their lists), `preset.rs` (`mxm_preset::Instrument` and the factory design),
`telemetry.rs` (the atomics the editor reads), and the editor: `editor.rs` (the window, the app
bar, paging, the developer Parameters list), `editor/cards.rs` (the cards as `mxm_ui::tree`s and
every parameter's binding), `editor/route_rows.rs` (the slots as rows), `editor/binding.rs`
(`mxm_params::binding`, re-exported) and `editor/proof.rs` (its tests). The DSP is the crate's.

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
  fails when they differ. At least fifty (`the_set_is_at_least_fifty_sounds`), each with its
  category, every engine with one (`every_engine_and_bank_has_a_sound`), names short, sentence case
  and the set's own (`every_name_is_its_own_short_and_in_sentence_case`); the order a first
  impression, then `Category::ALL`'s. **Their listening is owed (T4).**
- **The set's levels are `tests/factory_levels.rs`**: each file loaded as a host loads it, played
  as its category is (at 120 bpm), its BS.1770 momentary loudness at its loudest within `WINDOW_LU`
  of the set's median, no sample past full scale, and silent within `ENDS_WITHIN` of its key going
  up. **Balanced in the sound, never by Volume** (`factory_content_does_not_move_volume`): a small
  Level amount at Colour 1 for a sound through the gate; for an engine that envelopes itself, its
  knobs and its accent (`Sound::struck`).
- `control-map.json` claims only matching roles (its `_comment` says why each; held by
  `control_map::the_map_holds_to_the_standard_and_names_real_parameters`). No route amount is
  claimed: a slot moves with its routing.
- `telemetry::Telemetry` (shared through `MxmThanksEg::telemetry`): the peak meter, voices
  sounding, the tempo, and the developer channel's requests (CC 116–119 with `MXM_DEV_CC` set).

## Editor ([NOTES.md § The editor](../../NOTES.md#the-editor))

- **Twelve cards in the conventions' category order** (`cards::CARDS`): Voice (Performance);
  Envelope 1–3 and LFO 1–3 (Modulators); Engine, Shaping, From decay (Generators); Low-pass gate,
  Output (Tone). **Volume is the app bar's**, beside the meter (`editor::BAR_PARAMETER`,
  `VOLUME_CARD`). Every control is the collection's, through `mxm_params::binding`; a painted name
  drops what its card says (`cards::panel_of`: an envelope's faders read A, D, S, R), the
  parameter's name stays the host's, the tooltip's and the accessibility tree's.
- **A route slot is a row on the card of its target** (`Card::targets`: each of the 32 on one card,
  the card of what it moves; `every_target_has_its_routes_on_exactly_one_card`), the drum machine's
  shape: the target's line offers `‹ modulate ›`; a choice takes a free slot in one bracketed
  gesture, one this target had first, so a source removed and added back keeps its depth, else
  `Routes::first_free`, a depth left from another target zeroed (`route_rows::add`). A row is the
  amount, painted as its source, and a remove that switches the source off, one write
  (`route_rows::remove`). **A slot not in use is on no card.** With all sixteen in use every line
  reads `route_rows::FULL`, disabled, and offers nothing.
- **The engine picker** is the caret selector grouped by bank (`selector_grouped`); its hover says
  what the chosen engine sounds like (`cards::engine_sound`), in the player's words.
- **It opens on one page with every card**: `REFERENCE` is the budget hugged
  (`the_opening_size_is_the_budget_hugged`), held a few points inside that check's gutter so the one
  page has room (`the_window_opens_on_one_page_with_every_card`). **No preferred groups**: grouping
  the envelopes and the LFOs splits that page in two. `MINIMUM` is the app bar's last compact step
  (`the_app_bar_holds_in_the_minimum_window`).
- Step laws: Frequency and Bend range `Semitones`, the rest their own. Every knob is Standard, so
  each shows its reading; a synced Rate reads its division at the tempo in force.
- CC 118 asks for an expander this editor has not got: taken, and nothing changes. CC 119's 127 is
  the developer Parameters list, every parameter by its full name.
- `editor/proof.rs` holds the tree checks in every state that changes a card, paging, reflow (rows,
  floors, order), clipping, the keyboard cursor over every control and every slot, F1, BACK on a
  drag, the slots' gestures, the developer channel, both themes, and the review pictures.

# Work Guidance

- Change `params.rs`, `routes.rs` or the design, then regenerate:
  `cargo test -p mxm-thanks-eg --lib write_the_factory_presets -- --ignored`.
- The level test is the one here that depends on sound; `-- --nocapture` prints each sound's
  loudness, peak and ring.
- **Designing a sound, the gotchas**: Level reaches the gate through the module's compression
  (`1.3L/(0.3+L)`, so −6 dB is about 0.19) and darkens it unless Colour is 1, where it is a plain
  VCA. An engine that envelopes itself (the six-op, the chiptune, the string, the modal, the drums,
  speech's words) bypasses the gate: Level is only its accent, and it ignores the key going up, so
  the string and the modal ring as long as Morph says (from about 0.7 to the 30 s guard) unless a
  slot damps them (Envelope 1 into Morph). A Follow envelope whose input stays above zero after
  the note (Key above middle C) never rests, and the voice then never idles, guard or not: take
  the input away with another envelope's end of cycle.
- `cargo xtask bundle mxm-thanks-eg --release` makes the `.clap` and `.vst3` and installs them;
  `MXM_CLAP_DIR` and `MXM_VST3_DIR` send a debug bundle elsewhere.

# Verification

```bash
cargo test -p mxm-thanks-eg     # with tests/factory_levels.rs, the set's loudness
cargo clippy -p mxm-thanks-eg --all-targets -- -D warnings
cargo xtask bundle mxm-thanks-eg --release
clap-validator validate target/bundled/mxm-thanks-eg.clap
# Every page, light and dark, at Init and with slots in use, for review:
# target/layout-tree/mxm-thanks-eg/<MXM_PICTURES tag>/{init,routed}/
MXM_PICTURES=after cargo test -p mxm-thanks-eg --lib tree_pictures -- --ignored
```

`clap-validator` 0.4.1 fails `preset-discovery-crawl`, `-descriptor-consistency` and `-load` once the
factory folder is installed: the nice-plug fork's provider declares the folder as a Windows path
(`C:\…`), and the validator takes only one starting with `/`. Every plugin with installed presets
fails the same three (mxm-mono-pr1 reproduced, 2026-11-04); the fix is the fork's. The other tests
pass in debug and release.

# Child DOX Index

None.
