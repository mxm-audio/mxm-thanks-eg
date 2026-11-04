//! The instrument's side of the collection's preset system (`mxm-preset`): its id, version,
//! parameters and identity slot. Init is generated from the parameter defaults and has no file;
//! the factory set is the files in this crate's `presets/`, in `order.txt`'s order, which
//! `cargo xtask bundle` checks and installs for the plugin's version, where the plugin reads them.
//!
//! The factory sounds are designed in the test module's [`tests::design`], as overrides on Init in
//! each parameter's own units, and written to `presets/` by the ignored
//! `write_the_factory_presets`; `the_shipped_files_are_the_design` catches a forgotten regenerate.

use crate::params::ThanksEgParams;
use std::sync::RwLock;

pub use mxm_preset::{Category, INIT_NAME, Preset, PresetIdentity, Value};

impl mxm_preset::Instrument for ThanksEgParams {
    fn clap_id(&self) -> &'static str {
        crate::CLAP_ID
    }

    /// The plugin's version, its `Plugin::VERSION`: its factory presets are installed for it.
    fn version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    fn parameters(&self) -> Vec<(&'static str, &dyn mxm_params::ErasedParam)> {
        self.all_parameters()
    }

    fn identity(&self) -> &RwLock<PresetIdentity> {
        &self.preset
    }

    /// The installed set, but in a unit test the repository's, never the machine's.
    #[cfg(test)]
    fn factory_files(&self) -> Vec<(String, String)> {
        mxm_preset::read_factory_dir(&factory_folder())
    }
}

