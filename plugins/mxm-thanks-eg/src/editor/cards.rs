//! The cards and every parameter's binding.
//!
//! Each card is a `mxm_ui::tree` (`plans/plan-layout-tree.md`): [`card`] describes its body once,
//! and that one description is both measured — its floor and its height — and drawn, leaf by leaf,
//! through the bindings below ([`paint`]). Nothing is typed and nothing is drawn to learn a size.
//!
//! **The cards, in the conventions' category order** (`docs/plugin-conventions.md`, *Editor
//! contract*): Voice (Performance); the three envelopes and the three LFOs (Modulators); the
//! engine, its three sound controls and the decay envelope's three amounts (Generators); the
//! low-pass gate and the output (Tone). **Each target's routes are rows on the card of the control
//! it moves** ([`super::route_rows`]); Volume, the instrument's output level, is the app bar's.

use std::collections::HashMap;

use egui::Ui;
use mxm_thanks_eg_dsp::Target;
use mxm_ui::control::{Size, Wave};
use mxm_ui::paging::Category;
use mxm_ui::space::SPACE_3;
use mxm_ui::theme::Tokens;
use mxm_ui::tree::{self, Height, Kind, Node, leaf, pad, stack};
use nice_plug::prelude::ParamSetter;

use super::binding::{self, Bound, ErasedParam, StepLaw};
use super::route_rows;
use crate::params::{ENGINE_NAMES, ENVELOPE_IDS, LFO_IDS, LFO_SYNC, ThanksEgParams, VOICE_IDS};
use crate::routes::ROUTE_IDS;

/// Every knob is Standard: each always shows its reading, a synced rate's division among them
/// (`docs/plugin-conventions.md`, *A syncable rate or time shows its reading*).
const STANDARD: Size = Size::Standard;

/// One card of the editor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Card {
    /// How the voices take notes: Mode, Glide, Bend range.
    Voice,
    /// Envelope `n`, from 0: its seven, and the routes of its six targets.
    Envelope(usize),
    /// LFO `n`, from 0: its five, and its Rate's routes.
    Lfo(usize),
    /// Model, Frequency and Fine, with the routes of Pitch and Model.
    Engine,
    /// Harmonics, Timbre and Morph, with their routes.
    Shaping,
    /// The decay envelope's three amounts: Pitch, Timbre and Morph from decay.
    FromDecay,
    /// The low-pass gate's Decay and Colour, with the routes of Gate decay, Colour and Level.
    Gate,
    /// Aux and Spread, with the routes of Aux, Pan and Amplitude.
    Output,
}

/// The cards in paging order: the category order, then the signal's.
pub const CARDS: [Card; 12] = [
    Card::Voice,
    Card::Envelope(0),
    Card::Envelope(1),
    Card::Envelope(2),
    Card::Lfo(0),
    Card::Lfo(1),
    Card::Lfo(2),
    Card::Engine,
    Card::Shaping,
    Card::FromDecay,
    Card::Gate,
    Card::Output,
];

/// Each envelope's targets, in the order its card draws their routes: the times, then the gates.
pub const ENVELOPE_TARGETS: [[Target; 6]; 3] = [
    [
        Target::Envelope1Rise,
        Target::Envelope1Fall,
        Target::Envelope1Time,
        Target::Envelope1Cycle,
        Target::Envelope1Trigger,
        Target::Envelope1Input,
    ],
    [
        Target::Envelope2Rise,
        Target::Envelope2Fall,
        Target::Envelope2Time,
        Target::Envelope2Cycle,
        Target::Envelope2Trigger,
        Target::Envelope2Input,
    ],
    [
        Target::Envelope3Rise,
        Target::Envelope3Fall,
        Target::Envelope3Time,
        Target::Envelope3Cycle,
        Target::Envelope3Trigger,
        Target::Envelope3Input,
    ],
];

/// Each LFO's one target, its Rate.
pub const LFO_TARGETS: [Target; 3] = [Target::Lfo1Rate, Target::Lfo2Rate, Target::Lfo3Rate];

