//! Native Poseidon MLOD/P3DM writer and material bundle.
use crate::{
    preview,
    tree::{self, Part, Settings, V},
};
use block_compression::{encode::compress_rgba8, CompressionVariant};
use image::{imageops::FilterType, RgbaImage};
use macroquad::prelude::{vec2, vec3, Vec3};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const EDGE: u32 = 256;
const SIDECARS: [&str; 7] = [
    "bark_co.paa",
    "bark_nohq.paa",
    "leaves_ca.paa",
    "bark.rvmat",
    "leaves.rvmat",
    "bark-height.png",
    "poseidon.json",
];

pub fn sidecars(path: &Path) -> [PathBuf; 7] {
    SIDECARS.map(|name| path.with_extension(name))
}

pub fn game_sidecars(path: &Path) -> Vec<PathBuf> {
    let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_ascii_lowercase();
    let dir = path.parent().unwrap_or(Path::new("."));
    ["bark_co.paa", "bark_co_nohq.paa", "leaves_ca.paa", "leaves_ca_nohq.paa",
     "leaves_co.paa", "leaves_co_nohq.paa", "bark.rvmat", "leaves.rvmat", "wind.json", "occlusion.p3d", "poseidon.json"]
        .iter().map(|suffix| dir.join(format!("{stem}_{suffix}"))).collect()
}

fn name(path: &Path) -> Result<&str, String> {
    let name = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    if name.is_empty()
        || !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
    {
        return Err(
            "Poseidon filenames must contain only letters, digits, underscores or hyphens.".into(),
        );
    }
    Ok(name)
}

pub fn export(s: &Settings, lods: &[u32], path: &Path) -> Result<Value, String> {
    s.validate()?;
    let prefix = format!("grove_{}", name(path)?.to_ascii_lowercase());
    let [bark, normal, foliage, bark_mat, foliage_mat, height, manifest] = sidecars(path);
    let reference = |p: &Path| format!("{prefix}\\{}", p.file_name().unwrap().to_string_lossy());
    let bark_surface = Surface {
        texture: reference(&bark),
        material: reference(&bark_mat),
    };
    let leaf_surface = Surface {
        texture: reference(&foliage),
        material: reference(&foliage_mat),
    };
    let mut model = b"MLOD\x01\x01\x00\x00".to_vec();
    integer(&mut model, lods.len() as u32 + 1);
    let mut counts = Vec::new();
    for &lod in lods {
        let mesh = tree::generate(s, lod);
        let faces = mesh.bark.i.len() / 3 + mesh.leaves.i.len() * 2 / 3;
        let resolution = [1., 2., 4.][lod as usize];
        write_lod(
            &mut model,
            &mesh.bark,
            &mesh.leaves,
            &bark_surface,
            &leaf_surface,
            resolution,
        );
        counts.push(json!({"lod":lod,"resolution":resolution,"faces":faces,"source":mesh.stats()}));
    }
    let collider = trunk_collision(s);
    let empty = Surface {
        texture: String::new(),
        material: String::new(),
    };
    write_lod(
        &mut model,
        &collider,
        &Part::default(),
        &empty,
        &empty,
        1.0e13,
    );

    let load = |data: &[u8]| {
        image::load_from_memory(data)
            .map(|im| {
                im.resize_exact(EDGE, EDGE, FilterType::Triangle)
                    .into_rgba8()
            })
            .map_err(|err| err.to_string())
    };
    let bark_pixels = load(&tree::bark_texture(s)?)?;
    let (height_bytes, normal_bytes) = tree::relief_textures(s)?;
    let mut normal_pixels = load(&normal_bytes)?;
    for p in normal_pixels.pixels_mut() {
        // Poseidon's native NOHQ path reads tangent X from A and Y from G.
        p.0 = [128, p[1], 255, p[0]];
    }
    let atlas = RgbaImage::from_raw(EDGE, EDGE, preview::leaf_rgba(s)).unwrap();
    let foliage_pixels = RgbaImage::from_fn(EDGE, EDGE, |x, y| {
        let sample_y = (y as f32 * 2. - EDGE as f32 * 0.5).clamp(0., EDGE as f32 - 1.);
        let mut p = *atlas.get_pixel(x, sample_y as u32);
        // The leaf faces sample V=0.25..0.75. The transparent strip is outside
        // their footprint and activates the engine's existing cutout/flutter route.
        p.0[3] = if y < EDGE / 16 || y >= EDGE * 15 / 16 {
            0
        } else {
            p[3]
        };
        p
    });
    tree::write(&bark, paa(bark_pixels, CompressionVariant::BC1, false))?;
    tree::write(&normal, paa(normal_pixels, CompressionVariant::BC3, false))?;
    tree::write(&foliage, paa(foliage_pixels, CompressionVariant::BC3, true))?;
    tree::write(&height, height_bytes)?;
    tree::write(&bark_mat, material("TreeAdvTrunk", &reference(&normal)))?;
    tree::write(
        &foliage_mat,
        material("TreeAdv", "#(argb,8,8,3)color(0.5,0.5,1,1,NOHQ)"),
    )?;
    let asset = json!({
        "path": path, "format": "MLOD 1.1 / P3DM", "settings": s,
        "pboPrefix": prefix, "model": reference(path), "lods": counts,
        "geometryLod": "Convex trunk collision",
        "wind": {"map":"tree","height":"model-space Y, meters","leafFlutter":"TreeAdv alpha-test section"},
        "barkRelief": {"heightMap":height,"depth":s.parallax_depth,"whiteIsRaised":true},
        "sidecars": sidecars(path)
    });
    tree::write(path, model)?;
    tree::write(&manifest, serde_json::to_vec_pretty(&asset).unwrap())?;
    Ok(asset)
}