/// This crate's `presets/`: what the bundle installs, and what the tests read.
#[must_use]
pub fn factory_folder() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("presets")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::{BANKS, INIT_MODEL};
    use crate::routes::{SourceChoice, TargetChoice};
    use mxm_preset::Instrument as _;
    use mxm_thanks_eg_dsp::{Source, Target};
    use nice_plug::prelude::{Enum as _, ParamSetter, Params as _};

    /// One factory sound: its name, its category, and what it changes from Init, each value in its
    /// parameter's own units (seconds, hertz, a fraction, an engine's or a list's index).
    type Design = (&'static str, Category, Vec<(&'static str, f32)>);

    fn engine(index: i32) -> f32 {
        index as f32
    }

    fn source(source: Source) -> f32 {
        SourceChoice(Some(source)).to_index() as f32
    }

    fn target(target: Target) -> f32 {
        TargetChoice(Some(target)).to_index() as f32
    }

    /// **The factory sounds, readable.** First drafts, at least one per bank of engines; the set
    /// of fifty and its listening are owed (T4).
    pub(super) fn design() -> Vec<Design> {
        vec![
            // Bank 1.
            (
                "Filter bass",
                Category::Bass,
                vec![
                    ("model", engine(0)),
                    ("frequency", -12.0),
                    ("harmonics", 0.3),
                    ("timbre", 0.35),
                    ("morph", 0.2),
                    ("timbre_amount", 0.4),
                    ("decay", 0.4),
                    ("mode", 1.0),
                ],
            ),
            (
                "Bright keys",
                Category::Keys,
                vec![
                    ("model", engine(1)),
                    ("harmonics", 0.35),
                    ("timbre", 0.65),
                    ("morph", 0.3),
                    ("decay", 0.55),
                    ("colour", 0.6),
                ],
            ),
            (
                "String ensemble",
                Category::Strings,
                vec![
                    ("model", engine(6)),
                    ("harmonics", 0.5),
                    ("timbre", 0.6),
                    ("env1_attack", 0.6),
                    ("env1_sustain", 1.0),
                    ("env1_release", 1.2),
                    ("route1_source", source(Source::Envelope1)),
                    ("route1_target", target(Target::Level)),
                    ("route1_amount", 1.0),
                    ("route2_source", source(Source::Lfo2)),
                    ("route2_target", target(Target::Morph)),
                    ("route2_amount", 0.1),
                    ("spread", 0.4),
                ],
            ),
            // Bank 2.
            (
                "Soft saw pad",
                Category::Pad,
                vec![
                    ("model", engine(8)),
                    ("harmonics", 0.45),
                    ("timbre", 0.4),
                    ("env1_attack", 0.4),
                    ("env1_sustain", 0.9),
                    ("env1_release", 1.0),
                    ("route1_source", source(Source::Envelope1)),
                    ("route1_target", target(Target::Level)),
                    ("route1_amount", 1.0),
                    ("route2_source", source(Source::Lfo2)),
                    ("route2_target", target(Target::Timbre)),
                    ("route2_amount", 0.15),
                    ("spread", 0.5),
                ],
            ),
            (
                "Chord stabs",
                Category::Keys,
                vec![
                    ("model", engine(14)),
                    ("harmonics", 0.4),
                    ("timbre", 0.5),
                    ("morph", 0.4),
                    ("decay", 0.35),
                ],
            ),
            (
                "Legato lead",
                Category::Lead,
                vec![
                    ("model", engine(9)),
                    ("mode", 2.0),
                    ("glide", 0.08),
                    ("timbre", 0.55),
                    ("morph", 0.35),
                    ("env1_sustain", 1.0),
                    ("env1_release", 0.15),
                    ("route1_source", source(Source::Envelope1)),
                    ("route1_target", target(Target::Level)),
                    ("route1_amount", 1.0),
                    ("route2_source", source(Source::Lfo1)),
                    ("route2_target", target(Target::Pitch)),
                    ("route2_amount", 0.025),
                ],
            ),
            // Bank 3.
            (
                "Struck bar",
                Category::Percussion,
                vec![
                    ("model", engine(20)),
                    ("harmonics", 0.6),
                    ("timbre", 0.6),
                    ("morph", 0.4),
                    ("decay", 0.7),
                ],
            ),
            (
                "Deep kick",
                Category::Percussion,
                vec![
                    ("model", engine(21)),
                    ("harmonics", 0.4),
                    ("timbre", 0.3),
                    ("morph", 0.6),
                ],
            ),
            (
                "Noise wind",
                Category::Fx,
                vec![
                    ("model", engine(17)),
                    ("harmonics", 0.5),
                    ("timbre", 0.4),
                    ("morph", 0.7),
                    ("env1_attack", 1.5),
                    ("env1_sustain", 1.0),
                    ("env1_release", 2.0),
                    ("route1_source", source(Source::Envelope1)),
                    ("route1_target", target(Target::Level)),
                    ("route1_amount", 1.0),
                    ("route2_source", source(Source::Lfo2)),
                    ("route2_target", target(Target::Timbre)),
                    ("route2_amount", 0.3),
                    ("lfo2_rate", 0.3),
                ],
            ),
        ]
    }

    /// The file a sound is written to.
    fn file_name(name: &str) -> String {
        format!("{}.json", name.to_lowercase().replace(' ', "-"))
    }

    /// A design as the preset it is: Init, with each override converted to its parameter's
    /// normalised value and every text the plugin's own, read in the preset's state.
    fn generated(params: &ThanksEgParams, (name, category, overrides): &Design) -> Preset {
        let map = params.param_map();
        let mut preset = Preset::init(params);
        preset.name = (*name).to_owned();
        preset.category = *category;
        for (id, plain) in overrides {
            let (_, ptr, _) = map
                .iter()
                .find(|(candidate, ..)| candidate == id)
                .unwrap_or_else(|| panic!("{name:?} names unknown parameter `{id}`"));
            // SAFETY: `params` owns the parameter and outlives the call.
            let normalised = unsafe { ptr.preview_normalized(*plain) };
            preset
                .params
                .get_mut(*id)
                .unwrap_or_else(|| panic!("{id} is not in Init"))
                .v = normalised;
        }
        mxm_preset::refresh_texts(params, &mut preset);
        preset
    }

    fn shipped() -> Vec<(String, String)> {
        ThanksEgParams::default().factory_files()
    }

    #[test]
    #[ignore = "writes the factory preset files"]
    fn write_the_factory_presets() {
        let params = ThanksEgParams::default();
        let folder = factory_folder();
        std::fs::create_dir_all(&folder).unwrap();
        let mut order = String::new();
        for design in design() {
            let file = file_name(design.0);
            std::fs::write(folder.join(&file), generated(&params, &design).to_json()).unwrap();
            order.push_str(&file);
            order.push('\n');
        }
        std::fs::write(folder.join(mxm_preset::ORDER_FILE), order).unwrap();
    }

    /// **The shipped files are the design**, in its order: a design changed without regenerating
    /// fails here.
    #[test]
    fn the_shipped_files_are_the_design() {
        let params = ThanksEgParams::default();
        let files = shipped();
        let designs = design();
        assert_eq!(
            files
                .iter()
                .map(|(file, _)| file.clone())
                .collect::<Vec<_>>(),
            designs.iter().map(|d| file_name(d.0)).collect::<Vec<_>>(),
            "regenerate: cargo test -p mxm-thanks-eg --lib write_the_factory_presets -- --ignored"
        );
        for ((file, json), design) in files.iter().zip(&designs) {
            assert_eq!(
                Preset::parse(json, crate::CLAP_ID).unwrap(),
                generated(&params, design),
                "regenerate {file}"
            );
        }
    }

    /// **Every stored `text` is this plugin's own formatting of the `v` beside it**
    /// (`mxm_preset::stale_factory_texts`).
    #[test]
    fn every_factory_text_is_this_plugins_formatting_of_its_value() {
        let stale = mxm_preset::stale_factory_texts(&ThanksEgParams::default());
        assert!(stale.is_empty(), "{}", stale.join("\n"));
    }

    /// Every file is this plugin's, names every parameter, is categorised and is a sound of its
    /// own.
    #[test]
    fn every_file_is_complete_distinct_categorised_and_owned() {
        let params = ThanksEgParams::default();
        let mut seen = Vec::new();
        assert!(!shipped().is_empty(), "no factory files were read");
        for (file, json) in shipped() {
            let preset =
                Preset::parse(&json, crate::CLAP_ID).unwrap_or_else(|e| panic!("{file}: {e}"));
            assert_ne!(preset.category, Category::Uncategorised, "{file}");
            let (_, problems) = preset.resolve(&params);
            assert!(problems.is_empty(), "{file}: {problems:?}");
            assert_eq!(preset.params.len(), params.all_parameters().len(), "{file}");
            assert!(!seen.contains(&preset.params), "{file} repeats a sound");
            seen.push(preset.params);
        }
    }

    /// **Every file loads as a host loads it** (`mxm_preset::clap::load_file`, the CLAP preset
    /// loader's path), and marks itself loaded by its name.
    #[test]
    fn every_file_loads_through_the_hosts_loader() {
        let params = ThanksEgParams::default();
        let host = mxm_plugin_test::NoHost;
        let setter = ParamSetter::new(&host);
        for (file, json) in shipped() {
            let path = factory_folder().join(&file);
            assert_eq!(
                mxm_preset::clap::load_file(&params, &path, &setter),
                Ok(()),
                "{file}"
            );
            let name = Preset::parse(&json, crate::CLAP_ID).unwrap().name;
            assert_eq!(mxm_preset::loaded(&params).name(), Some(name.as_str()));
        }
    }

    /// `order.txt` names every file once, and nothing else.
    #[test]
    fn the_order_names_every_file_once() {
        let folder = factory_folder();
        let order = std::fs::read_to_string(folder.join(mxm_preset::ORDER_FILE)).unwrap();
        let mut listed: Vec<String> = order.lines().map(str::to_owned).collect();
        let mut files: Vec<String> = std::fs::read_dir(&folder)
            .unwrap()
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|name| name.ends_with(".json"))
            .collect();
        listed.sort();
        files.sort();
        assert_eq!(listed, files);
    }

    /// **Every bank of engines has a sound**: the browser opens on each part of the instrument.
    #[test]
    fn every_bank_has_a_sound() {
        let mut banks = [false; BANKS];
        for (_, _, overrides) in design() {
            let model = overrides
                .iter()
                .find(|(id, _)| *id == "model")
                .map_or(INIT_MODEL, |(_, v)| *v as i32);
            banks[model as usize / 8] = true;
        }
        assert_eq!(banks, [true; BANKS]);
    }

    /// **Init is every parameter's default, and the browser's first row**: the generated Init and
    /// the CLAP defaults are one set of numbers.
    #[test]
    fn init_is_every_parameter_default_and_factory_begins_with_it() {
        let params = ThanksEgParams::default();
        let init = Preset::init(&params);
        assert_eq!(init.params.len(), params.all_parameters().len());
        for (id, parameter) in params.all_parameters() {
            assert_eq!(init.params[id].v, parameter.default_normalised(), "{id}");
        }
        let all = mxm_preset::factory(&params);
        assert_eq!(all[0].name, INIT_NAME);
        assert_eq!(all.len(), 1 + shipped().len());
    }

    #[test]
    fn factory_content_does_not_move_volume() {
        let params = ThanksEgParams::default();
        let default = params.volume.default_normalised();
        for (file, json) in shipped() {
            let preset = Preset::parse(&json, crate::CLAP_ID).unwrap();
            assert_eq!(preset.params["volume"].v, default, "{file}");
        }
    }

    #[test]
    fn user_files_are_namespaced_to_the_permanent_id() {
        if let Some(root) = mxm_preset::user_root(crate::CLAP_ID) {
            assert!(root.to_string_lossy().contains(crate::CLAP_ID));
        }
    }

    use mxm_params::ErasedParam as _;
}
