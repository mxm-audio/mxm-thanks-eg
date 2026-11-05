# mxm-thanks-eg — UI design brief

Required by mxm-kit's [`MXM_DESIGN_SYSTEM.md`](https://github.com/mxm-audio/mxm-kit/blob/main/docs/MXM_DESIGN_SYSTEM.md)
§14. The instrument plays the 24 engines of a published macro-oscillator module, ported to Rust
from its maker's MIT code and held bit for bit to it (`mxm-macro-osc`), with
envelopes, LFOs and route slots around them (`crates/mxm-thanks-eg-dsp`). The design is ops'
`plans/plan-mxm-thanks-eg.md` §5; the interface is not a copy of the module's panel.

**Brief status: written after the editor's first build, 2026-11-04** (the collection asks for it
before; this instrument's editor was built from the plan's §5, and this brief records what that
plan and that build decided). Automated tree, layout, cursor and picture checks pass; the owner's
listening, a real DAW, native-window fit and design-system §15 QA remain the release gates.

## Product boundary

- **The engines are the module's**: what Harmonics, Timbre and Morph do is each engine's, as the
  module has it. The instrument adds what the module took from its case: eight voices, three
  envelopes that are function generators too, three LFOs and sixteen route slots.
- **No effect**: the module has none (the conventions' *An instrument ships the effects its
  original had*).
- **No maker's or module's name** anywhere in the editor; the engines carry the instrument's names
  (`params::ENGINE_NAMES`).

## Established warts to preserve

The module's, ported and kept: an idle output of +1 step (silenced here only once a voice is idle),
the six-op engines' first block after a model change reading leftover memory, a drone after release
on the chiptune engine (faded after 30 s by the instrument, a guard the owner will rule on), and
the engines' 16-bit grain. See `research:oscillators/mutable-instruments-plaits.md` §11.

## 1. Primary sound-design task

Choose an engine and find its sound with the four controls (Frequency, Harmonics, Timbre, Morph),
then make it move: the gate's decay, an envelope or LFO on Harmonics, Timbre or Morph.

## 2. The parameters users reach for most

Model, Harmonics, Timbre, Morph; the gate's Decay; an envelope's times.

## 3. Signal flow that must be visible

Engine (Model, Frequency, Fine) → its four controls (Shaping) → the low-pass gate (Decay, Colour,
Level) → the output (Aux, Spread) → Volume in the app bar. Modulators (envelopes, LFOs, the
module's own decay envelope) reach any of these through route slots, drawn as rows on the card of
the control they move.

## 4. Play view

The single page at the opening size: every card shown; nothing is behind a disclosure.

## 5. Advanced controls and disclosure

None hidden: the route slots appear as rows only when in use, and a card's *modulate* line adds
one. When all sixteen are in use, every card's line says so.

## 6. Categories, cards and groups

In the collection's category order: **Performance** (Voice: Mode, Glide, Bend range);
**Modulators** (Envelope 1–3, LFO 1–3); **Generators** (Engine, Shaping, From decay); **Tone**
(Low-pass gate, Output). No preferred groups: with them the opening split onto two pages with
bare canvas under both (`NOTES.md`, *One page at the opening*).

## 7. Identity accent

The shared collection accent (dark `#4CC9D8`, light `#247F91`), as mxm-mono-pr1's brief records
it measured; no accent of the module's.

## 8. Live visualizations

The meter in the app bar; each LFO's shape drawn in its picker. A drawn envelope (its Response
curve) and live modulator levels would need the DSP to expose the curve and the levels: later.

## 9. What is removed from the source's panel, and why

The bank buttons and LEDs (Model is a grouped picker), the hidden-settings menu (Decay and Colour
are plain controls), the CV jacks (route slots), the trigger and level inputs (the keyboard; a slot
on Level), the octave switch (a range of Frequency).

## 10. Quarter-4K fit, smaller sizes and 200% zoom

Measured by the editor's tests, not guessed: it opens at 1676 × 1070 logical points with all twelve
cards on one page; the minimum is 446 × 360, set by the app bar. Every page passes the tree and
layout checks in both themes. **No native-window, DPI or 200% run on a real display yet**: §15
responsive QA remains a manual gate.

## Init patch

Engine 8 (virtual analogue), Harmonics, Timbre and Morph at the middle, the gate's Decay and Colour
at the middle, the module's three attenuverters at zero; Poly, no glide, bend ±2 semitones; each
envelope 5 ms / 300 ms / 70 % / 300 ms, linear, Note mode, not cycling; the LFOs at 5, 1 and 0.2
Hz, sine, free-running, no fade; no route slot in use; Volume −9 dB. Every amount at zero, every
configuration musically useful, one set of numbers with the CLAP defaults (the plugin's Init test).