impl Card {
    pub const fn title(self) -> &'static str {
        match self {
            Self::Voice => "Voice",
            Self::Envelope(0) => "Envelope 1",
            Self::Envelope(1) => "Envelope 2",
            Self::Envelope(_) => "Envelope 3",
            Self::Lfo(0) => "LFO 1",
            Self::Lfo(1) => "LFO 2",
            Self::Lfo(_) => "LFO 3",
            Self::Engine => "Engine",
            Self::Shaping => "Shaping",
            Self::FromDecay => "From decay",
            Self::Gate => "Low-pass gate",
            Self::Output => "Output",
        }
    }

    /// Its primary category (design system §3.4).
    pub const fn category(self) -> Category {
        match self {
            Self::Voice => Category::Performance,
            Self::Envelope(_) | Self::Lfo(_) => Category::Modulators,
            Self::Engine | Self::Shaping | Self::FromDecay => Category::Generators,
            Self::Gate | Self::Output => Category::Tone,
        }
    }

    /// The name of a run of like cards, for a page that splits a category.
    pub const fn kind(self) -> &'static str {
        match self {
            Self::Voice => "Voice",
            Self::Envelope(_) => "Envelope",
            Self::Lfo(_) => "LFO",
            Self::Engine | Self::Shaping | Self::FromDecay => "Engine",
            Self::Gate => "Gate",
            Self::Output => "Output",
        }
    }

    /// **The musician controls the card owns**, by permanent id. Route slots are not listed: a row
    /// is drawn because its slot is in use, so at Init none is ([`Card::targets`] is where they go).
    pub fn parameters(self) -> Vec<&'static str> {
        match self {
            Self::Voice => vec!["mode", "glide", "bend_range"],
            Self::Envelope(n) => ENVELOPE_IDS[n].to_vec(),
            Self::Lfo(n) => LFO_IDS[n].to_vec(),
            Self::Engine => vec!["model", "frequency", "fine"],
            Self::Shaping => vec!["harmonics", "timbre", "morph"],
            Self::FromDecay => vec!["fm_amount", "timbre_amount", "morph_amount"],
            Self::Gate => vec!["decay", "colour"],
            Self::Output => vec!["aux", "spread"],
        }
    }

    /// **The targets whose routes are rows on this card**: each target on exactly one card, the
    /// card of the control it moves.
    pub fn targets(self) -> &'static [Target] {
        match self {
            Self::Voice | Self::FromDecay => &[],
            Self::Envelope(n) => &ENVELOPE_TARGETS[n],
            Self::Lfo(n) => std::slice::from_ref(&LFO_TARGETS[n]),
            Self::Engine => &[Target::Pitch, Target::Model],
            Self::Shaping => &[Target::Harmonics, Target::Timbre, Target::Morph],
            Self::Gate => &[Target::GateDecay, Target::Colour, Target::Level],
            Self::Output => &[Target::Aux, Target::Pan, Target::Amplitude],
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The cards, as trees.
// ---------------------------------------------------------------------------------------------

/// What a leaf of this editor's cards draws, by the permanent id of what it edits. Hashed by what
/// it names, which is also what keeps its widget ids stable when a route appears above it.
#[derive(Clone, Debug, Hash)]
pub enum Leaf {
    Knob(&'static str),
    /// One of an envelope's A, D, S and R, in the collection's fader row.
    Fader(&'static str),
    /// A stepped switch as a segmented control.
    Switch(&'static str),
    /// An on/off switch under its name.
    Toggle(&'static str),
    /// An LFO's tempo sync, the quarter note beside its Rate.
    Sync(&'static str),
    /// An LFO's shapes, drawn.
    Shape(&'static str),
    /// The engine picker.
    Model,
    /// A target's routes, by its index in `Target::ALL`.
    Routes(usize),
}

/// The LFO shapes, in `LfoShapeChoice`'s order, drawn rather than named (design system §7.3).
pub const LFO_SHAPES: [(Wave, &str); 7] = [
    (Wave::Sine, "Sine"),
    (Wave::Triangle, "Triangle"),
    (Wave::RampUp, "Saw up"),
    (Wave::RampDown, "Saw down"),
    (Wave::Square, "Square"),
    (Wave::Random, "Sample and hold"),
    (Wave::SmoothRandom, "Smooth random"),
];

/// What each LFO shape does, in [`LFO_SHAPES`]' order: a sentence for each cell (§7.3).
const LFO_DETAILS: [&str; 7] = [
    "A smooth wobble.",
    "Rises and falls in straight lines.",
    "Rises, then snaps back down.",
    "Falls, then snaps back up.",
    "Jumps between two values, like a trill.",
    "A new random value on every cycle, held until the next.",
    "Glides from one random value to the next.",
];

/// The engines in banks of eight, as the picker groups them.
const BANKS: [&str; crate::params::BANKS] = ["Bank 1", "Bank 2", "Bank 3"];

/// The engine picker's painted label.
pub const MODEL_LABEL: &str = "Model";

/// A stepped switch's cells, **labelled by the parameter itself**: each its option's own formatted
/// value, so a cell reads what the host's automation list reads.
fn switch_options(p: &ThanksEgParams, id: &'static str) -> Vec<String> {
    let param = binding_for(id, p).param;
    let last = param
        .steps()
        .unwrap_or_else(|| unreachable!("{id} is not a switch"));
    (0..=last)
        .map(|option| param.format(option as f32 / last as f32))
        .collect()
}

/// A knob, in a column never narrower than its widest reading — a synced Rate's divisions
/// included, so its column holds the reading whichever way its sync is.
fn knob(ui: &Ui, p: &ThanksEgParams, id: &'static str) -> Node<Leaf> {
    let bound = binding_for(id, p);
    let widest = if lfo_of_rate(id).is_some() {
        binding::synced_widest(bound.param, LFO_SYNC.span)
    } else {
        mxm_ui::control::widest_value(|n| bound.param.format(n as f32))
    };
    let column = if STANDARD.value_always_visible() {
        let font = mxm_ui::typography::value_style(ui.style()).resolve(ui.style());
        ui.painter()
            .layout_no_wrap(widest.clone(), font, egui::Color32::PLACEHOLDER)
            .size()
            .x
    } else {
        0.0
    };
    leaf(
        Leaf::Knob(id),
        Kind::Knob {
            name: bound.painted().to_owned(),
            widest,
            size: STANDARD,
            column,
        },
    )
}

/// The collection's knob row (`mxm_ui::tree::knob_row`).
fn knobs(ui: &Ui, p: &ThanksEgParams, ids: &[&'static str]) -> Node<Leaf> {
    tree::knob_row(
        ui,
        ids.iter().map(|id| (STANDARD, knob(ui, p, id))).collect(),
    )
}

/// An envelope's A, D, S and R as the collection's fader row (`mxm_ui::tree::fader_row`).
fn faders(ui: &Ui, p: &ThanksEgParams, ids: &[&'static str]) -> Node<Leaf> {
    tree::fader_row(
        ui,
        ids.iter()
            .map(|&id| {
                let bound = binding_for(id, p);
                tree::fader(
                    Leaf::Fader(id),
                    bound.painted(),
                    mxm_ui::control::widest_value(|n| bound.param.format(n as f32)),
                )
            })
            .collect(),
    )
}

fn switch(p: &ThanksEgParams, id: &'static str) -> Node<Leaf> {
    leaf(
        Leaf::Switch(id),
        Kind::Segmented {
            label: binding_for(id, p).painted().to_owned(),
            options: switch_options(p, id),
            beside: None,
        },
    )
}

fn toggle(p: &ThanksEgParams, id: &'static str) -> Node<Leaf> {
    leaf(
        Leaf::Toggle(id),
        Kind::Toggle {
            label: binding_for(id, p).painted().to_owned(),
        },
    )
}

/// A target's routes. A composite with a rule of its own — its floor is every source a row could
/// show — so it states its size (`route_rows::size`) and fills the card's width.
fn routes(ui: &Ui, p: &ThanksEgParams, target: Target) -> Node<Leaf> {
    let size = route_rows::size(ui, &p.routes, target);
    leaf(
        Leaf::Routes(route_rows::index_of(target)),
        Kind::Custom {
            min_width: size.x,
            height: Height::Fixed(size.y),
            fills: true,
        },
    )
}

/// Every route stack of `card`, `SPACE_3` below what precedes them.
fn stacks(ui: &Ui, p: &ThanksEgParams, card: Card) -> Vec<Node<Leaf>> {
    card.targets()
        .iter()
        .enumerate()
        .map(|(n, target)| {
            let stack = routes(ui, p, *target);
            if n == 0 { pad(SPACE_3, stack) } else { stack }
        })
        .collect()
}

/// A card's body, as a tree, from the parameters.
pub fn card(ui: &Ui, card: Card, p: &ThanksEgParams) -> Node<Leaf> {
    let gap = ui.spacing().item_spacing.x;
    let mut body = match card {
        Card::Voice => vec![switch(p, "mode"), knobs(ui, p, &["glide", "bend_range"])],
        // A, D, S and R as faders, the convention's (the owner, 2026-09-25: *"ADSRs are typically
        // sliders"*), then the curve, what drives it and Cycle.
        Card::Envelope(n) => {
            let ids = ENVELOPE_IDS[n];
            vec![
                faders(ui, p, &ids[..4]),
                knobs(ui, p, &[ids[4]]),
                switch(p, ids[6]),
                toggle(p, ids[5]),
            ]
        }
        // Rate, its quarter note straight after it and then Fade, one flat row (the conventions'
        // *Tempo sync*); then the shapes and Retrigger.
        Card::Lfo(n) => {
            let ids = LFO_IDS[n];
            vec![
                tree::row_gap(
                    gap,
                    vec![
                        knobs(ui, p, &[ids[0]]),
                        tree::switch_beside_knob(
                            STANDARD,
                            leaf(Leaf::Sync(ids[2]), Kind::SyncToggle),
                        ),
                        knobs(ui, p, &[ids[4]]),
                    ],
                ),
                leaf(
                    Leaf::Shape(ids[1]),
                    Kind::Waves {
                        label: Some(binding_for(ids[1], p).painted().to_owned()),
                        count: LFO_SHAPES.len(),
                        marks: Vec::new(),
                        beside: None,
                    },
                ),
                toggle(p, ids[3]),
            ]
        }
        Card::Engine => vec![
            leaf(
                Leaf::Model,
                Kind::Selector {
                    label: MODEL_LABEL.to_owned(),
                    options: ENGINE_NAMES.iter().map(|name| (*name).to_owned()).collect(),
                    caption: false,
                    width: None,
                },
            ),
            knobs(ui, p, &["frequency", "fine"]),
        ],
        Card::Shaping => vec![knobs(ui, p, &["harmonics", "timbre", "morph"])],
        Card::FromDecay => vec![knobs(
            ui,
            p,
            &["fm_amount", "timbre_amount", "morph_amount"],
        )],
        Card::Gate => vec![knobs(ui, p, &["decay", "colour"])],
        Card::Output => vec![knobs(ui, p, &["aux", "spread"])],
    };
    body.extend(stacks(ui, p, card));
    stack(body)
}

/// Everything a leaf draws with: the parameters and their host, the tempo in force and the text
/// buffers.
pub struct Live<'a, 'b> {
    pub params: &'a ThanksEgParams,
    /// The host tempo in force: a synced Rate reads its division with one and its hertz without.
    pub tempo: Option<f64>,
    pub setter: &'a ParamSetter<'b>,
    pub entries: &'a mut HashMap<&'static str, Option<String>>,
}

/// Draws one leaf, in the `Ui` the tree bounded to `rect`, through the bindings — so the controls,
/// their gestures and their names are the collection's.
pub fn paint(ui: &mut Ui, tokens: &Tokens, leaf: &Leaf, rect: egui::Rect, live: &mut Live<'_, '_>) {
    let p = live.params;
    let setter = live.setter;
    match *leaf {
        Leaf::Knob(id) => {
            let bound = binding_for(id, p);
            match synced_reading(p, id, live.tempo) {
                Some(reading) => bound.knob_with_reading(
                    ui,
                    tokens,
                    setter,
                    STANDARD,
                    rect.width(),
                    live.entries,
                    reading,
                ),
                None => bound.knob(ui, tokens, setter, STANDARD, rect.width(), live.entries),
            }
        }
        Leaf::Fader(id) => {
            let bound = binding_for(id, p);
            bound.slider_vertical(
                ui,
                tokens,
                setter,
                live.entries,
                bound.painted(),
                rect.width(),
                mxm_ui::control::FADER_HEIGHT,
            );
        }
        // Each cell as wide as the longest option and no wider (design system §7.3).
        Leaf::Switch(id) => {
            let bound = binding_for(id, p);
            let labels = switch_options(p, id);
            let options: Vec<&str> = labels.iter().map(String::as_str).collect();
            binding::segmented_named(
                ui,
                tokens,
                bound.id,
                bound.param,
                bound.panel.as_deref(),
                &options,
                bound.details,
                setter,
                0.0,
            );
        }
        Leaf::Toggle(id) => {
            let bound = binding_for(id, p);
            binding::toggle_labelled(
                ui,
                tokens,
                bound.id,
                bound.param,
                bound.painted(),
                bound.description,
                setter,
                0.0,
            );
        }
        Leaf::Sync(id) => binding::sync_picture(ui, tokens, id, param_of(id, p), setter),
        Leaf::Shape(id) => {
            let bound = binding_for(id, p);
            binding::segmented_waves_named(
                ui,
                tokens,
                bound.id,
                bound.param,
                bound.panel.as_deref(),
                &LFO_SHAPES,
                None,
                bound.details,
                setter,
            );
        }
        Leaf::Model => model_picker(ui, tokens, p, setter),
        Leaf::Routes(target) => route_rows::stack(
            ui,
            tokens,
            &p.routes,
            Target::ALL[target],
            setter,
            live.entries,
        ),
    }
}

/// The engine picker: the twenty-four engines by name, **grouped by bank** in the caret selector's
/// menu (`mxm_ui::control::selector_grouped`, as the drum machine's Model is), one bracketed write
/// a choice. Its hover says what the engine chosen sounds like.
fn model_picker(ui: &mut Ui, tokens: &Tokens, p: &ThanksEgParams, setter: &ParamSetter<'_>) {
    let param: &dyn ErasedParam = &p.model;
    let last = (ENGINE_NAMES.len() - 1) as f32;
    let mut selected = (param.normalised() * last).round() as usize;
    let sounding = binding::sounding_cell(param, last, selected);
    let default = (binding::reset_target(param) * last).round() as usize;
    let groups: Vec<&str> = (0..ENGINE_NAMES.len()).map(|i| BANKS[i / 8]).collect();
    let description = format!(
        "Chooses the engine, the way the voice makes its sound. {}",
        engine_sound(selected)
    );
    let changed = mxm_ui::navigation::at(ui, "model", |ui| {
        mxm_ui::control::selector_grouped(
            ui,
            tokens,
            MODEL_LABEL,
            &ENGINE_NAMES,
            &groups,
            &mut selected,
            sounding,
            Some(default),
            &description,
        )
    });
    if changed {
        binding::set_together(setter, &[(param, selected as f32 / last)]);
    }
}

/// What each engine sounds like, in a line: the engine picker's hover text, written for the player
/// (design system §7.6).
pub fn engine_sound(engine: usize) -> &'static str {
    match engine {
        0 => "Classic oscillator waves through a resonant filter.",
        1 => "Bright, vocal tones made by bending the shape of a wave.",
        2..=4 => "Rich FM tones from a bank of stored six-operator voices.",
        5 => "A wave traced across a shifting landscape, from smooth to gritty.",
        6 => "A chord of ensemble strings and organ, softly chorused.",
        7 => "Square-wave chords and arpeggios with the bright buzz of early home computers.",
        8 => "Two classic oscillator waves, detuned or locked together.",
        9 => "A triangle folded and shaped into bright, hollow tones.",
        10 => "Two-operator FM: one wave bending the pitch of another, with feedback.",
        11 => "Vocal and formant tones made from short grains of sound.",
        12 => "Organ-like tones built from a row of harmonics.",
        13 => "Waves swept through a table of shapes.",
        14 => "Four-note chords from a choice of chord shapes.",
        15 => "Vowels, formants and spoken words.",
        16 => "A cloud of tiny grains, from a shimmer to a swarm.",
        17 => "Noise through a resonant filter.",
        18 => "Scattered clicks and drops of sound through resonant filters.",
        19 => "A plucked or bowed string with a metallic, bell-like edge.",
        20 => "Struck bars, bells and plates.",
        21 => "A deep, punchy kick drum.",
        22 => "A snare drum, from tight to rattling.",
        _ => "A hi-hat, from closed to open.",
    }
}

/// The LFO whose Rate `id` is, from 0.
fn lfo_of_rate(id: &str) -> Option<usize> {
    LFO_IDS.iter().position(|ids| ids[0] == id)
}

/// A synced Rate's reading, the division it plays, while its sync is on and a tempo is in force.
fn synced_reading(p: &ThanksEgParams, id: &str, tempo: Option<f64>) -> Option<&'static str> {
    use nice_plug::prelude::Param as _;
    let lfo = p.lfos()[lfo_of_rate(id)?];
    if !lfo.sync.value() {
        return None;
    }
    LFO_SYNC
        .shown(
            lfo.rate.unmodulated_normalized_value(),
            tempo,
            f64::from(lfo.rate.preview_plain(0.0)),
            f64::from(lfo.rate.preview_plain(1.0)),
        )
        .map(mxm_tempo::Division::label)
}

// ---------------------------------------------------------------------------------------------
// The bindings: every parameter's control, sentence, painted name and step law.
// ---------------------------------------------------------------------------------------------

/// Where an id is among the parameters.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Place {
    Voice,
    /// Envelope `n`'s `k`th, in `ENVELOPE_IDS`' order.
    Envelope(usize, usize),
    /// LFO `n`'s `k`th, in `LFO_IDS`' order.
    Lfo(usize, usize),
    /// Route slot `r`'s `k`th: source, target, amount.
    Route(usize, usize),
}

fn place(id: &str) -> Place {
    let find = |lists: &[&[&str]]| {
        lists
            .iter()
            .enumerate()
            .find_map(|(n, ids)| ids.iter().position(|i| *i == id).map(|k| (n, k)))
    };
    let envelopes: Vec<&[&str]> = ENVELOPE_IDS.iter().map(|ids| &ids[..]).collect();
    let lfos: Vec<&[&str]> = LFO_IDS.iter().map(|ids| &ids[..]).collect();
    let routes: Vec<&[&str]> = ROUTE_IDS.iter().map(|ids| &ids[..]).collect();
    if VOICE_IDS.contains(&id) {
        Place::Voice
    } else if let Some((n, k)) = find(&envelopes) {
        Place::Envelope(n, k)
    } else if let Some((n, k)) = find(&lfos) {
        Place::Lfo(n, k)
    } else if let Some((r, k)) = find(&routes) {
        Place::Route(r, k)
    } else {
        unreachable!("{id} is not a parameter")
    }
}

/// The parameter `id` names.
pub fn param_of<'a>(id: &str, p: &'a ThanksEgParams) -> &'a dyn ErasedParam {
    match place(id) {
        Place::Voice => match id {
            "model" => &p.model,
            "frequency" => &p.frequency,
            "fine" => &p.fine,
            "harmonics" => &p.harmonics,
            "timbre" => &p.timbre,
            "morph" => &p.morph,
            "fm_amount" => &p.fm_amount,
            "timbre_amount" => &p.timbre_amount,
            "morph_amount" => &p.morph_amount,
            "decay" => &p.decay,
            "colour" => &p.colour,
            "aux" => &p.aux,
            "spread" => &p.spread,
            "mode" => &p.mode,
            "glide" => &p.glide,
            "bend_range" => &p.bend_range,
            _ => &p.volume,
        },
        Place::Envelope(n, k) => {
            let e = p.envelopes()[n];
            let params: [&dyn ErasedParam; 7] = [
                &e.attack,
                &e.decay,
                &e.sustain,
                &e.release,
                &e.response,
                &e.cycle,
                &e.mode,
            ];
            params[k]
        }
        Place::Lfo(n, k) => {
            let l = p.lfos()[n];
            let params: [&dyn ErasedParam; 5] = [&l.rate, &l.shape, &l.sync, &l.retrigger, &l.fade];
            params[k]
        }
        Place::Route(r, k) => {
            let route = p.routes.all()[r];
            let params: [&dyn ErasedParam; 3] = [&route.source, &route.target, &route.amount];
            params[k]
        }
    }
}

/// What a control tells the player on hover: what it does to the sound (design system §7.6).
fn description_of(id: &str) -> &'static str {
    match place(id) {
        Place::Voice => match id {
            "model" => "Chooses the engine, the way the voice makes its sound.",
            "frequency" => "Tunes the engine in semitones, up to four octaves either way.",
            "fine" => "Fine tuning, up to a semitone either way.",
            "harmonics" => {
                "The engine's first sound control: what it shapes depends on the engine."
            }
            "timbre" => "The engine's second sound control, most often its brightness or texture.",
            "morph" => "The engine's third sound control, most often a blend or a change of shape.",
            "fm_amount" => {
                "How far each note's decay sweeps the pitch: up, or below the centre, down."
            }
            "timbre_amount" => {
                "How far each note's decay sweeps Timbre: up, or below the centre, down."
            }
            "morph_amount" => {
                "How far each note's decay sweeps Morph: up, or below the centre, down."
            }
            "decay" => "How long each note takes to fade away through the gate.",
            "colour" => {
                "How the gate closes: from a plain fade to a filter that darkens the note as it fades."
            }
            "aux" => "Blends the main sound into the engine's second sound, a variation of it.",
            "spread" => "Moves the main sound to the left and the second sound to the right.",
            "mode" => "How the voices take notes.",
            "glide" => "Glide time between notes; zero turns glide off.",
            "bend_range" => "The pitch-bend range, in semitones.",
            _ => "The instrument's output level.",
        },
        Place::Envelope(_, k) => match k {
            0 => "Time to rise to full level.",
            1 => "Time to fall from full level to the sustain level.",
            2 => "The level held for as long as the note is held.",
            3 => "Time to fall back to rest once the note ends.",
            4 => {
                "The curve of every stage: logarithmic below the centre, straight at it, exponential above."
            }
            5 => {
                "Repeats the rise and fall while it is on, so the envelope runs as a wave of its own."
            }
            _ => "What starts the envelope.",
        },
        Place::Lfo(_, k) => match k {
            0 => "How fast this LFO runs; with Sync on, a note length.",
            1 => "This LFO's wave.",
            2 => binding::SYNC_DESCRIPTION,
            3 => {
                "Starts this LFO again with each note, for each voice apart; off, one LFO runs for every voice."
            }
            _ => "How long this LFO takes to fade in after each note; zero is at once.",
        },
        Place::Route(_, k) => match k {
            0 => "Where this route's movement comes from.",
            1 => "What this route moves.",
            _ => "How far this route moves its target. Below the centre it moves it the other way.",
        },
    }
}

/// What each option of a stepped control does, one sentence per cell in the parameter's own order
/// (design system §7.3). Empty for everything drawn as a knob, fader, slider or toggle.
fn details_of(id: &str) -> &'static [&'static str] {
    match place(id) {
        Place::Voice if id == "mode" => &[
            "Up to eight notes at once.",
            "One note at a time; each new note starts the sound again.",
            "One note at a time; a note played over a held one carries on without starting again.",
        ],
        Place::Envelope(_, 6) => &[
            "Each note starts it, and letting go of the key releases it.",
            "Its Trigger starts it; it holds while the trigger is high and releases when it falls.",
            "It follows its Input, rising at the attack time and falling at the release time.",
        ],
        Place::Lfo(_, 1) => &LFO_DETAILS,
        _ => &[],
    }
}

/// What a control paints where its card already says the rest (design system §7.1); `None` paints
/// the parameter's own name.
fn panel_of(id: &str) -> Option<&'static str> {
    match place(id) {
        Place::Voice => match id {
            "mode" => Some("Mode"),
            "fm_amount" => Some("Pitch"),
            "timbre_amount" => Some("Timbre"),
            "morph_amount" => Some("Morph"),
            "decay" => Some("Decay"),
            _ => None,
        },
        Place::Envelope(_, k) => Some(["A", "D", "S", "R", "Response", "Cycle", "Mode"][k]),
        Place::Lfo(_, k) => Some(["Rate", "Shape", "Sync", "Retrigger", "Fade"][k]),
        Place::Route(..) => None,
    }
}

/// How the keyboard steps it (the owner's scope, 2026-09-23): the semitone-stepped Frequency and
/// Bend range by `Semitones`; everything else its own step.
fn law_of(id: &str) -> StepLaw {
    match id {
        "frequency" | "bend_range" => StepLaw::Semitones,
        _ => StepLaw::Own,
    }
}

/// Whether its rest is the centre: a signed amount or a tune.
fn bipolar(id: &str) -> bool {
    matches!(
        id,
        "frequency" | "fine" | "fm_amount" | "timbre_amount" | "morph_amount"
    ) || matches!(place(id), Place::Envelope(_, 4))
}

/// **Every parameter's binding**: its control, its sentence, what it paints, its step law.
pub fn binding_for<'a>(id: &'static str, p: &'a ThanksEgParams) -> Bound<'a> {
    let mut bound = Bound::new(id, param_of(id, p), description_of(id))
        .details(details_of(id))
        .law(law_of(id));
    if let Some(panel) = panel_of(id) {
        bound = bound.labelled(panel);
    }
    if bipolar(id) { bound.bipolar() } else { bound }
}

/// Every parameter's binding, in declaration order: the developer Parameters list.
pub fn all_bindings(p: &ThanksEgParams) -> Vec<Bound<'_>> {
    p.all_parameters()
        .into_iter()
        .map(|(id, _)| binding_for(id, p))
        .collect()
}