/// How a game ladder export is written.
pub struct GameOptions {
    /// The PBO prefix the textures and materials are named under (default `grove_<name>`).
    pub prefix: Option<String>,
    /// Write the FOREST-PROXY tree properties (lodfade, facepairs, forestcrowns, barkpom,
    /// shadowcoarser, forestinterior) so the model can stand as a per-tree forest instance.
    pub forest_proxy: bool,
    /// Triangle budgets near, mid, far, very far (`--budgets`); None = the default ladder.
    pub budgets: Option<Vec<usize>>,
    /// `--opaque-leaves`: two levels, a near level of opaque leaf pieces (no alpha test, its own
    /// BC1 leaf texture `_leaves_co.paa`) marked `clusterfoliage=1`, which OpenPoseidon reduces
    /// per cluster with distance (GROVE-OPAQUE), and the very far spray level, which casts the
    /// shadows (`shadowcoarser`) and stands for the tree in the far forest.
    pub opaque_leaves: bool,
    /// `--twig-cards`: every level's foliage is cards of baked twigs (gamelod's ladder replaced by
    /// twigs.rs), sampling `<name>_twigs_ca.paa` and its `_nohq`; the far level casts the shadow.
    pub twig_cards: bool,
    /// `--card-bend`: share of the crown ellipsoid in the twig cards' normals (None = default).
    pub card_bend: Option<f32>,
    /// Trim alpha-card outlines using the generated atlas.
    pub trim_cards: bool,
}

