//! The tree library: Solstice's procedural species, plus the Poseidon engine's forest oak grown
//! with the same Solstice growth model.
use crate::tree::{self, Part, Settings, Tree, V};
use image::{ImageBuffer, ImageOutputFormat};
use macroquad::prelude::{vec2, vec3, Vec3};
use solstice_procgen::foliage::{leaf::LeafKind, tree::{Crown, Species}};
use std::{io::Cursor, sync::OnceLock};

pub use crate::catalog::IDS;

/// A species text that replaces one library species for this process (headless
/// `--species-file`): (species id, text). The library id still names the bark cache and palette.
static OVERRIDE: OnceLock<(String, String)> = OnceLock::new();

/// Grows `id` from `text` instead of its library definition. Fails if the text does not parse
/// or an override is already set.
pub fn override_species(id: &str, text: String) -> Result<(), String> {
    Species::parse(&text).map_err(|e| format!("species file: {e}"))?;
    OVERRIDE
        .set((id.to_string(), text))
        .map_err(|_| "only one species file per run".to_string())
}

fn plant(name: &str) -> Option<Species> {
    let id = format!("solstice-{name}");
    if let Some((over, text)) = OVERRIDE.get() {
        if *over == id {
            return Species::parse(text).ok();
        }
    }
    match crate::catalog::CUSTOM.iter().find(|entry| entry.0 == id) {
        Some((id, _, _, text)) => Some(Species::parse(text).unwrap_or_else(|e| panic!("{id}: {e}"))),
        None => Species::by_name(name),
    }
}

/// Whether a species is an evergreen with needles (its leaves are kept through winter).
pub fn evergreen_id(id: &str) -> bool {
    id.strip_prefix("solstice-")
        .and_then(plant)
        .is_some_and(|p| p.leaves.kind == LeafKind::Needles)
}

/// The kind of leaf a species grows.
pub fn leaf_kind(s: &Settings) -> Option<LeafKind> {
    species(s).map(|p| p.leaves.kind)
}

pub fn evergreen(s: &Settings) -> bool {
    evergreen_id(&s.species)
}

/// Colour palette a species' leaves use: pine green, birch yellow, or broadleaf.
pub fn palette(s: &Settings) -> &'static str {
    match species(s).map(|p| p.leaves.kind) {
        Some(LeafKind::Needles) => "pine",
        Some(LeafKind::Birch) => "birch",
        _ => "broadleaf",
    }
}

fn species(settings: &Settings) -> Option<Species> {
    plant(settings.species.strip_prefix("solstice-")?)
}

pub fn settings(id: &str) -> Option<Settings> {
    let plant = plant(id.strip_prefix("solstice-")?)?;
    Some(Settings {
        species: id.into(),
        seed: 0,
        height: plant.height,
        trunk_radius: plant.trunk_radius,
        leaf_size: (plant.leaves.length.0 + plant.leaves.length.1) * 0.5,
        ..Settings::default()
    })
}

