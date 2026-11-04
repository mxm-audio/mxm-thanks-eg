# NOTES.md — mxm-thanks-eg

The reasoning behind `AGENTS.md`, and the owner's rulings. The plan, with the facts about Plaits and
MATHS it was designed from, is ops' `plans/plan-mxm-thanks-eg.md`; Pi reviewed it in three rounds
(ops' `plans/review-mxm-thanks-eg-2026-11-04*.md`).

## The owner's rulings

- **2026-11-04: The instrument.** *"Can we make a plugin based on mutable instruments plaits, with
  some good modulations sources, like envelopes and LFO's"*. Plaits is a monophonic Eurorack
  module whose only modulation inside is one decay envelope; everything else reaches it on a jack.
  So the plugin is the module's voice with the case around it: envelopes, LFOs and the
  performance sources, eight voices.
- **2026-11-04: The name.** *"It should be called the mxm-thanks-eg as a tribute to the original
  coder"*: Emilie Gillet. Her repository's *Guidelines for derivative works* ask that "Mutable
  Instruments" not be used on derivative works and recommend not keeping a module's name, so
  "Plaits" names the original in `NOTICE.md` and the README only.
- **2026-11-04: Plaits' data ships as Plaits has it.** *"Keep the words and patches to keep it
  similar. Everyobody does it"*, and on Pi's review, which held their redistribution as a legal
  blocker: *"Ignore what PI says. The words and patches MUST be in. I will handle it with
  legal"*. The speech words are TI's (Plaits: *"extracted from various TI ROMs"*) and the six-op
  patches come from DX7 banks; Emilie Gillet's MIT headers don't license either, and the owner
  ruled knowing that. No stage waits on it; `NOTICE.md` records where each came from.
- **2026-11-04: MATHS, in the envelopes.** First *"instead of one of the envelope generators we
  could add a 2-kanals funktionsgenerator og CV-processor"* (*"yes, it is the maths"*), then *"If
  we can implement the function generator by adding our standard modulation to envelope
  generators, that is a good way to go"*. MATHS' functions (one curve, Cycle, a signal input it
  follows, the end-of-rise and end-of-cycle gates, CV on rise, fall and both) became the
  envelopes'; its attenuverters, offsets and sum are what modulation slots already do, so they
  need nothing new. Its OR bus has no equivalent and is left out.
- **2026-11-04: Fewer parameters.** *"fewer parameters is better. Lets figure out a way to cut it
  down"*, and *"the drum machine redefined what the interface meant"*: the host holds what the
  panel can show at once, the principle mxm-model-drums and mxm-drum-machine follow (the owner,
  2026-09-30). Every source paired with every target, as the collection's synths offer them,
  would have been some 1 160 routing parameters; sixteen route slots of source, target and amount
  are 48, and the instrument 101 in all.
- **2026-11-04: Our own port, at the module's rate.** The plan's other recommendations, which the
  owner agreed: port from the C++ rather than fork `sourcebox/mi-plaits-dsp-rs`, whose maths come
  from `libm`, so its bits are not the C++'s; run every voice at 48 kHz in blocks of 12, as the
  module, and resample, because the engines' constants and tables are written for 48 kHz; after
  version 1.0 of the collection.

## The editor

Built for T4 from the plan's §5.5 and the collection's two precedents: mxm-mono-pr1's editor for a
synth's cards, the app bar, the preset row and its tests; mxm-drum-machine's for route slots drawn
as rows under the control they move. No new widget: every control is `mxm-ui`'s, bound through
`mxm_params::binding`, and the slots' rows are the drum machine's `route_stack` over this
instrument's sixteen slots.

- **Why each target sits on one card.** A slot can aim at any of 32 targets, so "under the knob it
  moves" becomes "on the card of the control it moves": an envelope's six targets (Rise, Fall, Time,
  Cycle, Trigger, Input) on its own card, an LFO's Rate on its own, the voice's on the cards that
  hold those controls; Level, Pan and Amplitude, which have no knob, on the gate's and the output's.
  A source is chosen there, never on a routing page of its own.
