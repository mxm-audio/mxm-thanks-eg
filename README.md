# MXM Thanks EG

A polyphonic macro-oscillator instrument, CLAP and VST3: twenty-four synthesis engines, each played
from the same four controls, with envelopes that are function generators too, LFOs and sixteen
modulation slots. Its engines are ported to Rust from Emilie Gillet's code for Mutable Instruments
Plaits, which she published under the MIT licence; the name thanks her. She has no part in this
product and does not endorse it.

Eight voices; per voice three envelopes, each also a function generator (cycling, gated, or
following another signal), and three LFOs; sixteen route slots from nineteen sources to thirty-two
targets; fifty-two factory sounds. The engines are held bit for bit to the code they were ported
from; the port is its own crate, [mxm-macro-osc](https://github.com/mxm-audio/mxm-macro-osc)
(MIT). `cargo xtask bundle mxm-thanks-eg --release` builds and installs the CLAP and VST3.

Pre-alpha: nothing is released, and the repository is private until it is ready.

## Licence

GPL-3.0-or-later (`LICENSE`), with the MIT-licensed code it is ported from named in `NOTICE.md`.
The MXM name and logo are not covered by the licence: see `TRADEMARKS.md`.