pub fn grow(s: &Settings, lod: u32) -> Option<Tree> {
    let mut species = species(s)?;
    let original_leaf_size = (species.leaves.length.0 + species.leaves.length.1) * 0.5;
    let leaf_scale = s.leaf_size / original_leaf_size;
    // Grow younger stems at their actual length. Compressing only the Y axis after growing a
    // mature crown makes saplings unnaturally broad and also squashes their leaf geometry.
    let height_scale = s.height / species.height;
    species.height *= s.maturity;
    let juvenile = ((1. - s.maturity) / 0.6).clamp(0., 1.);
    let needle = species.leaves.kind == LeafKind::Needles;
    species.trunk_radius = s.trunk_radius * tree::age_radius_scale(s.maturity);
    species.root_flare *= 1. - 0.55 * juvenile;
    species.trunk_gnarl = species.trunk_gnarl * 2. * (1. - s.trunk_straightness)
        + 0.03 * (0.5 - s.trunk_straightness).max(0.);
    species.trunk_gnarl *= 1. - 0.55 * juvenile;
    species.levels[0].per_metre *= s.branches as f32 / Settings::default().branches as f32 * (0.7 + 0.3 * s.maturity);
    // Young conifer leaders carry relatively closely spaced whorls; retain their conical form.
    if needle { species.levels[0].per_metre *= 1. + 0.4 * juvenile; }
    if juvenile > 0. && !needle && s.species != "solstice-bush" {
        let old = species.crown.clone();
        let young = [0.58, 0.83, 1.0, 0.84, 0.58, 0.32, 0.12];
        species.crown = Crown::Profile(young.iter().enumerate().map(|(i, &target)| {
            let original = old.envelope(i as f32 / (young.len() - 1) as f32);
            original + (target - original) * juvenile * 0.75
        }).collect());
    }
    for (depth, level) in species.levels.iter_mut().enumerate() {
        level.angle.0 = (level.angle.0 * s.branch_angle).clamp(0., 180.);
        level.angle.1 = (level.angle.1 * s.branch_angle).clamp(0., 180.);
        level.droop = level.droop * s.branch_droop + 0.1 * (s.branch_droop - 1.).max(0.);
        level.droop *= 1. - 0.5 * juvenile;
        level.gnarl *= 1. - 0.35 * juvenile;
        if depth == 0 && s.species != "solstice-bush" {
            level.span.0 *= 1. - 0.4 * juvenile;
            level.length *= 1. - if needle { 0.08 } else { 0.20 } * juvenile;
            if !needle { level.angle.0 *= 1. - 0.18 * juvenile; level.angle.1 *= 1. - 0.12 * juvenile; }
        }
    }
    let deciduous = species.leaves.kind != LeafKind::Needles;
    let season_density = if deciduous { match s.season { 1 => 0.82, 2 => 0.80, _ => 1. } } else { 1. };
    let season_leaf = if deciduous && s.season == 2 { 0.82 } else { 1. };
    species.leaves.per_metre *= s.density * season_density;
    if !needle { species.leaves.per_metre *= 1. + 1.5 * juvenile; }
    species.leaves.length.0 *= leaf_scale * season_leaf;
    species.leaves.length.1 *= leaf_scale * season_leaf;
    let plant = species.grow_with_juvenile_foliage(s.seed, juvenile);
    let mut bark_branch = vec![0; plant.wood.vertices.len()];
    let mut original_leaf_branch = vec![0; plant.leaves.vertices.len()];
    for (i, branch) in plant.branches.iter().enumerate() {
        bark_branch[branch.wood_vertices.clone()].fill(i as u32);
        original_leaf_branch[branch.leaf_vertices.clone()].fill(i as u32);
    }
    let mut branches: Vec<tree::Branch> = plant.branches.iter().map(|branch| tree::Branch {
        parent: branch.parent,
        points: branch.points.iter().map(|p| [p.x * s.spread, p.y * height_scale, p.z * s.spread]).collect(),
        radii: branch.radii.iter().map(|r| r * s.spread).collect(),
    }).collect();
    let convert = |v: &solstice_builder::Vertex, c| V {
        p: vec3(
            v.position.x * s.spread,
            v.position.y * height_scale,
            v.position.z * s.spread,
        ),
        n: vec3(
            v.normal.x / s.spread,
            v.normal.y / height_scale,
            v.normal.z / s.spread,
        )
        .normalize(),
        uv: vec2(v.uv.x, v.uv.y),
        c,
        branch: 0,
    };
    let mut bark = Part {
        v: plant
            .wood
            .vertices
            .iter()
            .map(|v| convert(v, [1.; 4]))
            .collect(),
        i: plant.wood.indices,
    };
    for (vertex, &owner) in bark.v.iter_mut().zip(&bark_branch) { vertex.branch = owner; }
    bark.i = keep_area(&bark.i, |i| bark.v[i as usize].p);
    let (verts, indices) = if needle { (3, 3) } else { let n = species.leaves.kind.rows().len(); (2 + n * 3, n * 12) };
    let mut leaves = Part::default();
    let mut leaf_branch = Vec::new();
    let stride = [1, 2, 5][lod as usize];
    let winter = s.season == 3 && !needle;
    let colors: [[f32; 4]; 4] = std::array::from_fn(|variant| leaf_color(s, variant));
    for (i, (piece, triangles)) in plant
        .leaves
        .vertices
        .chunks_exact(verts)
        .zip(plant.leaves.indices.chunks_exact(indices))
        .enumerate()
    {
        if i % stride != 0 || (winter && (i + s.seed as usize % 29) % 29 != 0) {
            continue;
        }
        // Scale thinned leaves about the stalk, not the centre: otherwise their attachment
        // moves away from the twig as the LOD changes.
        let anchor = if needle { (piece[0].position + piece[1].position) * 0.5 } else { piece[0].position };
        // Spring flush: the leaves nearest a shoot tip are still expanding. This is an
        // illustrative developmental gradient, evaluated around each leaf's own stalk.
        let tip_scale = if s.season == 2 && !needle {
            let branch = &plant.branches[original_leaf_branch[i * verts] as usize];
            let first = branch.points[0];
            let last = *branch.points.last().unwrap();
            let from_base = anchor.distance(first);
            let from_tip = anchor.distance(last);
            1. - 0.18 * from_base / (from_base + from_tip).max(1e-5)
        } else { 1. };
        let grow = [1., 1.25, 1.63][lod as usize] * tip_scale;
        let offset = leaves.v.len() as u32;
        for v in piece {
            let variant = ((v.uv.x * 4.) as usize).min(3);
            let color = colors[variant];
            let mut vertex = convert(v, color);
            vertex.branch = original_leaf_branch[i * verts];
            let p = anchor + (v.position - anchor) * grow;
            vertex.p = vec3(p.x * s.spread, p.y * height_scale, p.z * s.spread);
            leaves.v.push(vertex);
            leaf_branch.push(original_leaf_branch[i * verts]);
        }
        leaves.i.extend(
            triangles
                .iter()
                .map(|index| index - (i * verts) as u32 + offset),
        );
    }
    if leaves.v.is_empty() && plant.leaves.vertices.len() >= verts {
        leaves.v.extend(
            plant.leaves.vertices[..verts]
                .iter()
                .map(|v| convert(v, leaf_color(s, 0))),
        );
        leaves.i.extend_from_slice(&plant.leaves.indices[..indices]);
        leaf_branch.extend_from_slice(&original_leaf_branch[..verts]);
        for (v, &owner) in leaves.v.iter_mut().zip(&leaf_branch) { v.branch = owner; }
    }
    leaves.i = keep_area(&leaves.i, |i| leaves.v[i as usize].p);
    let mut tree = Tree { bark, leaves, lod, branches: Vec::new(), bark_branch, leaf_branch };
    let base = tree.bark.v.iter().map(|v| v.p.y).fold(0., f32::min);
    for v in tree.bark.v.iter_mut().chain(&mut tree.leaves.v) {
        v.p -= Vec3::Y * base;
    }
    for branch in &mut branches {
        for point in &mut branch.points { point[1] -= base; }
    }
    tree.branches = branches;
    Some(tree)
}