- **Which free slot a choice takes.** The plan says the first free one; the drum machine takes one
  already aimed at the knob first. Both hold: a slot this target had (its source removed) is taken
  first, so a removed source added back finds its depth, as the remove's hover promises; otherwise
  `Routes::first_free`. A slot taken from another target starts at zero depth, as the drum
  machine's does.
- **"Says so when all sixteen are in use."** The line's caret reads `all 16 in use`, disabled, where
  it offered `modulate`, on every card at once: no free slot is a fact about the instrument, not one
  card. It is measured with the line, and the row's track floor is wider still, so a full patch
  changes no card's width.
- **One page at the opening.** At the budget the pager first planned Voice alone and the other
  eleven in two rows, because the envelopes and the LFOs were preferred groups and a group stays on
  one row; the hugged window was then two pages with bare canvas under both. Without the groups all
  twelve stand on one page, the LFOs reading on across the row break in their order. The derived
  size leaves that page about a point of room, so `REFERENCE` is held a few points inside the
  check's gutter.
- **The review pictures forget the harness's first plan.** `tree_checks::pictures` draws its first
  frame before the collection's fonts take effect and the pager keeps the plan it made in egui's
  own (two pages); a window installs the fonts before its first frame. `tree_pictures` clears that
  plan in its setup, so the pictures show what a window opens on.

## The instrument's choices, for the owner to hear (T5)

Where the plan left a choice open, T3 made one; each is the owner's to keep or change by ear.

- **The envelopes' Response curve** is `(e^(6r·x) − 1) / (e^(6r) − 1)`, *r* the Response from −1 to
  +1: linear at 0, and it does not change the times. Times run 0.5 ms to 30 s on each knob.
- **Rise and Fall**, MATHS' linear CV, reach the whole range: 1 % of an amount moves a time by about
  0.3 s, coarse for fine work; Time (exponential, ×/÷ 16) is the one for small moves.
- **Glide** is exponential; in Poly a new note glides from the last note played.
- **Pan** is a balance law, unity at the centre in both channels; **Spread** crossfades the Aux mix
  toward OUT left, AUX right.
- **Smooth random** glides by a raised cosine to a new point each cycle.
- **Aux, Spread and Volume** are smoothed over 5 ms; the engines' own controls are not (the module
  interpolates them inside each block).
- **A voice goes idle**, and is no longer rendered, once its gate's gain is below 10⁻⁶; one whose
  engine bypasses the gate, after 128 ms within one step of silence. The plan's first rule, one
  silent block, failed three ways: zero crossings in quiet tails, the six-op engines' never-silent
  residue, and the particle engine's gaps while its gate is open.

## Open points

- **The chiptune engine drones after release** with its Timbre attenuverter at noon, as the module
  does, and no slot can close it; the instrument fades a released note out 30 s after its key went
  up. A guard, not a design: the owner rules what it should be.
- **Level does nothing on the eight engines that make their own envelope** (the six-op engines, the
  string, the modal resonator, the drums), so on those the only level a slot reaches is Amplitude,
  which can only trim (×0…×2): no slot-driven VCA there.
- **Plaits does not clamp Gate decay and Colour**; the instrument clamps them to 0…1.
- **A shared LFO's Rate** (Retrigger off) can only follow shared sources: Wheel, Pressure, Bend and
  the other shared LFOs.
- **"Env n input"** names the Follow mode's target as the plan and the DSP do, though the
  conventions say a target is never called *Input*: for the freeze (T5).
- **Copies the kit could hold**: the time reading is a second copy of mxm-mono-pr1's, the route
  slots a third copy of the drums' shape, and the resampler is this instrument's until MXM Step
  Sampler needs one. Moving any into mxm-kit needs the owner's word to push it.
