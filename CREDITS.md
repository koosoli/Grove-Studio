# Credits and provenance

Grove Studio’s interface was created for this project. Its trees are Solstice’s procedural species, plus the Poseidon oak described below. Bark and leaf images come from Solstice’s procedural bake, so the application bundles no photographs.

## Poseidon oak

`species/poseidon-oak.species` re-expresses the forest oak preset of the Poseidon engine (`content/default-packs/forest-trees/source/presets/forest_oak.json`) in Solstice’s species format. That preset uses ez-tree’s growth model by Dan Greenheck, MIT licensed, https://github.com/dgreenheck/ez-tree. Only the preset’s proportions were carried over. No engine code, textures or models are included.

## Solstice

Grove Studio uses SamG-Coder’s MIT-licensed Solstice asset, cluster builder, procedural species and renderer code, pinned at `c347879d8f6b97a850db004ecfd6a9c5cf29768b`. The renderer copy includes Grove’s parallax and wind adaptations. See [Solstice](https://github.com/SamG-Coder/solstice) and [the vendored MIT license](vendor/solstice-render/LICENSE).

## Dependencies

Rust dependencies are managed by Cargo.lock. macroquad is licensed under MIT OR Apache-2.0; miniquad under MIT OR Apache-2.0; glam under MIT OR Apache-2.0; serde and serde_json under MIT OR Apache-2.0. See each dependency’s distributed license for its full terms. Macroquad’s included ProggyClean font is its fallback font. Segoe UI is loaded from the user’s own Windows installation, and is not redistributed with this project.

Full dependency license and copyright notices are included in THIRD-PARTY-NOTICES.txt.