/// The native Poseidon tree with the game LOD ladder (near, mid, far, very far; see gamelod.rs)
/// and a trunk Geometry LOD. Every texture ships its `_nohq` normal map beside it.
pub fn export_game(s: &Settings, path: &Path, options: &GameOptions) -> Result<Value, String> {
    s.validate()?;
    let stem = name(path)?.to_ascii_lowercase();
    let prefix = options.prefix.clone().unwrap_or_else(|| format!("grove_{stem}"));
    if prefix.is_empty() || prefix.contains('/') {
        return Err("The prefix must be a PBO prefix such as op_grove_trees".into());
    }
    let dir = path.parent().unwrap_or(Path::new("."));
    let file = |suffix: &str| dir.join(format!("{stem}_{suffix}"));
    let [bark, bark_n, leaves, leaves_n, bark_mat, leaf_mat, manifest] = [
        "bark_co.paa",
        "bark_co_nohq.paa",
        "leaves_ca.paa",
        "leaves_ca_nohq.paa",
        "bark.rvmat",
        "leaves.rvmat",
        "poseidon.json",
    ]
    .map(file);
    let wind_sidecar = file("wind.json");
    let reference = |p: &Path| format!("{prefix}\\{}", p.file_name().unwrap().to_string_lossy());
    // As the engine's own generated trees: no RVMAT, the face texture is the colour and the
    // renderer finds `<texture>_nohq.paa` beside it (an RVMAT without a BaseColour slot only
    // made the WGPU material path warn). The RVMATs are still written for other tools.
    let bark_surface = Surface {
        texture: reference(&bark),
        material: String::new(),
    };
    let leaf_surface = Surface {
        texture: reference(&leaves),
        material: String::new(),
    };
    let mut ladder = if options.twig_cards {
        crate::twigs::ladder(s, options.budgets.as_deref(), options.trim_cards, options.card_bend.unwrap_or(crate::twigs::DEFAULT_BEND))?
    } else if options.opaque_leaves {
        crate::gamelod::ladder_with(s, &crate::gamelod::opaque_rungs(options.budgets.as_deref())?)
    } else {
        match &options.budgets {
            Some(b) => crate::gamelod::ladder_with(s, &crate::gamelod::rungs_with_budgets(b)?),
            None => crate::gamelod::ladder(s),
        }
    };
    if options.trim_cards { crate::gamelod::trim_cards(&mut ladder); }
    let opaque_leaves = file("leaves_co.paa");
    let opaque_leaves_n = file("leaves_co_nohq.paa");
    let opaque_surface = Surface {
        texture: reference(&opaque_leaves),
        material: String::new(),
    };
    let mut model = b"MLOD\x01\x01\x00\x00".to_vec();
    let twigs = file("twigs_ca.paa");
    let twigs_n = file("twigs_ca_nohq.paa");
    let twig_surface = Surface { texture: reference(&twigs), material: String::new() };
    let proxy = crate::gamelod::proxy_ladder(s);
    // twig cards: the far level (limb cards) stands in for shadows and occlusion
    let shadow = if ladder.twig.is_some() { &ladder.levels[2] } else { &proxy.levels[0] };
    let shadow_leaf_surface = if ladder.twig.is_some() { &twig_surface } else { &leaf_surface };
    integer(&mut model, ladder.levels.len() as u32 + 2);
    let height = ladder
        .levels
        .first()
        .map(|l| {
            l.bark
                .v
                .iter()
                .chain(&l.leaves.v)
                .map(|v| v.p.y)
                .fold(0., f32::max)
        })
        .unwrap_or(s.height);
    let mut counts = Vec::new();
    for (k, level) in ladder.levels.iter().enumerate() {
        let mut props: Vec<(&str, String)> = Vec::new();
        if options.forest_proxy {
            props.push(("facepairs", "1".into()));
            props.push(("forestcrowns", "1".into()));
            props.push((
                "forestinterior",
                format!("{:.1} 0.35", (height * 0.65).max(4.)),
            ));
            if k == 0 {
                props.extend([("lodfade", "1".into()), ("barkpom", "1".into())]);
                // an opaque-leaf near level casts its own shadow, reduced per cluster from the eye
                // (OpenPoseidon WGR_CLUSTER_LOD_SHADOW_PX); the very far sprays are alpha-tested
                if !options.opaque_leaves {
                    props.push(("shadowcoarser", "1".into()));
                }
            }
        }
        // twig cards are alpha cards, not opaque leaf pieces for per-cluster reduction
        if k == 0 && !options.twig_cards {
            props.push(("clusterfoliage", "1".into()));
        }
        write_lod_with(
            &mut model,
            &level.bark,
            &level.leaves,
            &bark_surface,
            if options.twig_cards { &twig_surface } else if k == 0 { &opaque_surface } else { &leaf_surface },
            level.rung.resolution,
            true,
            &props,
        );
        counts.push(json!({
            "level": level.rung.name, "resolution": level.rung.resolution,
            "triangles": level.triangles(),
            "faces": level.bark.i.len() / 3 + level.leaves.i.len() * 2 / 3,
            "vertices": level.vertices(),
        }));
    }
    // Resolution 11000 is the ShadowBuffer sentinel recognized by the model loader.
    // Some engine builds still cast visible objects from visual LODs rather than this level.
    write_lod_with(&mut model, &shadow.bark, &shadow.leaves, &bark_surface, shadow_leaf_surface,
        11000.0, true, &[]);
    let occlusion_path = file("occlusion.p3d");
    let mut occlusion = b"MLOD\x01\x01\x00\x00".to_vec();
    integer(&mut occlusion, 1);
    write_lod_with(&mut occlusion, &shadow.bark, &shadow.leaves, &bark_surface, shadow_leaf_surface,
        8.0, true, &[]);
    tree::write(&occlusion_path, occlusion)?;
    let empty = Surface {
        texture: String::new(),
        material: String::new(),
    };
    write_lod(&mut model, &trunk_collision(s), &Part::default(), &empty, &empty, 1.0e13);

    const BARK_EDGE: u32 = 512;
    let load = |data: &[u8]| {
        image::load_from_memory(data)
            .map(|im| {
                im.resize_exact(BARK_EDGE, BARK_EDGE, FilterType::Triangle)
                    .into_rgba8()
            })
            .map_err(|err| err.to_string())
    };
    let bark_pixels = load(&tree::bark_texture(s)?)?;
    let (_, normal_bytes) = tree::relief_textures(s)?;
    let mut bark_normal = load(&normal_bytes)?;
    for p in bark_normal.pixels_mut() {
        // OpenPoseidon's `_nohq` layout, as its own tree generator writes it (TreeCommand.cpp
        // WriteNohq): G = y, B = z, A = x, R = 0. A non-zero R is not ignored by the WGPU
        // material path (2026-11-08: the first Grove forest drew pitch-black trunks with R=128).
        let [x, y] = [p[0], p[1]].map(|c| c as f32 / 127.5 - 1.);
        let z = (1. - x * x - y * y).max(0.).sqrt();
        p.0 = [0, p[1], (z * 127.5 + 127.5) as u8, p[0]];
    }
    let edge = crate::gamelod::ATLAS;
    // a twig-card ladder has no single-leaf atlas: its foliage samples the twig atlas only
    let atlas = RgbaImage::from_raw(edge, edge, ladder.atlas.clone()).unwrap_or_else(|| RgbaImage::new(edge, edge));
    let leaf_normal = if s.leaf_atlas_path.is_some() { leaf_normal_map(&atlas) } else {
        let (normals, _) = preview::leaf_surface_maps(s, true, edge as usize);
        RgbaImage::from_fn(edge, edge, |x,y| {
            let i = ((y*edge+x)*2) as usize;
            let nx = normals[i] as f32 / 127.5 - 1.;
            let ny = normals[i+1] as f32 / 127.5 - 1.;
            let nz = (1.-nx*nx-ny*ny).max(0.).sqrt();
            image::Rgba([0, normals[i+1], (nz*127.5+127.5).round() as u8, normals[i]])
        })
    };
    let debug_png = std::env::var_os("GROVE_DEBUG_PNG").is_some();
    if debug_png {
        // PNG to look at, raw RGBA (edge from the manifest) for offline tools
        bark_pixels.save(file("bark_co.png")).map_err(|e| e.to_string())?;
        tree::write(&file("bark_co.rgba"), bark_pixels.as_raw())?;
        if ladder.twig.is_none() {
            atlas.save(file("leaves_ca.png")).map_err(|e| e.to_string())?;
            tree::write(&file("leaves_ca.rgba"), atlas.as_raw())?;
        }
    }
    if let Some(twig) = &ladder.twig {
        let color = RgbaImage::from_raw(twig.edge, twig.edge, twig.color.clone()).unwrap();
        let normal = RgbaImage::from_raw(twig.edge, twig.edge, twig.normal.clone()).unwrap();
        if debug_png {
            color.save(file("twigs_ca.png")).map_err(|e| e.to_string())?;
            normal.save(file("twigs_ca_nohq.png")).map_err(|e| e.to_string())?;
            tree::write(&file("twigs_ca.rgba"), &twig.color)?;
            tree::write(&file("twigs_ca_nohq.rgba"), &twig.normal)?;
        }
        tree::write(&twigs, paa_with(color, CompressionVariant::BC3, true, Some(&twig.cells)))?;
        tree::write(&twigs_n, paa(normal, CompressionVariant::BC3, false))?;
    }
    tree::write(&bark, paa(bark_pixels, CompressionVariant::BC1, false))?;
    tree::write(&bark_n, paa(bark_normal, CompressionVariant::BC3, false))?;
    if ladder.twig.is_none() {
        let mut opaque = atlas.clone();
        for p in opaque.pixels_mut() {
            p.0[3] = 255;
        }
        tree::write(&opaque_leaves, paa(opaque, CompressionVariant::BC1, false))?;
        tree::write(&opaque_leaves_n, paa(leaf_normal.clone(), CompressionVariant::BC3, false))?;
        tree::write(&leaves, paa(atlas, CompressionVariant::BC3, true))?;
        tree::write(&leaves_n, paa(leaf_normal, CompressionVariant::BC3, false))?;
    }
    tree::write(&bark_mat, material("TreeAdvTrunk", &reference(&bark_n)))?;
    tree::write(&leaf_mat, material("TreeAdv", &reference(&leaves_n)))?;
    let detailed_wind = json!({
        "format": "grove-branch-wind", "version": 1,
        "model": path.file_name().unwrap().to_string_lossy(),
        "coordinates": "model-space metres, Y up; P3D origin",
        "pointOrder": "P3DM points: bark followed by foliage; collision LOD excluded",
        "branchAssignment": "source shoot for wood and individual leaves; lowest shared ancestor for a spray cluster",
        "branches": ladder.branches,
        "levels": ladder.levels.iter().map(|level| json!({
            "name": level.rung.name, "resolution": level.rung.resolution,
            "barkPoints": level.bark.v.len(), "foliagePoints": level.leaves.v.len(),
            "pointBranches": level.bark.v.iter().chain(&level.leaves.v).map(|v| v.branch).collect::<Vec<_>>()
        })).collect::<Vec<_>>()
    });
    tree::write(&wind_sidecar, serde_json::to_vec(&detailed_wind).map_err(|e| e.to_string())?)?;
    let asset = json!({
        "path": path, "format": "MLOD 1.1 / P3DM", "settings": s,
        "pboPrefix": prefix, "model": reference(path), "levels": counts,
        "forestProxy": options.forest_proxy, "opaqueLeaves": true, "trimCards": options.trim_cards,
        "twigCards": ladder.twig.as_ref().map(|t| json!({"texture": reference(&twigs), "normal": reference(&twigs_n),
            "edge": t.edge, "cells": t.cells, "levels": t.report})),
        "geometryLod": "Convex trunk collision",
        "shadowProxy": {"resolution":11000.0,"triangles":shadow.triangles(),"p3dFaces":shadow.bark.i.len()/3 + shadow.leaves.i.len()*2/3,"runtimeSelection":"requires engine support"},
        "occlusionProxy": occlusion_path.file_name().unwrap().to_string_lossy(),
        "wind": {"map":"tree","height":"model-space Y, meters","hierarchy":wind_sidecar.file_name().unwrap().to_string_lossy(),"hierarchyRequiresEngineSupport":true},
        "sidecars": if ladder.twig.is_some() {
            json!([&bark, &bark_n, &twigs, &twigs_n, &bark_mat, &leaf_mat, &wind_sidecar, &occlusion_path])
        } else {
            json!([&bark, &bark_n, &leaves, &leaves_n, &opaque_leaves, &opaque_leaves_n, &bark_mat, &leaf_mat, &wind_sidecar, &occlusion_path])
        },
    });
    tree::write(path, model)?;
    tree::write(&manifest, serde_json::to_vec_pretty(&asset).unwrap())?;
    Ok(asset)
}

