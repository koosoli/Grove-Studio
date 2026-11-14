//! Grass authoring: Solstice meadow geometry with bounded LODs and OpenPoseidon blade layers.
use glam::Vec3;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use solstice_builder::{Mesh, Vertex};
use solstice_procgen::foliage::grass::Meadow;
use std::{
    io::Cursor,
    path::{Path, PathBuf},
};

pub const PRESETS: [&str; 5] = ["meadow", "lawn", "dry", "tall", "verge"];
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub preset: String,
    pub style: String,
    pub seed: u32,
    pub size: f32,
    pub density: f32,
    pub height: f32,
    pub width: f32,
    pub lean: f32,
    pub dry: f32,
    pub patchiness: f32,
    pub weeds: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            preset: "meadow".into(),
            style: "solstice".into(),
            seed: 0,
            size: 2.,
            density: 0.75,
            height: 1.,
            width: 1.,
            lean: 1.,
            dry: 0.08,
            patchiness: 0.2,
            weeds: true,
        }
    }
}
impl Settings {
    pub fn preset(name: &str) -> Result<Self, String> {
        let meadow = Meadow::by_name(name).ok_or("Unknown grass preset")?;
        Ok(Self {
            preset: name.into(),
            dry: meadow.dry,
            ..Self::default()
        })
    }
    /// A safe slider ceiling at the current patch size, with one blade of rounding margin.
    pub fn max_density(&self) -> f32 {
        let Some(m) = Meadow::by_name(&self.preset) else { return 0.05; };
        (29999. / (self.size*self.size*(m.tufts_per_square_metre*m.blades_per_tuft.1 as f32+m.undergrowth_per_square_metre))).min(1.5)
    }
    pub fn validate(&self) -> Result<(), String> {
        if !["solstice", "poseidon"].contains(&self.style.as_str()) {
            return Err("Grass style is solstice or poseidon".into());
        }
        if !PRESETS.contains(&self.preset.as_str()) {
            return Err("Unknown grass preset".into());
        }
        for (name, v, lo, hi) in [
            ("size", self.size, 0.25, 4.),
            ("density", self.density, 0.05, 1.5),
            ("height", self.height, 0.25, 2.),
            ("width", self.width, 0.5, 3.),
            ("lean", self.lean, 0., 1.5),
            ("dry", self.dry, 0., 1.),
            ("patchiness", self.patchiness, 0., 1.),
        ] {
            if !v.is_finite() || v < lo || v > hi {
                return Err(format!("Grass {name} must be {lo}..{hi}"));
            }
        }
        // Bound generation before allocating: weeds add their own mesh complexity.
        let m = Meadow::by_name(&self.preset).unwrap();
        let blades = self.size
            * self.size
            * self.density
            * (m.tufts_per_square_metre * m.blades_per_tuft.1 as f32
                + m.undergrowth_per_square_metre);
        if blades > 30000. {
            return Err("Grass patch too dense: reduce size or density".into());
        }
        Ok(())
    }
    pub fn meadow(&self, lod: u32) -> Result<Meadow, String> {
        self.validate()?;
        let mut m = Meadow::by_name(&self.preset).unwrap();
        let keep = [1., 0.4, 0.15][lod.min(2) as usize];
        if self.style == "poseidon" {
            m.blades_per_tuft = (6, 6);
            m.normals_up = 0.45;
        }
        m.size = self.size;
        m.seed = self.seed;
        m.tufts_per_square_metre *= self.density * keep;
        m.undergrowth_per_square_metre *= self.density * keep;
        m.height = (m.height.0 * self.height, m.height.1 * self.height);
        m.undergrowth_height = (
            m.undergrowth_height.0 * self.height,
            m.undergrowth_height.1 * self.height,
        );
        m.width = (
            m.width.0 * self.width / keep.sqrt(),
            m.width.1 * self.width / keep.sqrt(),
        );
        m.lean = (m.lean.0 * self.lean, m.lean.1 * self.lean);
        m.dry = self.dry;
        if !self.weeds {
            m.weeds.clear();
        }
        for weed in &mut m.weeds {
            weed.per_square_metre *= self.density * keep;
            weed.length=(weed.length.0*self.height,weed.length.1*self.height);
            weed.stalk=(weed.stalk.0*self.height,weed.stalk.1*self.height);
            weed.flower_height=(weed.flower_height.0*self.height,weed.flower_height.1*self.height);
            weed.flower_size*=self.height;
        }
        Ok(m)
    }
    pub fn height_hint(&self) -> f32 {
        Meadow::by_name(&self.preset)
            .map(|m| m.height.1 * self.height)
            .unwrap_or(1.)
    }
    pub fn mesh(&self, lod: u32) -> Result<Mesh, String> {
        let mut mesh = self.meadow(lod)?.mesh();
        for v in &mut mesh.vertices {
            v.position -= Vec3::new(self.size * 0.5, 0., self.size * 0.5);
        }
        if self.patchiness > 0.0 && !mesh.vertices.is_empty() {
            // Keep connected blades/plants intact when opening metre-scale gaps. This is
            // an authoring analogue of the engine's spatial density rejection.
            let mut parent: Vec<usize> = (0..mesh.vertices.len()).collect();
            fn find(p: &mut [usize], mut i: usize) -> usize {
                while p[i] != i {
                    p[i] = p[p[i]];
                    i = p[i];
                }
                i
            }
            for t in mesh.indices.chunks_exact(3) {
                let a = find(&mut parent, t[0] as usize);
                for &b in &t[1..] {
                    let b = find(&mut parent, b as usize);
                    parent[b] = a;
                }
            }
            let mut roots = std::collections::HashMap::<usize, Vec3>::new();
            for (i, v) in mesh.vertices.iter().enumerate() {
                let id = find(&mut parent, i);
                roots
                    .entry(id)
                    .and_modify(|p| {
                        if v.position.y < p.y {
                            *p = v.position
                        }
                    })
                    .or_insert(v.position);
            }
            let keep: std::collections::HashSet<usize> = roots
                .into_iter()
                .filter(|(_, p)| {
                    solstice_procgen::noise::fbm(Vec3::new(p.x * 1.3, 3.7, p.z * 1.3), 3, self.seed)
                        > -0.65 + self.patchiness * 0.9
                })
                .map(|(i, _)| i)
                .collect();
            let mut remap = std::collections::HashMap::new();
            let mut filtered = Mesh::default();
            for t in mesh.indices.chunks_exact(3) {
                if !keep.contains(&find(&mut parent, t[0] as usize)) {
                    continue;
                }
                for &i in t {
                    let index = *remap.entry(i).or_insert_with(|| {
                        let n = filtered.vertices.len() as u32;
                        filtered.vertices.push(mesh.vertices[i as usize]);
                        n
                    });
                    filtered.indices.push(index);
                }
            }
            mesh = filtered;
        }
        if mesh.indices.is_empty() {
            return Err("Grass patch is empty: reduce patchiness or change seed".into());
        }
        Ok(mesh)
    }
    pub fn report(&self) -> Result<Value, String> {
        let mut levels = Vec::new();
        for lod in 0..3 {
            let m = self.mesh(lod)?;
            let min=m.vertices.iter().fold(Vec3::splat(f32::INFINITY),|a,v|a.min(v.position));
            let max=m.vertices.iter().fold(Vec3::splat(f32::NEG_INFINITY),|a,v|a.max(v.position));
            levels.push(json!({"lod":lod,"triangles":m.indices.len()/3,"vertices":m.vertices.len(),"vertexBytes":m.vertices.len()*std::mem::size_of::<Vertex>(),"indexBytes":m.indices.len()*4,"bounds":{"min":min.to_array(),"max":max.to_array()},"height_m":max.y-min.y,"above_ground_m":max.y}));
        }
        Ok(
            json!({"settings":self,"levels":levels,"units":"metres","wind":"preview only; runtime grass placement and interaction are engine-owned"}),
        )
    }
    pub fn atlas(&self) -> Result<solstice_asset::TextureLayer, String> {
        Ok(self.meadow(0)?.atlas(self.seed).bake(256))
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        crate::tree::write(path, serde_json::to_vec_pretty(self).unwrap())
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        let s: Self = serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        s.validate()?;
        Ok(s)
    }
}

