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