/// A tangent-space normal map for the leaf atlas from its luminance (veins and needle gaps sit
/// lower), in the native NOHQ layout (X in A, Y in G).
fn leaf_normal_map(atlas: &RgbaImage) -> RgbaImage {
    let (w, h) = atlas.dimensions();
    let lum = |x: i64, y: i64| {
        let p = atlas.get_pixel(x.clamp(0, w as i64 - 1) as u32, y.clamp(0, h as i64 - 1) as u32);
        (p[0] as f32 * 0.25 + p[1] as f32 * 0.5 + p[2] as f32 * 0.25) / 255.
    };
    RgbaImage::from_fn(w, h, |x, y| {
        let (x, y) = (x as i64, y as i64);
        let strength = 2.0;
        let dx = (lum(x + 1, y) - lum(x - 1, y)) * strength;
        let dy = (lum(x, y + 1) - lum(x, y - 1)) * strength;
        let n = vec3(-dx, -dy, 1.).normalize();
        let byte = |c: f32| ((c * 0.5 + 0.5) * 255.).round() as u8;
        image::Rgba([0, byte(n.y), byte(n.z), byte(n.x)])
    })
}

fn integer(data: &mut Vec<u8>, n: u32) {
    data.extend(n.to_le_bytes());
}

fn floats(data: &mut Vec<u8>, values: impl IntoIterator<Item = f32>) {
    for n in values {
        data.extend(n.to_le_bytes());
    }
}

