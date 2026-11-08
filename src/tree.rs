use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{fs, path::Path};

// Bark images come from the Solstice species' procedural bake, so exports need no bundled photos.
fn bark(s: &Settings) -> &'static crate::reference::BarkImages {
    crate::reference::bark_images(s).expect("validated Solstice species")
}
pub fn relief_textures(s: &Settings) -> (&'static [u8], &'static [u8]) {
    let bark = bark(s);
    (&bark.height, &bark.normal)
}
pub fn bark_texture(s: &Settings) -> &'static [u8] {
    &bark(s).color
}

pub fn leaf_color(s: &Settings, variant: usize) -> [f32; 4] {
    let t = variant as f32 / 3.;
    let palette = crate::reference::palette(s);

    let [r, g, b] = match (s.season, palette) {
        (3, "pine") => [0.043, 0.10 + 0.033 * t, 0.072],
        (3, _) => [0.22 + 0.11 * t, 0.11 + 0.06 * t, 0.045],
        (1, "pine") => [0.055, 0.11 + 0.05 * t, 0.031],
        (1, "birch") => [0.45 + 0.15 * t, 0.28 + 0.19 * t, 0.024],
        (1, _) => [0.30 + 0.20 * t, 0.11 + 0.13 * t, 0.024],
        (2, _) => [0.09 + 0.065 * t, 0.22 + 0.12 * t, 0.030],
        (_, "pine") => [0.029 + 0.021 * t, 0.08 + 0.052 * t, 0.040],
        (_, "birch") => [0.078 + 0.044 * t, 0.16 + 0.073 * t, 0.027],
        _ => [0.045 + 0.056 * t, 0.11 + 0.084 * t, 0.019],
    };
    [r, g, b, 1.]
}

pub fn srgb(linear: f32) -> f32 {
    if linear <= 0.0031308 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1. / 2.4) - 0.055
    }
}

pub fn wind_weights(v: &V, height: f32, foliage: bool) -> [f32; 2] {
    [
        (v.p.y / height).clamp(0., 1.).powi(2),
        if foliage { 1. } else { 0. },
    ]
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub species: String,
    pub seed: u32,
    pub height: f32,
    pub spread: f32,
    pub branches: u32,
    pub density: f32,
    pub leaf_size: f32,
    pub trunk_radius: f32,
    pub season: u32,
    pub parallax_depth: f32,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            species: "solstice-oak".into(),
            seed: 42,
            height: 8.,
            spread: 1.,
            branches: 11,
            density: 1.,
            leaf_size: 0.23,
            trunk_radius: 0.23,
            season: 0,
            parallax_depth: 0.035,
        }
    }
}
impl Settings {
    pub fn is_reference(&self) -> bool {
        crate::reference::IDS.contains(&self.species.as_str())
    }

