//! The files of Bohemia's sample mods, in the server program. `cargo xtask import --samples`
//! copies them to `src/game-files/samples/`, and `build.rs` lists them.

/// Every file of the sample mods, as (`<sample mod folder>/<path in the mod>`, file bytes), sorted
/// by path. Example path: `SampleMod_NewWeapon/Prefabs/Weapons/Rifles/SampleWeapon_01.et`.
pub(super) const SAMPLE_FILES: &[(&str, &[u8])] =
    include!(concat!(env!("OUT_DIR"), "/sample_files.rs"));
