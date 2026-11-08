//! The tree library: Solstice's procedural species, plus the Poseidon engine's forest oak grown
//! with the same Solstice growth model.
use crate::tree::{self, Part, Settings, Tree, V};
use image::{ImageBuffer, ImageOutputFormat};
use macroquad::prelude::{vec2, vec3, Vec3};
use solstice_procgen::foliage::{leaf::LeafKind, tree::Species};
use std::{io::Cursor, sync::OnceLock};

pub use crate::catalog::IDS;

fn plant(name: &str) -> Option<Species> {
    let id = format!("solstice-{name}");
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
    let height_scale = s.height / species.height;
    species.trunk_radius = s.trunk_radius;
    species.levels[0].per_metre *= s.branches as f32 / Settings::default().branches as f32;
    species.leaves.per_metre *= s.density;
    species.leaves.length.0 *= leaf_scale;
    species.leaves.length.1 *= leaf_scale;
    let plant = species.grow(s.seed);
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
    bark.i = keep_area(&bark.i, |i| bark.v[i as usize].p);
    let needle = species.leaves.kind == LeafKind::Needles;
    let (verts, indices) = if needle { (3, 3) } else { (8, 24) };
    let mut leaves = Part::default();
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
        let center = piece.iter().map(|v| v.position).sum::<glam::Vec3>() / verts as f32;
        let grow = [1., 1.25, 1.63][lod as usize];
        let offset = leaves.v.len() as u32;
        for v in piece {
            let variant = ((v.uv.x * 4.) as usize).min(3);
            let color = colors[variant];
            let mut vertex = convert(v, color);
            let p = center + (v.position - center) * grow;
            vertex.p = vec3(p.x * s.spread, p.y * height_scale, p.z * s.spread);
            leaves.v.push(vertex);
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
    }
    leaves.i = keep_area(&leaves.i, |i| leaves.v[i as usize].p);
    let mut tree = Tree { bark, leaves, lod };
    let base = tree.bark.v.iter().map(|v| v.p.y).fold(0., f32::min);
    for v in tree.bark.v.iter_mut().chain(&mut tree.leaves.v) {
        v.p -= Vec3::Y * base;
    }
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

pub fn leaf_atlas(s: &Settings) -> Option<&'static [u8]> {
    static BROADLEAF: OnceLock<Vec<u8>> = OnceLock::new();
    static BIRCH: OnceLock<Vec<u8>> = OnceLock::new();
    static NEEDLES: OnceLock<Vec<u8>> = OnceLock::new();
    let kind = species(s)?.leaves.kind;
    // One atlas per leaf kind, shared by every species that uses it.
    let cache = match kind {
        LeafKind::Broadleaf => &BROADLEAF,
        LeafKind::Birch => &BIRCH,
        LeafKind::Needles => &NEEDLES,
    };
    Some(cache.get_or_init(|| kind.atlas(0).bake(256).color_mips.remove(0)))
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
        let mut layer = species(s).unwrap().bark.surface(0).bake(EDGE);
        let mut color = layer.color_mips.remove(0);
        let mut height = Vec::new();
        for p in color.chunks_exact_mut(4) {
            // Cavity and albedo approximate a matching relief map from the upstream bake.
            let light = (u32::from(p[0]) + 2 * u32::from(p[1]) + u32::from(p[2])) / 4;
            height.push((light * u32::from(p[3]) / 255) as u8);
            p[3] = 255;
        }
        let (min, max) = (*height.iter().min().unwrap(), *height.iter().max().unwrap());
        for y in &mut height {
            *y = ((u32::from(*y - min) * 255) / u32::from((max - min).max(1))) as u8;
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
