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
    let bark_pixels = load(tree::bark_texture(s))?;
    let (height_bytes, normal_bytes) = tree::relief_textures(s);
    let mut normal_pixels = load(normal_bytes)?;
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
            255
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
    for v in wood.v.iter().chain(&leaves.v) {
        floats(data, v.n.to_array());
    }
    for v in &leaves.v {
        floats(data, (-v.n).to_array());
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
                        [v.uv.x, if foliage { 0.25 + 0.5 * v.uv.y } else { v.uv.y }],
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
            let radius = s.trunk_radius * if ring == 0 { 1.3 } else { 0.42 };
            part.v.push(V {
                p: vec3(cos * radius, ring as f32 * s.height * 0.62, sin * radius),
                n: Vec3::Y,
                uv: vec2(0., 0.),
                c: [1.; 4],
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

fn paa(mut image: RgbaImage, compression: CompressionVariant, alpha: bool) -> Vec<u8> {
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
    }
    output.extend([0; 4]);
    output
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