fn string(data: &mut Vec<u8>, s: &str) {
    data.extend(s.as_bytes());
    data.push(0);
}

fn tag(data: &mut Vec<u8>, name: &str, size: u32) {
    data.push(1);
    string(data, name);
    integer(data, size);
}

fn property(data: &mut Vec<u8>, key: &str, value: &str) {
    tag(data, "#Property#", 128);
    for field in [key, value] {
        data.extend(field.as_bytes());
        data.resize(data.len() + 64 - field.len(), 0);
    }
}

struct Surface {
    texture: String,
    material: String,
}

fn write_lod(
    data: &mut Vec<u8>,
    wood: &Part,
    leaves: &Part,
    bark: &Surface,
    foliage: &Surface,
    resolution: f32,
) {
    write_lod_with(data, wood, leaves, bark, foliage, resolution, false, &[]);
}

/// `raw_uv`: the foliage UVs are already in the leaf texture's layout (the game ladder's
/// atlas); otherwise V is squeezed into the editor atlas' opaque band. `props` are extra named
/// properties of the level.
#[allow(clippy::too_many_arguments)]
fn write_lod_with(
    data: &mut Vec<u8>,
    wood: &Part,
    leaves: &Part,
    bark: &Surface,
    foliage: &Surface,
    resolution: f32,
    raw_uv: bool,
    props: &[(&str, String)],
) {
    let points = (wood.v.len() + leaves.v.len()) as u32;
    let normals = points + leaves.v.len() as u32;
    let faces = (wood.i.len() / 3 + leaves.i.len() * 2 / 3) as u32;
    data.extend(b"P3DM");
    for n in [28, 256, points, normals, faces, 0] {
        integer(data, n);
    }
    for v in wood.v.iter().chain(&leaves.v) {
        floats(data, v.p.to_array());
        integer(data, 0);
    }
    // MLOD stores normals negated (as the face winding is: the loader flips both), exactly as
    // OpenPoseidon's own generated trees do; written as-is the bark was lit from inside and
    // drew black in game (2026-11-08, measured: the engine's trees' trunk normals point in).
    for v in wood.v.iter().chain(&leaves.v) {
        floats(data, (-v.n).to_array());
    }
    for v in &leaves.v {
        floats(data, v.n.to_array());
    }
    for (part, surface, first, foliage) in [
        (wood, bark, 0, false),
        (leaves, foliage, wood.v.len() as u32, true),
    ] {
        for tri in part.i.chunks_exact(3) {
            for back in [false, true].into_iter().take(if foliage { 2 } else { 1 }) {
                // The native loader reverses the first two corners of every face.
                let order = if back { [0, 1, 2] } else { [1, 0, 2] };
                integer(data, 3);
                for j in order {
                    let i = tri[j];
                    let v = &part.v[i as usize];
                    let point = first + i;
                    integer(data, point);
                    integer(data, if back { points + i } else { point });
                    floats(
                        data,
                        [
                            v.uv.x,
                            if foliage && !raw_uv { 0.25 + 0.5 * v.uv.y } else { v.uv.y },
                        ],
                    );
                }
                data.extend([0; 16]);
                integer(data, 0);
                string(data, &surface.texture);
                string(data, &surface.material);
            }
        }
    }
    data.extend(b"TAGG");
    property(data, "map", "tree");
    property(data, "autocenter", "0");
    for (key, value) in props {
        property(data, key, value);
    }
    if resolution == 1.0e13 {
        tag(data, "component01", points + faces);
        data.resize(data.len() + (points + faces) as usize, 1);
        tag(data, "#Mass#", 4 * points);
        floats(
            data,
            std::iter::repeat_n(100. / points as f32, points as usize),
        );
    }
    tag(data, "#EndOfFile#", 0);
    floats(data, [resolution]);
}