pub fn export(s: &Settings, path: &Path, format: &str, lod: u32) -> Result<Value, String> {
    let m = s.mesh(lod)?;
    let layer = s.atlas()?;
    let png = path.with_extension("grass.png");
    let im = image::RgbaImage::from_raw(256, 256, layer.color_mips[0].clone()).unwrap();
    let mut encoded = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(im)
        .write_to(&mut encoded, image::ImageOutputFormat::Png)
        .map_err(|e| e.to_string())?;
    match format {
        "obj" => {
            use std::fmt::Write;
            let mtl = path.with_extension("mtl");
            let mut out = format!(
                "# Grove grass, metres, Y up\nmtllib {}\no Grass\nusemtl Grass\n",
                mtl.file_name().unwrap().to_string_lossy()
            );
            for v in &m.vertices {
                writeln!(out, "v {} {} {}", v.position.x, v.position.y, v.position.z).unwrap();
            }
            for v in &m.vertices {
                writeln!(out, "vt {} {}", v.uv.x, v.uv.y).unwrap();
            }
            for v in &m.vertices {
                writeln!(out, "vn {} {} {}", v.normal.x, v.normal.y, v.normal.z).unwrap();
            }
            for t in m.indices.chunks_exact(3) {
                let [a, b, c] = [t[0] + 1, t[1] + 1, t[2] + 1];
                writeln!(out, "f {a}/{a}/{a} {b}/{b}/{b} {c}/{c}/{c}").unwrap();
            }
            crate::tree::write(&png, encoded.into_inner())?;
            crate::tree::write(
                &mtl,
                format!(
                    "newmtl Grass\nKd 1 1 1\nmap_Kd {}\n",
                    png.file_name().unwrap().to_string_lossy()
                ),
            )?;
            crate::tree::write(path, out)?;
        }
        "gltf" => {
            let mut bytes = Vec::new();
            let mut views = Vec::new();
            let mut access = Vec::new();
            for (kind, n) in [
                ("POSITION", 3),
                ("NORMAL", 3),
                ("TEXCOORD_0", 2),
                ("_WIND", 2),
            ] {
                let start = bytes.len();
                let mut lo = vec![f32::MAX; n];
                let mut hi = vec![f32::MIN; n];
                for v in &m.vertices {
                    let values = match kind {
                        "POSITION" => v.position.to_array().to_vec(),
                        "NORMAL" => v.normal.to_array().to_vec(),
                        "TEXCOORD_0" => v.uv.to_array().to_vec(),
                        _ => vec![(v.position.y / (s.height * 0.6)).clamp(0., 1.).powi(2), 1.],
                    };
                    for (i, v) in values.iter().enumerate() {
                        bytes.extend(v.to_le_bytes());
                        lo[i] = lo[i].min(*v);
                        hi[i] = hi[i].max(*v);
                    }
                }
                views.push(json!({"buffer":0,"byteOffset":start,"byteLength":bytes.len()-start,"target":34962}));
                access.push(json!({"bufferView":views.len()-1,"componentType":5126,"count":m.vertices.len(),"type":if n==3{"VEC3"}else{"VEC2"},"min":lo,"max":hi}));
            }
            let start = bytes.len();
            for i in &m.indices {
                bytes.extend(i.to_le_bytes());
            }
            views.push(json!({"buffer":0,"byteOffset":start,"byteLength":bytes.len()-start,"target":34963}));
            access.push(json!({"bufferView":4,"componentType":5125,"count":m.indices.len(),"type":"SCALAR"}));
            let start = bytes.len();
            bytes.extend(encoded.into_inner());
            views.push(json!({"buffer":0,"byteOffset":start,"byteLength":bytes.len()-start}));
            let bin = path.with_extension("bin");
            let uri = bin
                .file_name()
                .unwrap()
                .to_string_lossy()
                .bytes()
                .map(|b| {
                    if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                        (b as char).to_string()
                    } else {
                        format!("%{b:02X}")
                    }
                })
                .collect::<String>();
            let doc = json!({"asset":{"version":"2.0","generator":"Grove Studio grass"},"scene":0,"scenes":[{"nodes":[0]}],"nodes":[{"mesh":0}],"meshes":[{"primitives":[{"attributes":{"POSITION":0,"NORMAL":1,"TEXCOORD_0":2,"_WIND":3},"indices":4,"material":0}]}],"buffers":[{"uri":uri,"byteLength":bytes.len()}],"bufferViews":views,"accessors":access,"images":[{"bufferView":5,"mimeType":"image/png"}],"samplers":[{"magFilter":9729,"minFilter":9987,"wrapS":33071,"wrapT":33071}],"textures":[{"source":0,"sampler":0}],"materials":[{"name":"Grass","doubleSided":true,"pbrMetallicRoughness":{"baseColorTexture":{"index":0},"metallicFactor":0,"roughnessFactor":0.75}}],"extras":{"grass":s,"lod":lod,"wind":{"requiresEngineShader":true}}});
            crate::tree::write(&bin, bytes)?;
            crate::tree::write(path, serde_json::to_vec_pretty(&doc).unwrap())?;
        }
        _ => {
            return Err(
                "Grass mesh export supports gltf or obj; use engine bundle for OpenPoseidon".into(),
            )
        }
    }
    Ok(json!({"path":path,"triangles":m.indices.len()/3,"vertices":m.vertices.len()}))
}