/// Keeps the triangles that have area. Branch tips close to a point leave faces with none, which
/// add nothing to the picture or the export.
fn keep_area(indices: &[u32], position: impl Fn(u32) -> Vec3) -> Vec<u32> {
    indices
        .chunks_exact(3)
        .filter(|t| {
            let (a, b, c) = (position(t[0]), position(t[1]), position(t[2]));
            (b - a).cross(c - a).length_squared() > 1e-16
        })
        .flat_map(|t| t.iter().copied())
        .collect()
}

fn leaf_color(s: &Settings, variant: usize) -> [f32; 4] {
    if s.season != 0 {
        return tree::leaf_color(s, variant);
    }
    let rgba = leaf_atlas(s).unwrap();
    let i = (128 * 256 + (variant * 64 + 32)) * 4;
    let linear = |b: u8| ((b as f32 / 255. + 0.055) / 1.055).powf(2.4);
    [
        linear(rgba[i]),
        linear(rgba[i + 1]),
        linear(rgba[i + 2]),
        1.,
    ]
}

pub fn leaf_maps(s: &Settings) -> Option<&'static solstice_asset::TextureLayer> {
    static CACHE: [OnceLock<solstice_asset::TextureLayer>; 5] = [const { OnceLock::new() }; 5];
    let kind = species(s)?.leaves.kind;
    let i = LeafKind::ALL.iter().position(|k| *k == kind)?;
    Some(CACHE[i].get_or_init(|| kind.atlas(0).bake(256)))
}