fn trunk_collision(s: &Settings) -> Part {
    let mut part = Part::default();
    for ring in 0..2 {
        for j in 0..8 {
            let (sin, cos) = (j as f32 * std::f32::consts::TAU / 8.).sin_cos();
            let radius = s.trunk_radius * tree::age_radius_scale(s.maturity) * if ring == 0 { 1.3 } else { 0.42 };
            part.v.push(V {
                p: vec3(cos * radius, ring as f32 * s.height * s.maturity * 0.62, sin * radius),
                n: Vec3::Y,
                uv: vec2(0., 0.),
                c: [1.; 4],
                branch: 0,
            });
        }
    }
    for j in 0..8 {
        let next = (j + 1) % 8;
        part.i.extend([j, j + 8, next, next, j + 8, next + 8]);
    }
    for j in 1..7 {
        part.i.extend([0, j, j + 1, 8, j + 9, j + 8]);
    }
    part
}

fn paa(image: RgbaImage, compression: CompressionVariant, alpha: bool) -> Vec<u8> {
    paa_with(image, compression, alpha, None)
}

/// Alpha-test coverage of a texel rectangle (x, y, w, h): the share at or above half alpha.
fn coverage(image: &RgbaImage, r: [u32; 4]) -> f32 {
    let mut n = 0;
    for y in r[1]..r[1] + r[3] { for x in r[0]..r[0] + r[2] { if image.get_pixel(x, y)[3] >= 128 { n += 1; } } }
    n as f32 / (r[2] * r[3]).max(1) as f32
}