/// Matches the eight independent 64x256 layers loaded by OpenPoseidon's existing loader.
/// Geometry, placement, wind and species selection remain owned by the runtime shader.
pub fn export_engine(s: &Settings, dir: &Path) -> Result<Value, String> {
    s.validate()?;
    let m = s.meadow(0)?;
    let atlas = s.atlas()?;
    let columns = m.columns() as usize;
    let colors = &atlas.color_mips[0];
    let mut files: Vec<PathBuf> = Vec::new();
    for layer in 0..8usize {
        let column = if layer < 4 {
            [0, 1, 3, 2][layer]
        } else if columns > 4 {
            4 + ((layer - 4) % ((columns - 4) / 2)) * 2 + usize::from(layer >= 6)
        } else {
            layer % 4
        };
        let im = image::RgbaImage::from_fn(64, 256, |x, y| {
            let u = (column as f32 + (x as f32 + 0.5) / 64.) / columns as f32;
            let i = (((255 - y) as usize * 256) + (u * 256.) as usize).min(256 * 256 - 1) * 4;
            let mut p = [colors[i], colors[i + 1], colors[i + 2], 255];
            if columns == 4 && layer >= 6 {
                let warm = if layer == 6 {
                    [225, 194, 60]
                } else {
                    [232, 223, 214]
                };
                let head = ((1. - y as f32 / 255.) - 0.8).max(0.) * 4.;
                for c in 0..3 {
                    p[c] = (p[c] as f32 * (1. - head) + warm[c] as f32 * head) as u8;
                }
            }
            image::Rgba(p)
        });
        let file = dir.join(format!("assets/grass/meadow-grass-blade-{layer}.png"));
        let mut data = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(im)
            .write_to(&mut data, image::ImageOutputFormat::Png)
            .map_err(|e| e.to_string())?;
        crate::tree::write(&file, data.into_inner())?;
        files.push(file);
    }
    let manifest = json!({"format":"grove-poseidon-grass-textures","version":1,"settings":s,"layers":8,"width":64,"height":256,"order":["grass","grass","dry grass","grass","weed","weed","flower","flower"],"vOrientation":"top is blade tip; bottom is root","engineEnvironment":{"WGR_GRASS_BLADE_DIR":dir.join("assets/grass")},"files":files,"runtime":"OpenPoseidon supplies blade geometry, placement, LOD, wind and interaction; Grove meadow settings are authoring metadata, not automatically imported engine settings"});
    crate::tree::write(
        &dir.join("grass-bundle.json"),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )?;
    crate::tree::write(&dir.join("source.grass"), m.to_text())?;
    s.save(&dir.join("project.json"))?;
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn grass_is_seeded_valid_and_lods_reduce() {
        for name in PRESETS {
            let s = Settings::preset(name).unwrap();
            let a = s.mesh(0).unwrap();
            let b = s.mesh(0).unwrap();
            assert_eq!(a.indices, b.indices);
            assert_eq!(a.vertices[0].position, b.vertices[0].position);
            assert!(a
                .vertices
                .iter()
                .all(|v| v.position.is_finite() && v.normal.is_finite()));
            assert!(s.mesh(2).unwrap().indices.len() < a.indices.len());
        }
    }
    #[test]
    fn gaps_remove_whole_blades_and_remain_seeded() {
        let mut s = Settings::default();
        s.patchiness = 0.;
        let full = s.mesh(0).unwrap();
        s.patchiness = 0.8;
        let patchy = s.mesh(0).unwrap();
        assert!(patchy.indices.len() < full.indices.len());
        assert!(patchy
            .indices
            .iter()
            .all(|i| (*i as usize) < patchy.vertices.len()));
        assert_eq!(patchy.indices, s.mesh(0).unwrap().indices);
    }
    #[test]
    fn limits_and_engine_layout() {
        for preset in PRESETS {
            let mut patch=Settings::preset(preset).unwrap();
            patch.size=4.; patch.density=patch.max_density();
            assert!(patch.validate().is_ok(),"the editor's largest {preset} patch must stay valid");
        }
        let mut s = Settings::default();
        s.size = f32::NAN;
        assert!(s.validate().is_err());
        let s = Settings::default();
        let dir = std::env::temp_dir().join(format!("grove-grass-bundle-{}", std::process::id()));
        let result = export_engine(&s, &dir).unwrap();
        assert_eq!(result["layers"], 8);
        for i in 0..8 {
            let im = image::open(dir.join(format!("assets/grass/meadow-grass-blade-{i}.png")))
                .unwrap()
                .to_rgba8();
            assert_eq!(im.dimensions(), (64, 256));
            assert!(im.pixels().all(|p| p[3] == 255));
            if i < 4 {
                let source = s.atlas().unwrap();
                let columns = s.meadow(0).unwrap().columns() as f32;
                let x = (([0, 1, 3, 2][i] as f32 + 32.5 / 64.) / columns * 256.) as usize;
                assert_eq!(
                    &im.get_pixel(32, 0).0[..3],
                    &source.color_mips[0][(255 * 256 + x) * 4..(255 * 256 + x) * 4 + 3]
                );
            }
        }
        export(&s, &dir.join("patch.gltf"), "gltf", 1).unwrap();
        export(&s, &dir.join("patch.obj"), "obj", 1).unwrap();
        assert_eq!(Settings::load(&dir.join("project.json")).unwrap(), s);
    }
}