    pub fn preset(name: &str) -> Result<Self, String> {
        crate::reference::settings(&name.to_lowercase()).ok_or_else(|| {
            format!(
                "Unknown preset. Choose one of: {}.",
                crate::reference::IDS.join(", ")
            )
        })
    }
    pub fn validate(&self) -> Result<(), String> {
        if !self.is_reference() {
            return Err("Unknown tree species".into());
        }
        let shrub = self.species == "solstice-bush";
        for (name, v, lo, hi) in [
            (
                "height",
                self.height,
                if shrub { 0.25 } else { 3. },
                // forest-grown trees (species files) reach 25-30 m
                if shrub { 0.65 } else { 40. },
            ),
            ("spread", self.spread, 0.4, 1.6),
            ("density", self.density, 0.25, 1.8),
            ("leaf_size", self.leaf_size, 0.025, 0.40),
            ("trunk_radius", self.trunk_radius, 0.05, 0.45),
            ("parallax_depth", self.parallax_depth, 0., 0.08),
        ] {
            if !v.is_finite() || v < lo || v > hi {
                return Err(format!("{name} must be between {lo} and {hi}"));
            }
        }
        if !(5..=20).contains(&self.branches) || self.season > 3 {
            return Err("Branches must be 5..20; season must be 0..3 (winter is 3)".into());
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct V {
    pub p: Vec3,
    pub n: Vec3,
    pub c: [f32; 4],
    pub uv: Vec2,
}
#[derive(Clone, Default)]
pub struct Part {
    pub v: Vec<V>,
    pub i: Vec<u32>,
}
impl Part {
    // UV-derived tangent basis shared by the preview and glTF normal/parallax maps.
    pub fn tangents(&self) -> Vec<Vec4> {
        let mut ts = vec![Vec3::ZERO; self.v.len()];
        let mut bs = ts.clone();
        for tri in self.i.chunks_exact(3) {
            let [a, b, c] = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
            let e1 = self.v[b].p - self.v[a].p;
            let e2 = self.v[c].p - self.v[a].p;
            let d1 = self.v[b].uv - self.v[a].uv;
            let d2 = self.v[c].uv - self.v[a].uv;
            let det = d1.x * d2.y - d1.y * d2.x;
            if det.abs() < 1e-8 {
                continue;
            }
            let t = (e1 * d2.y - e2 * d1.y) / det;
            let b = (e2 * d1.x - e1 * d2.x) / det;
            for i in tri {
                ts[*i as usize] += t;
                bs[*i as usize] += b;
            }
        }
        self.v
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let n = v.n.normalize();
                let mut t = (ts[i] - n * n.dot(ts[i])).normalize_or_zero();
                if t.length_squared() < 0.5 {
                    t = basis(n).0;
                }
                t.extend(if n.cross(t).dot(bs[i]) < 0. { -1. } else { 1. })
            })
            .collect()
    }
}
pub struct Tree {
    pub bark: Part,
    pub leaves: Part,
    pub lod: u32,
}
impl Tree {
    pub fn triangles(&self) -> usize {
        (self.bark.i.len() + self.leaves.i.len()) / 3
    }
    pub fn vertices(&self) -> usize {
        self.bark.v.len() + self.leaves.v.len()
    }
    pub fn bounds(&self) -> (Vec3, Vec3) {
        let mut lo = vec3(f32::MAX, f32::MAX, f32::MAX);
        let mut hi = -lo;
        for v in self.bark.v.iter().chain(&self.leaves.v) {
            lo = lo.min(v.p);
            hi = hi.max(v.p)
        }
        (lo, hi)
    }
    pub fn stats(&self) -> Value {
        let (lo, hi) = self.bounds();
        json!({"lod":self.lod,"triangles":self.triangles(),"vertices":self.vertices(),"draw_calls":2,"materials":2,"bounds":{"min":lo.to_array(),"max":hi.to_array()},"height_m":hi.y-lo.y,"estimated_vertex_bytes":self.vertices()*72,"index_bytes":self.triangles()*12,"wind_attribute":"_WIND","valid":self.valid()})
    }
    pub fn valid(&self) -> bool {
        [&self.bark, &self.leaves].iter().all(|p| {
            !p.v.is_empty()
                && p.v.iter().all(|v| v.p.is_finite() && v.n.is_finite())
                && p.i.iter().all(|i| (*i as usize) < p.v.len())
        })
    }
}
fn basis(dir: Vec3) -> (Vec3, Vec3) {
    let d = dir.normalize_or_zero();
    let a = if d.y.abs() > 0.95 { Vec3::X } else { Vec3::Y };
    let u = d.cross(a).normalize();
    (u, d.cross(u).normalize())
}

pub fn generate(s: &Settings, lod: u32) -> Tree {
    crate::reference::grow(s, lod.min(2)).expect("validated Solstice species")
}

pub(crate) fn write(path: &Path, data: impl AsRef<[u8]>) -> Result<(), String> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).map_err(|e| e.to_string())?
    }
    fs::write(path, data).map_err(|e| e.to_string())
}
pub fn save_project(s: &Settings, path: &Path) -> Result<(), String> {
    write(path, serde_json::to_vec_pretty(s).unwrap())
}
pub fn load_project(path: &Path) -> Result<Settings, String> {
    let mut value: Value =
        serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    // Projects saved before the Grove generator was removed name its species directly.
    if let Some(species) = value.get_mut("species") {
        if let Some(name) = species.as_str().map(str::to_owned) {
            *species = json!(match name.as_str() {
                "oak" => "solstice-oak",
                "birch" => "solstice-birch",
                "pine" => "solstice-fir",
                other => other,
            });
        }
    }
    let s: Settings = serde_json::from_value(value).map_err(|e| e.to_string())?;
    s.validate()?;
    Ok(s)
}
pub fn export(s: &Settings, lod: u32, format: &str, path: &Path) -> Result<Value, String> {
    s.validate()?;
    let tree = generate(s, lod);
    match format {
        "obj" => export_obj(&tree, path, s)?,
        "gltf" => export_gltf(&tree, path, s)?,
        _ => return Err("Format must be obj or gltf".into()),
    };
    Ok(json!({"path":path,"settings":s,"stats":tree.stats()}))
}
fn export_obj(t: &Tree, path: &Path, s: &Settings) -> Result<(), String> {
    use std::fmt::Write as _;
    let mtl = path.with_extension("mtl");
    let mut text = format!(
        "# Grove Studio | meters | Y up | seed generated\nmtllib {}\n",
        mtl.file_name().unwrap().to_string_lossy()
    );
    writeln!(
        text,
        "# species {} | seed {} | LOD {}",
        s.species, s.seed, t.lod
    )
    .unwrap();
    let mut offset = 1;
    for (name, p) in [("Bark", &t.bark), ("Foliage", &t.leaves)] {
        writeln!(text, "o {name}\nusemtl {name}").unwrap();
        for v in &p.v {
            writeln!(
                text,
                "v {:.6} {:.6} {:.6} {:.4} {:.4} {:.4}",
                v.p.x, v.p.y, v.p.z, v.c[0], v.c[1], v.c[2]
            )
            .unwrap()
        }
        for v in &p.v {
            writeln!(text, "vt {:.5} {:.5}", v.uv.x, v.uv.y).unwrap()
        }
        for v in &p.v {
            writeln!(text, "vn {:.5} {:.5} {:.5}", v.n.x, v.n.y, v.n.z).unwrap()
        }
        for tri in p.i.chunks(3) {
            let a = tri[0] + offset;
            let b = tri[1] + offset;
            let c = tri[2] + offset;
            writeln!(text, "f {a}/{a}/{a} {b}/{b}/{b} {c}/{c}/{c}").unwrap()
        }
        offset += p.v.len() as u32;
    }
    let bark_path = path.with_extension("bark.png");
    write(&bark_path, bark_texture(s))?;
    let (height, normal) = relief_textures(s);
    let height_path = path.with_extension("bark-height.png");
    let normal_path = path.with_extension("bark-normal.png");
    write(&height_path, height)?;
    write(&normal_path, normal)?;
    let materials = format!(
        "newmtl Bark\nKd 1 1 1\nmap_Kd {}\nbump -bm {} {}\ndisp {}\nnorm {}\nnewmtl Foliage\nKd 1 1 1\n",
        bark_path.file_name().unwrap().to_string_lossy(), s.parallax_depth,
        height_path.file_name().unwrap().to_string_lossy(),
        height_path.file_name().unwrap().to_string_lossy(),
        normal_path.file_name().unwrap().to_string_lossy(),
    );
    write(&mtl, materials)?;
    write(path, text)
}
fn export_gltf(t: &Tree, path: &Path, s: &Settings) -> Result<(), String> {
    let mut bin = Vec::<u8>::new();
    let mut views = vec![];
    let mut access = vec![];
    let mut primitives = vec![];
    for (material, p) in [&t.bark, &t.leaves].iter().enumerate() {
        let mut attrs = serde_json::Map::new();
        let tangents = p.tangents();
        for (semantic, kind, n) in [
            ("POSITION", "VEC3", 3),
            ("NORMAL", "VEC3", 3),
            ("TANGENT", "VEC4", 4),
            ("COLOR_0", "VEC4", 4),
            ("TEXCOORD_0", "VEC2", 2),
            ("_WIND", "VEC2", 2),
        ] {
            let offset = bin.len();
            let mut lo = vec![f32::MAX; n];
            let mut hi = vec![f32::MIN; n];
            for (vertex, v) in p.v.iter().enumerate() {
                let data = match semantic {
                    "POSITION" => vec![v.p.x, v.p.y, v.p.z],
                    "NORMAL" => vec![v.n.x, v.n.y, v.n.z],
                    "TANGENT" => tangents[vertex].to_array().to_vec(),
                    "COLOR_0" => v.c.to_vec(),
                    "_WIND" => wind_weights(v, s.height, material == 1).to_vec(),
                    _ => vec![v.uv.x, v.uv.y],
                };
                for (i, f) in data.iter().enumerate() {
                    bin.extend(f.to_le_bytes());
                    lo[i] = lo[i].min(*f);
                    hi[i] = hi[i].max(*f)
                }
            }
            let view = views.len();
            views.push(json!({"buffer":0,"byteOffset":offset,"byteLength":bin.len()-offset,"target":34962}));
            let id = access.len();
            access.push(json!({"bufferView":view,"componentType":5126,"count":p.v.len(),"type":kind,"min":lo,"max":hi}));
            attrs.insert(semantic.into(), json!(id));
        }
        let offset = bin.len();
        for i in &p.i {
            bin.extend(i.to_le_bytes())
        }
        let view = views.len();
        views.push(
            json!({"buffer":0,"byteOffset":offset,"byteLength":bin.len()-offset,"target":34963}),
        );
        let index = access.len();
        access.push(
            json!({"bufferView":view,"componentType":5125,"count":p.i.len(),"type":"SCALAR"}),
        );
        primitives.push(json!({"attributes":attrs,"indices":index,"material":material}));
    }
    let buffer = path.with_extension("bin");
    let buffer_uri = buffer
        .file_name()
        .unwrap()
        .to_string_lossy()
        .as_bytes()
        .iter()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(b) {
                (*b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect::<String>();
    let mut images = vec![];
    let mut texture_defs = vec![];
    let (height, normal) = relief_textures(s);
    let height_index = 1;
    for data in [bark_texture(s), height, normal] {
        while bin.len() % 4 != 0 {
            bin.push(0);
        }
        let offset = bin.len();
        bin.extend_from_slice(data);
        let view = views.len();
        views.push(json!({"buffer":0,"byteOffset":offset,"byteLength":data.len()}));
        images.push(json!({"bufferView":view,"mimeType":"image/png"}));
        texture_defs.push(json!({"source":images.len()-1,"sampler":0}));
    }
    let mut materials = json!([
        {"name":"Bark","pbrMetallicRoughness":{"baseColorFactor":[1,1,1,1],"baseColorTexture":{"index":0},"metallicFactor":0,"roughnessFactor":0.92}},
        {"name":"Foliage","doubleSided":true,"pbrMetallicRoughness":{"baseColorFactor":[1,1,1,1],"metallicFactor":0,"roughnessFactor":0.85}}
    ]);
    materials[0]["normalTexture"] = json!({"index":height_index+1});
    materials[0]["extras"] = json!({"parallax":{"heightTexture":{"index":height_index,"texCoord":0},"scale":s.parallax_depth,"layers":16,"heightConvention":"white is raised; depth = 1 - height","requiresEngineShader":true}});
    let doc = json!({"asset":{"version":"2.0","generator":"Grove Studio"},"scene":0,"scenes":[{"nodes":[0]}],"nodes":[{"mesh":0,"name":format!("Tree_LOD{}",t.lod)}],"meshes":[{"primitives":primitives}],"materials":materials,"images":images,"textures":texture_defs,"samplers":[{"magFilter":9729,"minFilter":9987,"wrapS":10497,"wrapT":10497}],"buffers":[{"uri":buffer_uri,"byteLength":bin.len()}],"bufferViews":views,"accessors":access,"extras":{"units":"meters","upAxis":"Y","lod":t.lod,"settings":s,"wind":{"attribute":"_WIND","components":["normalized height squared","foliage flutter weight"],"requiresEngineShader":true}}});
    write(&buffer, bin)?;
    write(path, serde_json::to_vec_pretty(&doc).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trees_are_deterministic_finite_and_lods_reduce() {
        for name in crate::reference::IDS.iter().copied().step_by(6) {
            let s = Settings::preset(name).unwrap();
            let a = generate(&s, 0);
            let b = generate(&s, 0);
            assert!(a.valid(), "{name}");
            assert_eq!(a.bark.v[30].p, b.bark.v[30].p, "{name}");
            assert_eq!(a.leaves.v[100].p, b.leaves.v[100].p, "{name}");
            let low = generate(&s, 2);
            assert!(low.valid());
            assert!(
                low.triangles() < a.triangles() * 3 / 4,
                "{name}: LOD 0 has {} triangles, LOD 2 has {}",
                a.triangles(),
                low.triangles()
            );
            assert!(a.triangles() < 900_000, "{name}");
            assert!(a.bounds().1.y > s.height * 0.85, "{name}");
        }
    }
    #[test]
    fn invalid_settings_rejected() {
        let mut s = Settings::default();
        s.height = f32::NAN;
        assert!(s.validate().is_err());
        s.height = 8.;
        s.branches = 0;
        assert!(s.validate().is_err());
        s.branches = 11;
        s.species = "oak".into();
        assert!(s.validate().is_err());
        assert!(Settings::preset("oak").is_err());
    }
    #[test]
    fn winter_drops_leaves_without_changing_the_wood() {
        for species in crate::reference::IDS.iter().copied().step_by(6) {
            for lod in 0..3 {
                let mut s = Settings::preset(species).unwrap();
                let summer = generate(&s, lod);
                s.season = 3;
                let winter = generate(&s, lod);
                assert!(winter.valid());
                assert!(summer
                    .bark
                    .v
                    .iter()
                    .zip(&winter.bark.v)
                    .all(|(a, b)| a.p == b.p));
                if crate::reference::evergreen_id(species) {
                    assert_eq!(summer.leaves.v.len(), winter.leaves.v.len());
                } else {
                    assert!(
                        winter.leaves.v.len() > 0
                            && winter.leaves.v.len() < summer.leaves.v.len() / 20,
                        "{species} lod {lod}"
                    );
                }
                let tangents = winter.bark.tangents();
                assert!(tangents
                    .iter()
                    .zip(&winter.bark.v)
                    .all(|(t, v)| t.is_finite() && t.truncate().dot(v.n).abs() < 1e-4));
            }
        }
    }
    #[test]
    fn legacy_species_names_migrate_when_a_project_is_opened() {
        let path = std::env::temp_dir().join("grove-legacy-project-test.json");
        std::fs::write(&path, r#"{"species":"pine","seed":7,"height":11.0}"#).unwrap();
        let s = load_project(&path).unwrap();
        assert_eq!(s.species, "solstice-fir");
        assert_eq!(s.seed, 7);
        let _ = std::fs::remove_file(&path);
    }
}
