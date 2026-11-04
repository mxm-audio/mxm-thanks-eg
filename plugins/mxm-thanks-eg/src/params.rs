//! T2's bare parameters: the module's own controls, and two of the instrument's. Provisional: the
//! instrument's 101 come in T3 and are fixed at the gate (ops' `plans/plan-mxm-thanks-eg.md` §5.5).

use nice_plug::prelude::*;
use std::sync::Arc;

/// The engines, in Plaits' registration order (its `dsp/voice.cc`): the instrument's own names.
pub const ENGINE_NAMES: [&str; 24] = [
    "Analogue with filter",
    "Phase distortion",
    "Six-op FM 1",
    "Six-op FM 2",
    "Six-op FM 3",
    "Wave terrain",
    "String machine",
    "Chiptune",
    "Virtual analogue",
    "Waveshaping",
    "Two-op FM",
    "Grain and formant",
    "Harmonic",
    "Wavetable",
    "Chords",
    "Speech",
    "Granular cloud",
    "Filtered noise",
    "Particle noise",
    "Inharmonic string",
    "Modal resonator",
    "Bass drum",
    "Snare drum",
    "Hi-hat",
];

#[derive(Params)]
pub struct ThanksEgParams {
    #[id = "model"]
    pub model: IntParam,
    #[id = "frequency"]
    pub frequency: FloatParam,
    #[id = "fine"]
    pub fine: FloatParam,
    #[id = "harmonics"]
    pub harmonics: FloatParam,
    #[id = "timbre"]
    pub timbre: FloatParam,
    #[id = "morph"]
    pub morph: FloatParam,
    /// The module's three attenuverters: how far the decay envelope moves each control.
    #[id = "fm"]
    pub fm: FloatParam,
    #[id = "timbre_mod"]
    pub timbre_mod: FloatParam,
    #[id = "morph_mod"]
    pub morph_mod: FloatParam,
    #[id = "decay"]
    pub decay: FloatParam,
    #[id = "colour"]
    pub colour: FloatParam,
    #[id = "aux"]
    pub aux: FloatParam,
    #[id = "volume"]
    pub volume: FloatParam,
}

fn unit(name: &str, default: f32) -> FloatParam {
    FloatParam::new(name, default, FloatRange::Linear { min: 0.0, max: 1.0 })
        .with_value_to_string(formatters::v2s_f32_percentage(0))
        .with_string_to_value(formatters::s2v_f32_percentage())
        .with_unit("%")
}

/// A parameter's text, and the value read back from text, as nice-plug takes them.
type Reading = Arc<dyn Fn(f32) -> String + Send + Sync>;
type Parsing = Arc<dyn Fn(&str) -> Option<f32> + Send + Sync>;

/// Semitones, `decimals` places, never `-0`: a reading that does not round-trip fails
/// `clap-validator`'s conversions.
fn semitones(decimals: usize) -> Reading {
    Arc::new(move |v| {
        let scale = 10f32.powi(decimals as i32);
        // Adding zero turns a rounded -0 into 0.
        format!("{:.*}", decimals, (v * scale).round() / scale + 0.0)
    })
}

fn parse_semitones() -> Parsing {
    Arc::new(|s| s.trim().trim_end_matches("st").trim().parse::<f32>().ok())
}

fn amount(name: &str) -> FloatParam {
    FloatParam::new(
        name,
        0.0,
        FloatRange::Linear {
            min: -1.0,
            max: 1.0,
        },
    )
    .with_value_to_string(formatters::v2s_f32_percentage(0))
    .with_string_to_value(formatters::s2v_f32_percentage())
    .with_unit("%")
}

impl Default for ThanksEgParams {
    fn default() -> Self {
        Self {
            model: IntParam::new("Model", 8, IntRange::Linear { min: 0, max: 23 })
                .with_value_to_string(Arc::new(|v| {
                    ENGINE_NAMES[v.clamp(0, 23) as usize].to_owned()
                }))
                // An engine's name, or its number.
                .with_string_to_value(Arc::new(|s| {
                    let s = s.trim();
                    ENGINE_NAMES
                        .iter()
                        .position(|name| name.eq_ignore_ascii_case(s))
                        .map(|i| i as i32)
                        .or_else(|| s.parse().ok())
                })),
            frequency: FloatParam::new(
                "Frequency",
                0.0,
                FloatRange::Linear {
                    min: -48.0,
                    max: 48.0,
                },
            )
            .with_step_size(1.0)
            .with_unit(" st")
            .with_value_to_string(semitones(0))
            .with_string_to_value(parse_semitones()),
            fine: FloatParam::new(
                "Fine",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .with_step_size(0.01)
            .with_unit(" st")
            .with_value_to_string(semitones(2))
            .with_string_to_value(parse_semitones()),
            harmonics: unit("Harmonics", 0.5),
            timbre: unit("Timbre", 0.5),
            morph: unit("Morph", 0.5),
            fm: amount("Pitch from decay"),
            timbre_mod: amount("Timbre from decay"),
            morph_mod: amount("Morph from decay"),
            decay: unit("Decay", 0.5),
            colour: unit("Colour", 0.5),
            aux: unit("Aux", 0.0),
            volume: FloatParam::new("Volume", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage())
                .with_unit("%"),
        }
    }
}