pub fn headless(args: &[String]) -> Result<(), String> {
    use crate::agent::option;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--headless" | "--inspect" | "--preview" | "--no-weeds" => i += 1,
            "--grass" | "--style" | "--seed" | "--size" | "--density" | "--height" | "--width"
            | "--lean" | "--dry" | "--patchiness" | "--out" | "--format" | "--lod" | "--config"
            | "--distance" | "--wind-time" | "--wind-strength" | "--wind-direction"
            | "--time-of-day" | "--sun-azimuth" => {
                if i + 1 >= args.len() || args[i + 1].starts_with("--") {
                    return Err(format!("Missing {}", args[i]));
                }
                i += 2;
            }
            other => return Err(format!("Unknown grass option: {other}")),
        }
    }
    let mut s = if let Some(path) = option(args, "--config") {
        Settings::load(Path::new(&path))?
    } else {
        Settings::preset(&option(args, "--grass").ok_or("--grass needs a preset")?)?
    };
    if let Some(style) = option(args, "--style") {
        s.style = style;
    }
    if let Some(seed) = option(args, "--seed") {
        s.seed = seed.parse().map_err(|_| "Invalid grass seed")?;
    }
    for (flag, field) in [
        ("--size", &mut s.size),
        ("--density", &mut s.density),
        ("--height", &mut s.height),
        ("--width", &mut s.width),
        ("--lean", &mut s.lean),
        ("--dry", &mut s.dry),
        ("--patchiness", &mut s.patchiness),
    ] {
        if let Some(v) = option(args, flag) {
            *field = v.parse().map_err(|_| format!("Invalid {flag}"))?;
        }
    }
    if args.iter().any(|a| a == "--no-weeds") {
        s.weeds = false;
    }
    s.validate()?;
    if args.iter().any(|a| a == "--inspect") {
        println!("{}", s.report()?);
        return Ok(());
    }
    let out = PathBuf::from(option(args, "--out").unwrap_or_else(|| "exports/grass.gltf".into()));
    if args.iter().any(|a| a == "--preview") {
        let mut request = json!({"grass":s});
        for (flag, key) in [
            ("--distance", "distance"),
            ("--wind-time", "windTime"),
            ("--wind-strength", "windStrength"),
            ("--wind-direction", "windDirection"),
            ("--time-of-day", "timeOfDay"),
            ("--sun-azimuth", "sunAzimuth"),
        ] {
            if let Some(v) = option(args, flag) {
                request[key] = json!(v.parse::<f64>().map_err(|_| format!("Invalid {flag}"))?);
            }
        }
        if let Some(lod) = option(args, "--lod") {
            request["lod"] = json!(lod);
        }
        println!(
            "{}",
            crate::agent::capture_preview(crate::tree::Settings::default(), &request, &out)?
        );
        return Ok(());
    }
    let format = option(args, "--format").unwrap_or_else(|| {
        out.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("gltf")
            .into()
    });
    if format == "engine" {
        println!("{}", export_engine(&s, &out)?);
        return Ok(());
    }
    let lod = option(args, "--lod").unwrap_or_else(|| "all".into());
    let lods = match lod.as_str() {
        "all" => vec![0, 1, 2],
        "0" => vec![0],
        "1" => vec![1],
        "2" => vec![2],
        _ => return Err("Grass LOD is 0, 1, 2 or all".into()),
    };
    let mut files = Vec::new();
    for lod in lods.iter().copied() {
        let path = if lods.len() > 1 {
            out.with_file_name(format!(
                "{}_lod{lod}.{format}",
                out.file_stem().unwrap_or_default().to_string_lossy()
            ))
        } else {
            out.clone()
        };
        files.push(export(&s, &path, &format, lod)?);
    }
    println!("{}", json!({"files":files}));
    Ok(())
}