pub fn leaf_atlas(s: &Settings) -> Option<&'static [u8]> {
    Some(&leaf_maps(s)?.color_mips[0])
}

pub struct BarkImages {
    pub color: Vec<u8>,
    pub normal: Vec<u8>,
    pub height: Vec<u8>,
}

pub fn bark_images(s: &Settings) -> Option<&'static BarkImages> {
    static CACHE: [OnceLock<BarkImages>; IDS.len()] = [const { OnceLock::new() }; IDS.len()];
    let i = IDS.iter().position(|id| *id == s.species)?;
    Some(CACHE[i].get_or_init(|| {
        const EDGE: u32 = 512;
        let surface = species(s).unwrap().bark.surface(0);
        let height = surface.height_map(EDGE);
        let mut layer = surface.bake(EDGE);
        let mut color = layer.color_mips.remove(0);
        // The upstream bake is deliberately subtle and can quantize to nearly flat grey at
        // tree scale. Retain its species colors while bringing out the measured relief in
        // the diffuse map. Custom imported albedos bypass this generated texture entirely.
        for (p, &h) in color.chunks_exact_mut(4).zip(&height) {
            let relief = h as f32 / 255.;
            let gain = 0.62 + 0.76 * relief;
            for channel in &mut p[..3] { *channel = (*channel as f32 * gain).round().clamp(0.,255.) as u8; }
            p[3] = 255;
        }
        let normal = layer
            .normal_mips
            .remove(0)
            .chunks_exact(2)
            .flat_map(|xy| {
                let [x, y] = [xy[0], xy[1]].map(|n| n as f32 / 127.5 - 1.);
                [
                    xy[0],
                    xy[1],
                    ((1. - x * x - y * y).max(0.).sqrt() * 127.5 + 127.5) as u8,
                ]
            })
            .collect();
        let encode = |image: image::DynamicImage| {
            let mut bytes = Cursor::new(Vec::new());
            image
                .write_to(&mut bytes, ImageOutputFormat::Png)
                .expect("encode bark");
            bytes.into_inner()
        };
        BarkImages {
            color: encode(image::DynamicImage::ImageRgba8(
                ImageBuffer::from_raw(EDGE, EDGE, color).unwrap(),
            )),
            normal: encode(image::DynamicImage::ImageRgb8(
                ImageBuffer::from_raw(EDGE, EDGE, normal).unwrap(),
            )),
            height: encode(image::DynamicImage::ImageLuma8(
                ImageBuffer::from_raw(EDGE, EDGE, height).unwrap(),
            )),
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn growth_stage_seasons_and_branch_ownership_are_consistent() {
        let s = settings("solstice-oak").unwrap();
        let mature = grow(&s, 0).unwrap();
        let mut young_settings = s.clone();
        young_settings.maturity = 0.5;
        let young = grow(&young_settings, 0).unwrap();
        assert!(young.bounds().1.y < mature.bounds().1.y * 0.7);
        let mut spring_settings = s.clone();
        spring_settings.season = 2;
        let spring = grow(&spring_settings, 0).unwrap();
        assert!(spring.leaves.v.len() < mature.leaves.v.len());
        assert_eq!(mature.bark_branch.len(), mature.bark.v.len());
        assert_eq!(mature.leaf_branch.len(), mature.leaves.v.len());
        assert_eq!(mature.branches[0].parent, None);
        for (i, branch) in mature.branches.iter().enumerate().skip(1) {
            assert!(branch.parent.unwrap() < i);
            assert!(branch.points.iter().flatten().all(|x| x.is_finite()));
        }
        assert!(mature.bark_branch.iter().chain(&mature.leaf_branch).all(|&id| (id as usize) < mature.branches.len()));
        assert!(mature.leaf_branch.iter().any(|&id| id > 0));
    }

    #[test]
    fn juvenile_trees_grow_smaller_stems_without_flattening_leaves() {
        for id in ["solstice-oak", "solstice-birch", "solstice-fir", "solstice-cwa-buk"] {
            let mature_settings = settings(id).unwrap();
            let mature = grow(&mature_settings, 0).unwrap();
            let mut young_settings = mature_settings.clone();
            young_settings.maturity = 0.4;
            let young = grow(&young_settings, 0).unwrap();
            assert!(young.valid(), "{id}");
            assert!(young.branches.len() < mature.branches.len(), "{id}: juvenile shoots should develop over time");
            assert!(young.branches[0].radii[0] < mature.branches[0].radii[0] * 0.4, "{id}: juvenile trunk should be slender");
            let (mlo, mhi) = mature.bounds(); let (ylo, yhi) = young.bounds();
            assert!((yhi.y - ylo.y) < (mhi.y - mlo.y) * 0.65, "{id}");
            let mw = (mhi.x - mlo.x).max(mhi.z - mlo.z);
            let yw = (yhi.x - ylo.x).max(yhi.z - ylo.z);
            assert!(yw < mw * 0.7, "{id}: young crown should not retain mature width");
            let vertices = if species(&mature_settings).unwrap().leaves.kind == LeafKind::Needles { 3 } else { 2 + species(&mature_settings).unwrap().leaves.kind.rows().len() * 3 };
            let leaf_length = |tree: &Tree| {
                let mut lengths: Vec<f32> = tree.leaves.v.chunks_exact(vertices).take(1000)
                    .map(|piece| piece[0].p.distance(piece[vertices - 1].p)).collect();
                lengths.sort_by(f32::total_cmp); lengths[lengths.len()/2]
            };
            let ratio = leaf_length(&young) / leaf_length(&mature);
            assert!((0.65..1.45).contains(&ratio), "{id}: leaf dimensions changed by {ratio}");
            young_settings.maturity = 0.15;
            let sapling = grow(&young_settings, 0).unwrap();
            assert!(sapling.valid(), "{id}: youngest sapling must keep wood and foliage");
            assert!(sapling.bounds().1.y < young.bounds().1.y, "{id}: sapling should be shorter than young tree");
        }
    }

    #[test]
    fn bark_height_comes_from_relief_not_albedo() {
        use solstice_procgen::surface::{Surface, Texel};
        let surface = Surface::new(1., |u, _| Texel { color: glam::Vec3::splat(0.5), height: u, roughness: 0.5, cavity: 1. });
        let height = surface.height_map(16);
        assert_eq!(height[0], 0);
        assert_eq!(height[15], 255);
        assert!(height[8] > height[4]);
    }

    #[test]
    fn generated_beech_bark_maps_have_detail() {
        let settings = settings("solstice-forest-beech").unwrap();
        let maps = bark_images(&settings).unwrap();
        let albedo = image::load_from_memory(&maps.color).unwrap().to_rgba8();
        let height = image::load_from_memory(&maps.height).unwrap().to_luma8();
        let normal = image::load_from_memory(&maps.normal).unwrap().to_rgb8();
        let range = |v: Vec<u8>| v.iter().max().unwrap() - v.iter().min().unwrap();
        assert!(range(albedo.pixels().map(|p| p[0]).collect()) > 40);
        assert!(range(height.pixels().map(|p| p[0]).collect()) > 100);
        assert!(range(normal.pixels().map(|p| p[1]).collect()) > 20);
    }

    #[test]
    fn thick_trunk_normals_follow_surface_without_a_uv_seam() {
        let plant = Species::by_name("old_oak").unwrap().grow(0);
        let sides = 40usize;
        let row = sides + 1;
        let trunk = &plant.wood.vertices[plant.branches[0].wood_vertices.clone()];
        assert_eq!(trunk.len() % row, 0);
        for ring in trunk.chunks_exact(row) {
            assert!(ring[0].position.distance(ring[sides].position) < 0.0001);
            assert!(ring[0].normal.distance(ring[sides].normal) < 0.0001);
            assert!(ring.iter().all(|v| v.normal.is_finite() && v.normal.length_squared() > 0.9));
        }
    }

    #[test]
    fn garden_specimen_tip_shoots_retain_branch_parents() {
        for id in ["solstice-willow", "solstice-old_oak"] {
            let mut settings = settings(id).unwrap();
            settings.maturity = 0.4;
            let tree = grow(&settings, 0).unwrap();
            let continuing = tree.branches.iter().skip(1).filter(|branch| {
                let parent = &tree.branches[branch.parent.unwrap()];
                let start = Vec3::from_array(branch.points[0]);
                let end = Vec3::from_array(*parent.points.last().unwrap());
                let origin = Vec3::from_array(parent.points[0]);
                start.distance(end) < origin.distance(end) * 0.025 + 0.002
            }).count();
            assert!(continuing > 0, "{id}: continuation shoots should start near a parent tip");
            assert!(tree.leaf_branch.iter().all(|&branch| (branch as usize) < tree.branches.len()));
        }
    }

    #[test]
    fn all_catalog_species_grow_at_the_youngest_supported_stage() {
        for &id in IDS.iter() {
            let mut settings = settings(id).unwrap();
            settings.maturity = if id == "solstice-bush" { 0.4 } else { 0.15 };
            settings.validate().unwrap();
            assert!(grow(&settings, 0).unwrap().valid(), "{id}: young tree must have valid wood and foliage");
        }
    }

    #[test]
    fn forest_species_files_parse_at_forest_heights() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("species/forest");
        let mut count = 0;
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let text = std::fs::read_to_string(&path).unwrap();
            let plant = Species::parse(&text).unwrap_or_else(|e| panic!("{path:?}: {e}"));
            assert!((14.0..=30.0).contains(&plant.height), "{path:?}");
            count += 1;
        }
        assert!(count >= 5);
    }

    #[test]
    fn reference_presets_preserve_upstream_geometry_and_reduce_foliage() {
        for id in IDS.iter().copied().step_by(6) {
            let settings = settings(id).unwrap();
            settings.validate().unwrap();
            let upstream = species(&settings).unwrap().grow(settings.seed);
            let full = grow(&settings, 0).unwrap();
            let far = grow(&settings, 2).unwrap();
            assert!(full.valid() && far.valid());
            // Same faces as upstream, less the zero-area ones; positions are unscaled at the defaults.
            let at = |v: &[solstice_builder::Vertex], i: u32| {
                let p = v[i as usize].position;
                vec3(p.x, p.y, p.z)
            };
            assert_eq!(
                full.bark.i,
                keep_area(&upstream.wood.indices, |i| at(&upstream.wood.vertices, i))
            );
            assert_eq!(
                full.leaves.i,
                keep_area(&upstream.leaves.indices, |i| at(&upstream.leaves.vertices, i))
            );
            assert!(far.leaves.i.len() < full.leaves.i.len() / 3);
            for (piece, original) in full
                .leaves
                .v
                .windows(2)
                .zip(upstream.leaves.vertices.windows(2))
                .take(64)
            {
                let a = piece[0].p.distance(piece[1].p);
                let b = original[0].position.distance(original[1].position);
                assert!((a - b).abs() < 1e-5);
            }
        }
    }
}