/// `cells`: scale each mip's alpha per atlas cell (texel rectangles of the base level) so the
/// alpha test keeps the base level's coverage; a box-filtered twig otherwise dissolves at a
/// distance. This is the one coverage lift: the engine's `cutout_mip_alpha` must stay off for
/// these textures (its shadow pass does not apply one, this one reaches the shadows too).
fn paa_with(mut image: RgbaImage, compression: CompressionVariant, alpha: bool, cells: Option<&[[u32; 4]]>) -> Vec<u8> {
    let base: Option<Vec<([u32; 4], f32)>> = cells.map(|c| c.iter().map(|&r| (r, coverage(&image, r))).collect());
    let edge = image.width();
    let magic = if matches!(compression, CompressionVariant::BC1) {
        0xff01_u16
    } else {
        0xff05
    };
    let mut output = magic.to_le_bytes().to_vec();
    if alpha {
        output.extend(b"GGATGALF");
        integer(&mut output, 4);
        integer(&mut output, 1);
    }
    output.extend([0; 2]);
    loop {
        let size = image.width();
        let mut compressed = vec![0; compression.blocks_byte_size(size, size)];
        compress_rgba8(
            compression,
            image.as_raw(),
            &mut compressed,
            size,
            size,
            size * 4,
        );
        output.extend((size as u16).to_le_bytes());
        output.extend((size as u16).to_le_bytes());
        output.extend_from_slice(&(compressed.len() as u32).to_le_bytes()[..3]);
        output.extend(compressed);
        if size == 4 {
            break;
        }
        image = image::imageops::resize(&image, size / 2, size / 2, FilterType::Triangle);
        if let Some(wanted) = &base {
            let down = edge / image.width();
            for &(r, want) in wanted {
                let (cx, cy, cw, ch) = (r[0] / down, r[1] / down, r[2] / down, r[3] / down);
                if cw < 2 || ch < 2 { continue; }
                {
                    let covered = |scale: f32, image: &RgbaImage| {
                        let mut n = 0;
                        for y in cy..cy + ch { for x in cx..cx + cw {
                            if image.get_pixel(x, y)[3] as f32 * scale >= 127.5 { n += 1; }
                        } }
                        n as f32 / (cw * ch) as f32
                    };
                    let (mut lo, mut hi) = (1f32, 8f32);
                    if covered(lo, &image) >= want { continue; }
                    for _ in 0..12 {
                        let mid = (lo + hi) * 0.5;
                        if covered(mid, &image) < want { lo = mid } else { hi = mid }
                    }
                    for y in cy..cy + ch { for x in cx..cx + cw {
                        let p = image.get_pixel_mut(x, y);
                        p[3] = (p[3] as f32 * hi).min(255.).round() as u8;
                    } }
                }
            }
        }
    }
    output.extend([0; 4]);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forest_proxy_export_writes_the_per_tree_properties_and_normal_maps() {
        let dir = std::env::temp_dir().join(format!("grove-proxy-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("beech_a.p3d");
        let s = Settings::preset("solstice-cwa-buk").unwrap();
        let options = GameOptions {
            prefix: Some("op_grove_trees".into()),
            forest_proxy: true,
            budgets: None,
            opaque_leaves: false,
            twig_cards: false,
            card_bend: None,
            trim_cards: true,
        };
        let asset = export_game(&s, &path, &options).unwrap();
        let wind: Value = serde_json::from_slice(&std::fs::read(dir.join("beech_a_wind.json")).unwrap()).unwrap();
        let branches = wind["branches"].as_array().unwrap();
        assert!(branches.len() > 1);
        for (i, branch) in branches.iter().enumerate().skip(1) {
            assert!(branch["parent"].as_u64().unwrap() < i as u64);
        }
        for (level, metadata) in asset["levels"].as_array().unwrap().iter().zip(wind["levels"].as_array().unwrap()) {
            let owners = metadata["pointBranches"].as_array().unwrap();
            assert_eq!(owners.len() as u64, level["vertices"].as_u64().unwrap());
            assert!(owners.iter().all(|owner| owner.as_u64().unwrap() < branches.len() as u64));
            assert!(owners.iter().any(|owner| owner.as_u64().unwrap() > 0));
        }
        let model = std::fs::read(&path).unwrap();
        let has = |needle: &[u8]| model.windows(needle.len()).any(|w| w == needle);
        for key in ["facepairs", "forestcrowns", "forestinterior", "lodfade", "barkpom", "shadowcoarser"] {
            assert!(has(key.as_bytes()), "{key}");
        }
        assert!(has(b"op_grove_trees\\beech_a_leaves_ca.paa"));
        assert!(has(b"op_grove_trees\\beech_a_leaves_co.paa"));
        assert!(has(b"clusterfoliage"));
        assert_eq!(u32::from_le_bytes(model[8..12].try_into().unwrap()), 6);
        assert!(model.windows(4).any(|bytes| bytes == 11000_f32.to_le_bytes()));
        assert!(asset["shadowProxy"]["p3dFaces"].as_u64().unwrap() <= 2000);
        assert!(dir.join("beech_a_occlusion.p3d").is_file());
        assert!(!has(b".rvmat"), "faces name no RVMAT");
        for tex in ["bark_co", "leaves_ca"] {
            assert!(dir.join(format!("beech_a_{tex}_nohq.paa")).is_file());
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn opaque_leaves_export_is_two_levels_with_an_opaque_cluster_foliage_near_level() {
        let dir = std::env::temp_dir().join(format!("grove-opaque-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("oak_a.p3d");
        let s = Settings::preset("solstice-cwa-dub").unwrap();
        let options = GameOptions {
            prefix: Some("op_grove_trees".into()),
            forest_proxy: true,
            budgets: Some(vec![40_000, 400]),
            opaque_leaves: true,
            twig_cards: false,
            card_bend: None,
            trim_cards: true,
        };
        let asset = export_game(&s, &path, &options).unwrap();
        assert_eq!(asset["levels"].as_array().unwrap().len(), 2);
        let model = std::fs::read(&path).unwrap();
        let has = |needle: &[u8]| model.windows(needle.len()).any(|w| w == needle);
        assert!(has(b"clusterfoliage"));
        assert!(!has(b"shadowcoarser"), "the opaque near level casts its own shadow");
        assert!(has(b"op_grove_trees\\oak_a_leaves_co.paa"));
        assert!(has(b"op_grove_trees\\oak_a_leaves_ca.paa"), "the very far sprays keep the alpha atlas");
        for tex in ["leaves_co", "leaves_co_nohq", "leaves_ca"] {
            assert!(dir.join(format!("oak_a_{tex}.paa")).is_file(), "{tex}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}

fn material(shader: &str, normal: &str) -> String {
    format!(
        r##"ambient[]={{1,1,1,1}};
diffuse[]={{1,1,1,1}};
forcedDiffuse[]={{0.12,0.12,0.12,1}};
emmisive[]={{0.1,0.1,0.1,0.5}};
specular[]={{0.08,0.08,0.08,0.1}};
specularPower=24;
PixelShaderID="{shader}";
VertexShaderID="{shader}";
class Stage1 {{
    texture="{normal}";
    uvSource="tex";
}};
class Stage2 {{
    texture="#(argb,8,8,3)color(1,1,1,1,MCA)";
    uvSource="tex";
}};
"##
    )
}
