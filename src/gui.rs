use crate::{
    agent, preview,
    tree::{self, Part, Settings, Tree},
};
use macroquad::prelude::*;
use serde_json::json;
use std::{
    cell::Cell,
    fs,
    path::{Path, PathBuf},
};

const W: f32 = 1440.;
const H: f32 = 900.;
// Distance change per wheel notch, as a fraction (0.05 is about 5 %; 14 notches double the range).
const ZOOM_PER_NOTCH: f32 = 0.05;
/// Smooth wind starts/stops and keep phase continuous when the speed slider changes.
#[derive(Default)]
struct WindPlayback { phase: f32, envelope: f32 }
impl WindPlayback {
    fn step(&mut self, enabled: bool, speed: f32, springiness: f32, dt: f32) {
        let dt = dt.clamp(0., 0.2);
        let active = enabled && speed > 0.001;
        let target = if active { 1. } else { 0. };
        let tau = if active { 0.2 } else { 0.45 + springiness * 1.1 };
        self.envelope += (target - self.envelope) * (1. - (-dt / tau).exp());
        if self.envelope > 0.001 {
            self.phase = (self.phase + dt * if active { speed } else { speed.max(0.6) }) % 10000.;
        } else if !active { self.envelope = 0.; }
    }
}

#[cfg(test)]
mod wind_playback_tests {
    use super::WindPlayback;
    #[test]
    fn wind_starts_smoothly_and_rings_down_without_restarting_phase() {
        let mut wind = WindPlayback::default();
        wind.step(true, 1., 0.8, 0.1);
        assert!(wind.envelope > 0. && wind.envelope < 1.);
        let previous_phase = wind.phase;
        wind.step(false, 1., 0.8, 0.1);
        assert!(wind.envelope > 0. && wind.phase > previous_phase);
        for _ in 0..200 { wind.step(false, 0., 0.8, 0.1); }
        assert_eq!(wind.envelope, 0.);
    }
}
// Species labels, in the order of preview::SPECIES.
const LIBRARY_PAGE: usize = 5;
const FOREST_PAGE: usize = 7;
const TREE_VERTEX: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
attribute vec4 normal;
uniform mat4 Model;
uniform mat4 Projection;
uniform vec4 Wind;
varying lowp vec2 uv;
varying lowp vec4 color;
varying vec3 worldP;
varying vec3 worldN;
varying vec3 worldT;
varying vec3 worldB;
void main() {
    vec3 p = position;
    // Poseidon vegetation sway at the reference wind speed.
    float weight = pow(clamp(p.y / 12.0, 0.0, 3.0), 1.6);
    vec2 dir = vec2(sin(Wind.z), cos(Wind.z));
    vec2 side = vec2(-dir.y, dir.x);
    float t = Wind.x;
    float bend = sin(t * 0.9) * 0.65 + sin(t * 1.63) * 0.35;
    p.xz += Wind.y * 0.35 * weight * dir * bend;
    gl_Position = Projection * Model * vec4(p, 1.0);
    uv = texcoord;
    color = color0 / 255.0;
    worldP = p;
    worldN = normal.xyz;
    vec3 reference = abs(normal.y) > 0.95 ? vec3(1.0,0.0,0.0) : vec3(0.0,1.0,0.0);
    vec3 u = normalize(cross(normal.xyz, reference));
    vec3 v = cross(normal.xyz, u);
    float angle = mod(normal.w, 6.2831853);
    worldT = u*cos(angle)+v*sin(angle);
    worldB = cross(normal.xyz, worldT)*(normal.w >= 6.2831853 ? -1.0 : 1.0);
}"#;
const TREE_FRAGMENT: &str = r#"#version 100
precision highp float;
varying lowp vec2 uv;
varying lowp vec4 color;
uniform sampler2D Texture;
uniform sampler2D HeightMap;
uniform sampler2D NormalMap;
uniform vec3 CameraPos;
uniform vec4 Relief;
varying vec3 worldP;
varying vec3 worldN;
varying vec3 worldT;
varying vec3 worldB;
void main() {
    vec2 mapped = uv;
    vec3 N = normalize(worldN);
    vec3 T = normalize(worldT-N*dot(N,worldT));
    vec3 B = normalize(worldB);
    if (Relief.y > 0.5) {
        vec3 view = normalize(CameraPos-worldP);
        vec3 eye = vec3(dot(view,T),dot(view,B),dot(view,N));
        vec2 stepUV = eye.xy / max(abs(eye.z), 0.28) * Relief.x / 16.0;
        float layer = 0.0;
        float surface = 1.0-texture2D(HeightMap,fract(mapped)).r;
        vec2 previous = mapped;
        float before = surface;
        for (int i=0; i<16; i++) {
            if (layer < surface) {
                previous = mapped;
                before = surface-layer;
                mapped -= stepUV;
                layer += 0.0625;
                surface = 1.0-texture2D(HeightMap,fract(mapped)).r;
            }
        }
        float after = surface-layer;
        float blend = clamp(after/(after-before-0.00001),0.0,1.0);
        mapped = mix(mapped,previous,blend);
        vec3 detail = texture2D(NormalMap,fract(mapped)).xyz*2.0-1.0;
        N = normalize(T*detail.x+B*detail.y+N*detail.z);
        mapped = fract(mapped);
    }
    lowp vec4 sampleColor = texture2D(Texture, mapped);
    if (sampleColor.a < 0.5) discard;
    float light = Relief.y > 0.5 ? 0.48+0.52*max(dot(N,normalize(vec3(-0.5,0.7,0.5))),0.0) : 1.0;
    gl_FragColor = vec4(sampleColor.rgb * color.rgb * light, 1.0);
}"#;

struct Textures {
    species: String,
    paths: [Option<String>; 3],
    weathering: f32,
    bark: Texture2D,
    height: Texture2D,
    normal: Texture2D,
}

fn solstice_view(view: u32) -> bool { view != 1 }
fn preview_leaves(view: u32, enabled: bool) -> bool { enabled && view != 2 }

#[cfg(test)]
mod viewport_routing_tests {
    #[test]
    fn structure_keeps_the_live_renderer_and_hides_foliage() {
        assert!(super::solstice_view(0));
        assert!(!super::solstice_view(1));
        assert!(super::solstice_view(2));
        assert!(!super::preview_leaves(2, true));
        assert!(super::preview_leaves(0, true));
    }
    #[test]
    fn library_search_finds_both_catalogs_by_name_and_id() {
        assert!(super::library_matches("OAK", true).iter().any(|v| v.0 == "solstice-oak"));
        assert!(super::library_matches("smrk", false).iter().any(|v| v.0.contains("smrk")));
        assert!(super::library_matches("not-a-real-species", false).is_empty());
        assert_eq!(super::library_matches("", false).len(), 14);
    }
}
impl Textures {
    fn new(settings: &Settings) -> Self {
        fn load(bytes: &[u8]) -> Texture2D {
            let texture = Texture2D::from_file_with_format(bytes, Some(ImageFormat::Png));
            texture.set_filter(FilterMode::Linear);
            texture
        }
        let mut fallback = settings.clone();
        fallback.bark_color_path = None; fallback.bark_height_path = None; fallback.bark_normal_path = None;
        let (height, normal) = tree::relief_textures(settings).unwrap_or_else(|_| tree::relief_textures(&fallback).expect("generated textures"));
        let bark = tree::bark_texture(settings).unwrap_or_else(|_| tree::bark_texture(&fallback).expect("generated bark"));
        Self {
            species: settings.species.clone(),
            paths: [settings.bark_color_path.clone(), settings.bark_height_path.clone(), settings.bark_normal_path.clone()],
            weathering: settings.bark_weathering,
            bark: load(&bark),
            height: load(&height),
            normal: load(&normal),
        }
    }
    fn apply(&self, model: &mut Model) {
        for mesh in &mut model.bark {
            mesh.texture = Some(self.bark.clone());
        }
    }
}
thread_local! {
    static DARK_MODE: Cell<bool> = const { Cell::new(true) };
}

fn theme_path() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME"))
        .map(|root| PathBuf::from(root).join("Grove Studio").join("theme"))
}

fn dark_mode() -> bool {
    DARK_MODE.with(Cell::get)
}

fn toggle_theme() -> Result<(), String> {
    let dark = !dark_mode();
    DARK_MODE.with(|mode| mode.set(dark));
    if let Some(path) = theme_path() {
        tree::write(&path, if dark { "dark" } else { "light" })?;
    }
    Ok(())
}

fn rgb(s: u32) -> Color {
    Color::from_rgba((s >> 16) as u8, (s >> 8) as u8, s as u8, 255)
}
fn col(s: u32) -> Color {
    // Keep colors that describe the scene or convey a specific status.
    if matches!(
        s,
        0x527a37
            | 0xba7639
            | 0x93b865
            | 0x9ba6aa
            | 0x648558
            | 0xaa7960
            | 0x688c9c
            | 0xad553f
            | 0xb28b42
    ) {
        return rgb(s);
    }
    let themed = if dark_mode() {
        match s {
            0x172a22 => 0x16171a,
            0x21372b => 0x1a1c20,
            0x253c32 => 0xe8eaed,
            0x356b4c => 0xb9c0ca,
            0x5c7055 | 0x77866a | 0x7b877e | 0x819176 => 0xa9adb4,
            0xdde3d9 | 0xd4dfc9 => 0x41454d,
            0xfafbf7 | 0xf6f7f2 | 0xf3f5ee | 0xf4f6f0 => 0x202226,
            0xf8faf4 | 0xf7f9f2 | 0xf7f8f4 | 0xf2f5ec => 0x282b30,
            0xe9eee1 | 0xe8eee1 | 0xeaf0e2 | 0xeaf0e5 | 0xebeee5 | 0xeef1e9 | 0xeff2e9
            | 0xf0f2eb => 0x2c2f35,
            0xe8eee4 | 0xedefe6 => 0x363a41,
            0xe0ead6 | 0xe1e9d8 | 0xe1eadb | 0xe5eddd | 0xffffff => 0x41464e,
            0xfcfdf9 => 0x22252a,
            _ => neutral(s),
        }
    } else {
        neutral(s)
    };
    rgb(themed)
}

fn neutral(s: u32) -> u32 {
    let r = (s >> 16) & 255;
    let g = (s >> 8) & 255;
    let b = s & 255;
    let lightness = (54 * r + 183 * g + 19 * b + 128) / 256;
    lightness * 0x010101
}

fn ink() -> Color {
    col(0x253c32)
}
fn muted() -> Color {
    col(0x7b877e)
}
fn line() -> Color {
    col(0xdde3d9)
}
fn green() -> Color {
    col(0x356b4c)
}
fn mint() -> Color {
    col(0xb9dd9e)
}

struct Ui {
    font: Option<Font>,
    mouse: Vec2,
    enabled: bool,
    scale: f32,
    hud_markers: Cell<u8>,
}
impl Ui {
    fn new(font: Option<Font>) -> Self {
        Self {
            font,
            mouse: Vec2::ZERO,
            enabled: true,
            scale: 1.,
            hud_markers: Cell::new(0),
        }
    }
    fn text(&self, s: &str, x: f32, y: f32, size: f32, c: Color) {
        let marker = match s {
            "GAME MESH" | "SOURCE MESH" => 1,
            "Auto LOD   0" => 2,
            "VARIATION SEED" => 4,
            _ => 0,
        };
        self.hud_markers.set(self.hud_markers.get() | marker);
        draw_text_ex(
            s,
            x,
            y,
            TextParams {
                font: self.font.as_ref(),
                font_size: size as u16,
                color: c,
                ..Default::default()
            },
        );
    }
    fn width(&self, s: &str, size: f32) -> f32 {
        measure_text(s, self.font.as_ref(), size as u16, 1.).width
    }
    fn center(&self, s: &str, r: Rect, size: f32, c: Color) {
        self.text(
            s,
            r.x + (r.w - self.width(s, size)) / 2.,
            r.y + r.h / 2. + size * 0.32,
            size,
            c,
        )
    }
    fn over(&self, r: Rect) -> bool {
        self.enabled && r.contains(self.mouse)
    }
    fn button(&self, s: &str, r: Rect, style: u8) -> bool {
        let over = self.over(r);
        let bg = match style {
            1 => {
                if over {
                    col(0xc9edb0)
                } else {
                    mint()
                }
            }
            2 => {
                if over {
                    col(0xe1eadb)
                } else {
                    col(0xeaf0e5)
                }
            }
            3 => {
                if over {
                    col(0x3c5147)
                } else {
                    col(0x273a31)
                }
            }
            _ => {
                if over {
                    col(0xe8eee4)
                } else {
                    col(0xf7f8f4)
                }
            }
        };
        rounded(r, 7., bg);
        if style == 0 {
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 1., line());
        }
        self.center(
            s,
            r,
            14.,
            match style {
                1 => rgb(0x26282c),
                3 => rgb(0xe8eaed),
                _ => ink(),
            },
        );
        over && is_mouse_button_pressed(MouseButton::Left)
    }
    fn slider(
        &self,
        label: &str,
        value: &mut f32,
        lo: f32,
        hi: f32,
        x: f32,
        y: f32,
        unit: &str,
    ) -> bool {
        self.text(label, x, y, 14., ink());
        let val = if unit == "" {
            format!("{:.0}", *value)
        } else if unit == "%" {
            format!("{:.0}%", *value * 100.)
        } else {
            format!("{:.2} {unit}", *value)
        };
        self.text(&val, x + 252. - self.width(&val, 13.), y, 13., muted());
        let r = Rect::new(x, y + 9., 252., 21.);
        let track_y = y + 18.;
        draw_line(x, track_y, x + 252., track_y, 4., line());
        let t = (*value - lo) / (hi - lo);
        draw_line(x, track_y, x + t * 252., track_y, 4., green());
        draw_circle(x + t * 252., track_y, 6., col(0xfcfdf9));
        draw_circle_lines(x + t * 252., track_y, 6., 2., green());
        if self.over(r) && is_mouse_button_down(MouseButton::Left) {
            let old = *value;
            *value = lo + ((self.mouse.x - x) / 252.).clamp(0., 1.) * (hi - lo);
            return old != *value;
        }
        false
    }
    fn heading(&self, num: &str, title: &str, x: f32, y: f32) {
        self.text(num, x, y, 11., green());
        self.text(title, x + 25., y, 16., ink());
    }
}
fn rounded(r: Rect, radius: f32, c: Color) {
    draw_rectangle(r.x + radius, r.y, r.w - 2. * radius, r.h, c);
    draw_rectangle(r.x, r.y + radius, r.w, r.h - 2. * radius, c);
    for (x, y) in [
        (r.x + radius, r.y + radius),
        (r.x + r.w - radius, r.y + radius),
        (r.x + radius, r.y + r.h - radius),
        (r.x + r.w - radius, r.y + r.h - radius),
    ] {
        draw_circle(x, y, radius, c)
    }
}
fn tree_icon(x: f32, y: f32, size: f32, species: &str, c: Color) {
    draw_line(x, y + size * 0.22, x, y + size, 2., c);
    if species == "pine" {
        for i in 0..3 {
            let t = i as f32;
            let top = y + t * size * 0.16;
            let w = size * (0.23 + t * 0.09);
            draw_triangle(
                vec2(x, top),
                vec2(x - w, top + size * 0.45),
                vec2(x + w, top + size * 0.45),
                c,
            )
        }
    } else {
        for (dx, dy, r) in [
            (0., 0.18, 0.28),
            (-0.20, 0.35, 0.25),
            (0.22, 0.34, 0.25),
            (-0.12, 0.03, 0.20),
            (0.14, 0.05, 0.20),
        ] {
            draw_circle(x + dx * size, y + dy * size, size * r, c)
        }
        draw_line(
            x,
            y + size * 0.65,
            x - size * 0.18,
            y + size * 0.35,
            1.5,
            col(0x77866a),
        );
        draw_line(
            x,
            y + size * 0.54,
            x + size * 0.20,
            y + size * 0.25,
            1.5,
            col(0x77866a),
        );
    }
}
fn meshes(p: &Part, foliage: bool) -> Vec<Mesh> {
    let tangents = p.tangents();
    let vertices: Vec<_> =
        p.v.iter()
            .enumerate()
            .map(|(i, v)| {
                let [r, g, b, a] = if foliage { v.c.map(tree::srgb) } else { v.c };
                let mut vertex = Vertex::new2(v.p, v.uv, Color::new(r, g, b, a));
                let n = v.n.normalize();
                let reference = if n.y.abs() > 0.95 { Vec3::X } else { Vec3::Y };
                let u = n.cross(reference).normalize();
                let b = n.cross(u);
                let tangent = tangents[i].truncate();
                let angle = tangent
                    .dot(b)
                    .atan2(tangent.dot(u))
                    .rem_euclid(std::f32::consts::TAU);
                vertex.normal = n.extend(
                    angle
                        + if tangents[i].w < 0. {
                            std::f32::consts::TAU
                        } else {
                            0.
                        },
                );
                vertex
            })
            .collect();
    // Macroquad uses 16-bit mesh indices. Keep complete triangles and remap each batch.
    let mut output = Vec::new();
    let mut start = 0;
    while start < p.i.len() {
        let mut lo = p.i[start];
        let mut hi = lo;
        let mut end = start;
        for tri in p.i[start..].chunks_exact(3) {
            let min = lo.min(*tri.iter().min().unwrap());
            let max = hi.max(*tri.iter().max().unwrap());
            if max - min >= 32_000 {
                break;
            }
            lo = min;
            hi = max;
            end += 3;
        }
        output.push(Mesh {
            vertices: vertices[lo as usize..=hi as usize].to_vec(),
            indices: p.i[start..end].iter().map(|i| (*i - lo) as u16).collect(),
            texture: None,
        });
        start = end;
    }
    output
}

struct Model {
    tree: Tree,
    bark: Vec<Mesh>,
    leaves: Vec<Mesh>,
    lod_counts: [usize; 3],
}
impl Model {
    fn new(s: &Settings, lod: u32) -> Self {
        let t = tree::generate(s, lod);
        let counts = std::array::from_fn(|i| {
            if i == lod as usize {
                t.triangles()
            } else {
                tree::generate(s, i as u32).triangles()
            }
        });
        Self {
            bark: meshes(&t.bark, false),
            leaves: meshes(&t.leaves, true),
            tree: t,
            lod_counts: counts,
        }
    }
}
#[derive(PartialEq, Clone, Copy)]
enum Modal {
    Export,
    Save,
    Open,
    Automation,
    Help,
    BarkColor,
    BarkHeight,
    BarkNormal,
    LeafAtlas,
}
struct Editor {
    s: Settings,
    model: Model,
    lod: u32,
    auto_lod: bool,
    forest: bool,
    forest_random: bool,
    forest_floor: bool,
    // Each species' share of the stand, 0 to 1. Shares are relative: the mix is normalised.
    forest_share: [f32; crate::catalog::GROUPS.len()],
    forest_count: u32,
    library_cwa: bool,
    library_page: usize,
    library_search: String,
    library_search_focus: bool,
    random_state: u32,
    forest_page: usize,
    sky: bool,
    skybox: usize,
    cloud_cover: f32,
    leaf_specular: bool,
    leaf_roughness: f32,
    leaf_transmission: f32,
    wind_speed: f32,
    wind_direction: f32,
    wind_strength: f32,
    wind_gust: f32,
    leaf_flutter: f32,
    branch_sway: f32,
    branch_springiness: f32,
    debug: solstice_render::DebugView,
    lighting: preview::Lighting,
    yaw: f32,
    pitch: f32,
    distance: f32,
    pan: Vec3,
    history: Vec<Settings>,
    redo: Vec<Settings>,
    drag_start: Option<Settings>,
    tab: u32,
    view: u32,
    grid: bool,
    bark: bool,
    leaves: bool,
    rotate: bool,
    wind: bool,
    modal: Option<Modal>,
    path: String,
    format: String,
    all_lods: bool,
    game_lods: bool,
    game_preview: bool,
    trim_cards: bool,
    path_focus: bool,
    status: String,
    status_until: f64,
    dirty: bool,
    saved: Settings,
    record_edit: bool,
    project_name: String,
}
impl Editor {
    fn new() -> Self {
        let s = Settings::preset("solstice-oak").expect("built-in Solstice oak");
        let distance = s.height * 2.32;
        Self {
            model: Model::new(&s, 0),
            saved: s.clone(),
            s,
            lod: 0,
            auto_lod: true,
            forest: false,
            forest_random: true,
            forest_floor: true,
            forest_share: std::array::from_fn(|i| if i < 5 { 0.2 } else { 0. }),
            forest_count: 200,
            library_cwa: false,
            library_page: 0,
            library_search: String::new(),
            library_search_focus: false,
            random_state: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().subsec_nanos() | 1,
            forest_page: 0,
            sky: true,
            skybox: 1,
            cloud_cover: preview::skybox("fair").expect("fair weather").coverage,
            leaf_specular: true,
            leaf_roughness: 0.55,
            leaf_transmission: 0.8,
            wind_speed: 1.,
            wind_direction: 68.,
            wind_strength: 1.,
            wind_gust: 0.2,
            leaf_flutter: 1.,
            branch_sway: 1.,
            branch_springiness: 0.5,
            debug: solstice_render::DebugView::Lit,
            lighting: preview::Lighting::default(),
            yaw: 0.65,
            pitch: 0.15,
            distance,
            pan: Vec3::ZERO,
            history: vec![],
            redo: vec![],
            drag_start: None,
            tab: 0,
            view: 0,
            grid: true,
            bark: true,
            leaves: true,
            rotate: false,
            wind: false,
            modal: None,
            path: "exports/oak.gltf".into(),
            format: "gltf".into(),
            all_lods: true,
            game_lods: false,
            game_preview: true,
            trim_cards: true,
            path_focus: false,
            status: "Ready. Make something that grows.".into(),
            status_until: 0.,
            dirty: false,
            record_edit: true,
            project_name: "Untitled grove".into(),
        }
    }
    fn rebuild(&mut self) {
        self.model = Model::new(&self.s, self.lod);
        self.dirty = self.s != self.saved;
    }
    fn notice(&mut self, s: impl Into<String>) {
        self.status = s.into();
        self.status_until = get_time() + 5.;
    }
    fn apply_preset(&mut self, name: &str) {
        self.s = Settings::preset(name).unwrap();
        self.lod = 0;
        self.rebuild();
        self.frame();
    }
    fn reroll(&mut self) {
        self.s.seed = self.s.seed.wrapping_add(1);
    }
    fn randomize(&mut self, scope: &str, field: Option<&str>) {
        self.random_state = self.random_state.wrapping_mul(1664525).wrapping_add(1013904223);
        let result = if let Some(field) = field { self.s.randomize_field(field, self.random_state) } else { self.s.randomize(scope, self.random_state) };
        if let Err(err) = result { self.notice(err); return; }
        self.notice("Generated a random variation.");
    }
    fn orbit(&mut self, delta: Vec2) {
        self.yaw -= delta.x * 0.008;
        self.pitch = (self.pitch + delta.y * 0.006).clamp(-0.55, 1.1);
    }
    /// Species in the stand: those with a non-zero share.
    fn forest_mask(&self) -> u128 {
        preview::stand(&self.forest_share).0
    }
    /// Each planted species' whole-percent share, for the scene.
    fn forest_ratios(&self) -> [u32; preview::SPECIES.len()] {
        preview::stand(&self.forest_share).1
    }
    fn scene_height(&self) -> f32 {
        preview::scene_height(&self.s, if self.forest { self.forest_mask() } else { 0 })
    }
    fn frame(&mut self) {
        self.yaw = 0.65;
        self.pitch = if self.forest { 0.44 } else { 0.15 };
        self.distance = if self.forest {
            preview::frame_distance(&self.s, self.forest_mask(), self.forest_count)
        } else {
            self.scene_height() * 2.32
        };
        self.pan = Vec3::ZERO;
    }
    fn set_lod(&mut self, n: u32) {
        self.auto_lod = false;
        if self.lod != n {
            self.lod = n;
            self.rebuild()
        }
    }
    fn undo(&mut self) {
        self.record_edit = false;
        self.drag_start = None;
        if let Some(s) = self.history.pop() {
            self.redo.push(self.s.clone());
            self.s = s;
            self.rebuild();
            self.notice("Change undone.")
        }
    }
    fn redo(&mut self) {
        self.record_edit = false;
        self.drag_start = None;
        if let Some(s) = self.redo.pop() {
            self.history.push(self.s.clone());
            self.s = s;
            self.rebuild();
            self.notice("Change restored.")
        }
    }
    fn open_modal(&mut self, m: Modal) {
        self.modal = Some(m);
        self.path = match m {
            Modal::Save | Modal::Open => "exports/project.json".into(),
            Modal::Export => format!("exports/{}.{}", self.s.species, self.format),
            Modal::BarkColor => self.s.bark_color_path.clone().unwrap_or_default(),
            Modal::BarkHeight => self.s.bark_height_path.clone().unwrap_or_default(),
            Modal::BarkNormal => self.s.bark_normal_path.clone().unwrap_or_default(),
            Modal::LeafAtlas => self.s.leaf_atlas_path.clone().unwrap_or_default(),
            _ => String::new(),
        };
        self.path_focus = false;
    }
    fn export(&mut self) -> Result<Vec<PathBuf>, String> {
        let base = Path::new(&self.path);
        if base.as_os_str().is_empty() {
            return Err("Enter an output filename.".into());
        }
        if self.format == "p3d" && self.game_lods {
            if base.extension().and_then(|s| s.to_str()) != Some("p3d") {
                return Err("Game LOD export needs a .p3d filename.".into());
            }
            let options = crate::poseidon::GameOptions { prefix: None, forest_proxy: false, budgets: None, opaque_leaves: false, trim_cards: self.trim_cards };
            crate::poseidon::export_game(&self.s, base, &options)?;
            return Ok(vec![base.to_path_buf()]);
        }
        let lods = if self.all_lods {
            vec![0, 1, 2]
        } else {
            vec![self.lod.min(2)]
        };
        agent::export_lods(&self.s, &lods, &self.format, base, None, None)?
            .iter()
            .map(|file| {
                file["path"]
                    .as_str()
                    .map(PathBuf::from)
                    .ok_or("Missing export path".into())
            })
            .collect()
    }
    fn project_action(&mut self, save: bool) -> Result<(), String> {
        if self.path.is_empty() {
            return Err("Enter a project filename.".into());
        }
        if save {
            tree::save_project(&self.s, Path::new(&self.path))?;
            self.saved = self.s.clone();
            self.dirty = false;
        } else {
            self.record_edit = false;
            let s = tree::load_project(Path::new(&self.path))?;
            self.history.push(self.s.clone());
            self.redo.clear();
            self.s = s;
            self.saved = self.s.clone();
            self.rebuild();
            self.frame();
        }
        self.project_name = Path::new(&self.path)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        Ok(())
    }
}

pub async fn run(args: Vec<String>) {
    let light = theme_path()
        .and_then(|path| fs::read_to_string(path).ok())
        .is_some_and(|theme| theme.trim() == "light");
    DARK_MODE.with(|mode| mode.set(!light));
    let font = fs::read("C:/Windows/Fonts/segoeui.ttf")
        .ok()
        .and_then(|b| load_ttf_font_from_bytes(&b).ok());
    let mut ui = Ui::new(font);
    let mut e = Editor::new();
    if let Some(name) = agent::option(&args, "--preset") {
        match Settings::preset(&name) {
            Ok(s) => {
                e.s = s;
                e.saved = e.s.clone();
                e.rebuild();
                e.frame();
            }
            Err(err) => e.notice(err),
        }
    }
    if let Some(path) = agent::option(&args, "--config") {
        e.path = path;
        if let Err(err) = e.project_action(false) {
            e.notice(err)
        }
    }
    if let Some(forest) = agent::option(&args, "--forest") {
        e.forest = true;
        let mask = preview::forest_mask(&forest).expect("validated forest selection");
        e.forest_share = std::array::from_fn(|g| {
            if crate::catalog::GROUPS[g].1.iter().any(|m| preview::SPECIES.iter().position(|s| s == m).is_some_and(|i| mask & (1u128 << i) != 0)) { 0.2 } else { 0. }
        });
        if let Some(count) = agent::option(&args, "--forest-count").and_then(|v| v.parse().ok()) {
            e.forest_count = count;
        }
        e.frame();
    }
    if let Some(view) = agent::option(&args, "--preview-view") {
        e.debug = solstice_render::DebugView::from_name(&view).expect("validated preview view");
    }
    if args.iter().any(|arg| arg == "--structure") { e.view = 2; }
    if args.iter().any(|arg| arg == "--wind") { e.wind = true; }
    let target = render_target(864, 730);
    // Macroquad negates Camera2D's Y zoom on the backbuffer. A positive zoom
    // keeps the layout's top-left origin aligned with mouse_position().
    let ui_camera = Camera2D {
        target: vec2(W * 0.5, H * 0.5),
        zoom: vec2(2. / W, 2. / H),
        ..Default::default()
    };
    let mut textures = Textures::new(&e.s);
    let tree_material = load_material(
        ShaderSource::Glsl {
            vertex: TREE_VERTEX,
            fragment: TREE_FRAGMENT,
        },
        MaterialParams {
            uniforms: vec![
                UniformDesc::new("Wind", UniformType::Float4),
                UniformDesc::new("CameraPos", UniformType::Float3),
                UniformDesc::new("Relief", UniformType::Float4),
            ],
            textures: vec!["HeightMap".into(), "NormalMap".into()],
            pipeline_params: miniquad::PipelineParams {
                depth_test: miniquad::Comparison::LessOrEqual,
                depth_write: true,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .expect("compile tree material");
    target.texture.set_filter(FilterMode::Linear);
    let mut prev = vec2(0., 0.);
    let mut frames = 0;
    let mut shot_saved = false;
    let shot = agent::option(&args, "--screenshot");
    let self_test = agent::option(&args, "--self-test");
    let classic = args.iter().any(|arg| arg == "--classic") || self_test.is_some();
    let mut worker = (!classic).then(preview::Worker::start);
    let mut preview_texture = None;
    let mut preview_frame: Option<(preview::SceneKey, solstice_render::DebugView, u32, u32)> = None;
    let mut preview_fps = 0.0f64;
    let mut preview_fps_start = get_time();
    let mut preview_fps_frames = 0u32;
    let mut preview_last_frame = 0.0f64;
    let mut preview_error = None;
    let mut wind_playback = WindPlayback::default();
    let initial_yaw = e.yaw;
    let initial_seed = e.s.seed;
    let initial_mesh = e.model.tree.leaves.v.first().map(|v| v.p);
    let mut hud_drawn = false;
    let mut test_steps = vec![];
    let mut tested_lod = false;
    let mut wind_reference: Option<Vec<u8>> = None;
    let mut relief_reference: Option<Vec<u8>> = None;
    let mut model_rebuild_pending = false;
    crate::log::log("editor loop starting");
    loop {
        frames += 1;
        if frames == 1 {
            crate::log::log(format!("first frame: forest={} count={} mask={:#x} skybox={} tab={}", e.forest, e.forest_count, e.forest_mask(), e.skybox, e.tab));
        }
        let scx = screen_width() / W;
        let scy = screen_height() / H;
        let (mx, my) = mouse_position();
        ui.mouse = vec2(mx / scx, my / scy);
        ui.scale = scx;
        ui.enabled = e.modal.is_none();
        let delta = ui.mouse - prev;
        prev = ui.mouse;
        ui.hud_markers.set(0);
        e.record_edit = true;
        let before = e.s.clone();
        if self_test.is_some() {
            if frames == 145 {
                e.wind = false;
                e.tab = 0;
                e.distance = e.s.height * 0.85;
                e.pan.y = -e.s.height * 0.33;
                e.s.parallax_depth = 0.;
            }
            if frames == 160 {
                e.s.parallax_depth = 0.08;
            }
            if frames == 180 {
                let count = e.model.tree.leaves.v.len();
                e.s.season = 3;
                e.rebuild();
                e.frame();
                e.tab = 1;
                let winter_count = e.model.tree.leaves.v.len();
                test_steps.push(json!({"step":"winter","passed":if e.s.species=="solstice-fir" {winter_count==count} else {winter_count>0&&winter_count<count/20},"detail":format!("Season palette changed foliage vertices from {count} to {winter_count}; winter remains editable and exportable.")}));
            }
            if frames == 130 {
                e.wind = true;
            }
            if frames == 10 {
                e.orbit(vec2(80., 10.));
                test_steps.push(json!({"step":"move","passed":e.yaw<initial_yaw-0.5,"detail":"Rightward drag orbits in the direction that moves the near face to screen right."}));
            }
            if frames == 30 {
                e.reroll();
            }
            if frames == 50 {
                test_steps.push(json!({"step":"act","passed":e.s.seed!=initial_seed&&e.model.tree.leaves.v.first().map(|v| v.p)!=initial_mesh,"detail":"Generate variation changed the seed and measured foliage positions."}));
            }
            if frames == 65 {
                let hi = e.model.lod_counts[0];
                e.set_lod(2);
                tested_lod = e.model.tree.triangles() < hi / 2;
            }
            if frames == 80 {
                e.set_lod(0);
            }
            if frames == 90 {
                let current = e.s.clone();
                e.undo();
                let changed = e.s != current;
                e.redo();
                test_steps.push(json!({"step":"undo-redo","passed":changed&&e.s==current,"detail":"Undid the generated variation and restored it through redo."}));
            }
            if frames == 110 {
                e.open_modal(Modal::Export);
            }
            if frames == 116 {
                e.modal = None;
                e.tab = 1;
            }
            if frames == 100 {
                let root = Path::new(self_test.as_ref().unwrap())
                    .parent()
                    .unwrap_or(Path::new("."));
                e.path = root
                    .join("self-test-exports/tree.gltf")
                    .to_string_lossy()
                    .to_string();
                let result = e.export();
                test_steps.push(json!({"step":"export","passed":result.as_ref().is_ok_and(|p|p.len()==3&&p.iter().all(|p|p.exists()&&p.with_extension("bin").exists())),"detail":match result{Ok(p)=>format!("Exported {} glTF detail levels with binary buffers.",p.len()),Err(err)=>err}}));
                e.path = root
                    .join("self-test-exports/project.json")
                    .to_string_lossy()
                    .to_string();
                let saved = e.s.clone();
                let ok = e.project_action(true).is_ok();
                e.s.height = 5.;
                let loaded = e.project_action(false).is_ok();
                test_steps.push(json!({"step":"project","passed":ok&&loaded&&e.s==saved,"detail":"Saved settings, changed height, reopened and matched all saved values."}));
            }
        }
        if e.modal.is_none() && !e.library_search_focus {
            let vp = Rect::new(
                258.,
                if e.forest { 198. } else { 162. },
                864.,
                if e.forest { 538. } else { 574. },
            );
            if vp.contains(ui.mouse) {
                if (is_mouse_button_down(MouseButton::Left)
                    && !is_mouse_button_pressed(MouseButton::Left))
                    || (is_mouse_button_down(MouseButton::Right)
                        && !is_mouse_button_pressed(MouseButton::Right))
                {
                    e.orbit(delta)
                }
                if is_mouse_button_down(MouseButton::Middle) {
                    let right = vec3(e.yaw.cos(), 0., -e.yaw.sin());
                    e.pan -= right * delta.x * e.distance * 0.001;
                    e.pan.y += delta.y * e.distance * 0.001;
                }
                let (_, scroll) = mouse_wheel();
                // Windows reports 120 per wheel notch; other platforms report notches already.
                let notches = if cfg!(windows) { scroll / 120. } else { scroll };
                let max_zoom = if e.forest { 15. } else { 9. };
                e.distance = (e.distance * (-notches * ZOOM_PER_NOTCH).exp())
                    .clamp(0.15, e.scene_height() * max_zoom);
            }
            let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
            if ctrl && is_key_pressed(KeyCode::S) {
                e.open_modal(Modal::Save)
            } else if ctrl && is_key_pressed(KeyCode::O) {
                e.open_modal(Modal::Open)
            } else if ctrl && is_key_pressed(KeyCode::Z) {
                e.undo()
            } else if ctrl && is_key_pressed(KeyCode::Y) {
                e.redo()
            }
            if is_key_pressed(KeyCode::Space) {
                e.reroll()
            }
            if is_key_pressed(KeyCode::F) {
                e.frame()
            }
            if is_key_pressed(KeyCode::G) {
                e.grid = !e.grid
            }
            if !e.forest && is_key_pressed(KeyCode::W) {
                e.view = if e.view == 1 { 0 } else { 1 }
            }
            if is_key_pressed(KeyCode::B) {
                e.wind = !e.wind;
            }
            if is_key_pressed(KeyCode::Key0) {
                e.auto_lod = true;
            }
            if is_key_pressed(KeyCode::Key1) {
                e.set_lod(0)
            }
            if is_key_pressed(KeyCode::Key2) {
                e.set_lod(1)
            }
            if is_key_pressed(KeyCode::Key3) {
                e.set_lod(2)
            }
            if is_key_pressed(KeyCode::F1) {
                e.open_modal(Modal::Help)
            }
            if is_key_pressed(KeyCode::T) {
                if let Err(error) = toggle_theme() {
                    e.notice(error);
                }
            }
        }
        if e.rotate && e.modal.is_none() {
            e.yaw += get_frame_time() * 0.20;
        }
        wind_playback.step(e.wind, e.wind_speed, e.branch_springiness, get_frame_time());
        let center = vec3(0., e.scene_height() * 0.49, 0.) + e.pan;
        let pos = center
            + vec3(
                e.yaw.sin() * e.pitch.cos(),
                e.pitch.sin(),
                e.yaw.cos() * e.pitch.cos(),
            ) * e.distance;
        let scene = preview::SceneKey {
            settings: e.s.clone(),
            lod: if e.auto_lod && e.game_preview {
                Some(if e.distance > e.scene_height() * 10. { 3 } else if e.distance > e.scene_height() * 5. { 2 } else if e.distance > e.scene_height() * 2.8 { 1 } else { 0 })
            } else { (!e.auto_lod).then_some(e.lod) },
            forest: if e.forest { e.forest_mask() } else { 0 },
            count: e.forest_count,
            ratios: e.forest_ratios(),
            grid: e.grid && !e.forest,
            bark: e.bark,
            leaves: preview_leaves(e.view, e.leaves),
            game_cards: e.game_preview,
            trim_cards: e.trim_cards,
            random_mix: e.forest_random,
            forest_floor: e.forest_floor,
            leaf_specular: e.leaf_specular,
            leaf_roughness: e.leaf_roughness,
            leaf_transmission: e.leaf_transmission,
        };
        // Structure is the Solstice preview with foliage hidden, so its branches use the
        // same hierarchy-based animation as the shaded tree. Only wireframe uses classic.
        let use_solstice = solstice_view(e.view);
        if let Some(renderer) = worker.as_mut() {
            match renderer.poll() {
                Some(Ok(frame)) => {
                    let now = get_time();
                    if preview_last_frame == 0.0 || now - preview_last_frame > 2.0 {
                        preview_fps_start = now;
                        preview_fps_frames = 0;
                        preview_fps = 0.0;
                    }
                    preview_last_frame = now;
                    preview_fps_frames += 1;
                    let elapsed = now - preview_fps_start;
                    if elapsed >= 1.0 {
                        preview_fps = preview_fps_frames as f64 / elapsed;
                        preview_fps_frames = 0;
                        preview_fps_start = now;
                    }
                    let (w, h) = preview::SIZE;
                    if let Some(texture) = &preview_texture {
                        Texture2D::update_from_bytes(texture, w, h, &frame.pixels);
                    } else {
                        let texture = Texture2D::from_rgba8(w as u16, h as u16, &frame.pixels);
                        texture.set_filter(FilterMode::Linear);
                        preview_texture = Some(texture);
                    }
                    preview_frame =
                        Some((frame.scene, frame.debug, frame.clusters, frame.triangles));
                }
                Some(Err(err)) => {
                    eprintln!("Solstice preview: {err}");
                    crate::log::log(format!("Solstice preview failed, falling back to classic: {err}"));
                    preview_error = Some(err);
                    worker = None;
                    if e.forest {
                        e.forest = false;
                        e.frame();
                    }
                }
                None => {}
            }
        }
        if use_solstice {
            if let Some(renderer) = worker.as_mut() {
                renderer.request(preview::Request {
                    scene: scene.clone(),
                    camera: solstice_render::Camera {
                        position: ::glam::Vec3::from_array(pos.to_array()),
                        yaw: e.yaw,
                        pitch: -e.pitch,
                        fov_y: 40_f32.to_radians(),
                        near: 0.05,
                    },
                    debug: e.debug,
                    wind_time: (wind_playback.envelope > 0.).then_some(wind_playback.phase),
                    wind_direction: e.wind_direction,
                    wind_strength: e.wind_strength * wind_playback.envelope,
                    wind_gust: e.wind_gust,
                    leaf_flutter: e.leaf_flutter,
                    branch_sway: e.branch_sway,
                    branch_springiness: e.branch_springiness,
                    sky: e.sky,
                    skybox: {
                        let mut weather = preview::skybox(preview::SKYBOXES[e.skybox].0).expect("listed skybox");
                        weather.coverage = e.cloud_cover;
                        weather
                    },
                    lighting: e.lighting,
                });
            }
        }
        let solstice_stats = preview_frame
            .as_ref()
            .filter(|(key, debug, _, _)| use_solstice && key == &scene && *debug == e.debug)
            .map(|(_, _, clusters, triangles)| (*clusters, *triangles));
        let awaiting_solstice = use_solstice && worker.is_some();
        let showing_solstice = awaiting_solstice && preview_texture.is_some();
        if !awaiting_solstice {
            set_camera(&Camera3D {
                position: pos,
                target: center,
                up: Vec3::Y,
                fovy: 40_f32.to_radians(),
                aspect: Some(864. / 730.),
                render_target: Some(target.clone()),
                z_near: 0.05,
                z_far: 150.,
                ..Default::default()
            });
            clear_background(rgb(0xe3e4e6));
            draw_plane(Vec3::ZERO, vec2(200., 200.), None, rgb(0xdfe0e2));
            // Soft procedural contact shadow, assembled from translucent rings.
            for k in (1..=22).rev() {
                let r = k as f32 / 22.;
                let alpha = 0.007;
                let c = Color::new(0.18, 0.18, 0.18, alpha);
                let p = vec3(0.55, 0.009 + (23 - k) as f32 * 0.0004, 0.1);
                disc(p, e.s.height * 0.36 * r, e.s.height * 0.24 * r, c);
            }
            if e.grid {
                draw_grid(30, 1., col(0xbdc9b7), col(0xd2dbca));
            }
            if e.view == 1 {
                for p in [&e.model.tree.bark, &e.model.tree.leaves] {
                    if (std::ptr::eq(p, &e.model.tree.bark) && !e.bark)
                        || (std::ptr::eq(p, &e.model.tree.leaves) && !e.leaves)
                    {
                        continue;
                    }
                    for tri in p.i.chunks(3) {
                        let a = p.v[tri[0] as usize].p;
                        let b = p.v[tri[1] as usize].p;
                        let c = p.v[tri[2] as usize].p;
                        draw_line_3d(a, b, col(0x48794b));
                        draw_line_3d(b, c, col(0x48794b));
                        draw_line_3d(c, a, col(0x48794b));
                    }
                }
            } else {
                if textures.species != e.s.species || textures.weathering != e.s.bark_weathering || textures.paths != [e.s.bark_color_path.clone(), e.s.bark_height_path.clone(), e.s.bark_normal_path.clone()] {
                    textures = Textures::new(&e.s);
                }
                textures.apply(&mut e.model);
                gl_use_material(&tree_material);
                tree_material.set_uniform("CameraPos", pos);
                tree_material.set_uniform("Relief", (e.s.parallax_depth, 1_f32, 0_f32, 0_f32));
                tree_material.set_texture("HeightMap", textures.height.clone());
                tree_material.set_texture("NormalMap", textures.normal.clone());
                let wind_time = if self_test.is_some() && frames >= 130 {
                    (frames - 130) as f32 * 0.2
                } else {
                    wind_playback.phase
                };
                tree_material.set_uniform(
                    "Wind",
                    (
                        wind_time,
                        if self_test.is_some() { if e.wind { 1. } else { 0. } } else { e.wind_strength * wind_playback.envelope },
                        e.wind_direction.to_radians(),
                        0_f32,
                    ),
                );
                if e.bark {
                    for mesh in &e.model.bark {
                        draw_mesh(mesh);
                    }
                }
                tree_material.set_uniform("Relief", (0_f32, 0_f32, 0_f32, 0_f32));
                tree_material.set_uniform(
                    "Wind",
                    (
                        wind_time,
                        if self_test.is_some() { if e.wind { 1. } else { 0. } } else { e.wind_strength * wind_playback.envelope },
                        e.wind_direction.to_radians(),
                        1_f32,
                    ),
                );
                if e.leaves && e.view != 2 {
                    for mesh in &e.model.leaves {
                        draw_mesh(mesh);
                    }
                }
                gl_use_default_material();
            }
            set_default_camera();
        }
        clear_background(col(0xf4f6f0));
        set_camera(&ui_camera);
        if showing_solstice || !awaiting_solstice {
            draw_texture_ex(
                if showing_solstice {
                    preview_texture.as_ref().unwrap()
                } else {
                    &target.texture
                },
                258.,
                72.,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(864., 730.)),
                    flip_y: !showing_solstice,
                    ..Default::default()
                },
            );
        } else {
            draw_rectangle(258., 72., 864., 730., col(0xf4f6f0));
        }
        header(&ui, &mut e);
        library(&ui, &mut e);
        inspector(&ui, &mut e, worker.is_some());
        let renderer_label = if solstice_stats.is_some() {
            "SOLSTICE"
        } else if preview_error.is_some() {
            "CLASSIC / GPU UNAVAILABLE"
        } else if use_solstice && worker.is_some() {
            "BUILDING SOLSTICE PREVIEW"
        } else {
            "CLASSIC"
        };
        viewport_ui(
            &ui,
            &mut e,
            solstice_stats,
            renderer_label,
            worker.is_some(),
            if use_solstice && preview_fps > 0.0 && get_time() - preview_last_frame < 2.0 { Some(preview_fps) } else { None },
        );
        // Small corner notice while the Solstice frame for the current settings is still rendering.
        let rendering = use_solstice && worker.is_some() && solstice_stats.is_none();
        if rendering {
            // A small pill in the viewport's corner, with a pulsing dot while the view rebuilds.
            let dots = ".".repeat(1 + (get_time() * 3.) as usize % 3);
            let (built, total) = preview::build_progress();
            let label = if total > 0 && built < total {
                format!("Building {built} of {total} species")
            } else {
                format!("Loading{dots}")
            };
            let width = ui.width(&label, 12.) + 24.;
            let x = 1112. - width;
            rounded(Rect::new(x, 778., width, 22.), 6., col(0xe8eee4));
            ui.text(&label, x + 12., 793., 12., ink());
        }
        lod_strip(&ui, &mut e);
        footer(&ui, &e);
        hud_drawn |= ui.hud_markers.get() == 7 && e.model.tree.valid();
        if let Some(m) = e.modal {
            ui.enabled = true;
            modal(&ui, &mut e, m);
        }
        if before != e.s {
            if e.record_edit && e.drag_start.is_none() {
                e.drag_start = Some(before)
            }
            e.dirty = e.s != e.saved;
            if worker.is_some() && solstice_view(e.view) && is_mouse_button_down(MouseButton::Left) {
                model_rebuild_pending = true;
            } else {
                e.rebuild();
                model_rebuild_pending = false;
            }
        }
        if !is_mouse_button_down(MouseButton::Left) {
            if model_rebuild_pending {
                e.rebuild();
                model_rebuild_pending = false;
            }
            if let Some(s) = e.drag_start.take() {
                e.history.push(s);
                if e.history.len() > 60 {
                    e.history.remove(0);
                }
                e.redo.clear();
            }
        }
        if let Some(path) = &self_test {
            if frames == 155 || frames == 170 {
                set_default_camera();
                let screen = get_screen_data();
                let mut pixels = vec![];
                // Compare only the 3D viewport, excluding changed slider text and UI.
                for y in (170. * scy) as usize..(700. * scy) as usize {
                    let a = (y * screen.width as usize + (300. * scx) as usize) * 4;
                    let b = (y * screen.width as usize + (1080. * scx) as usize) * 4;
                    pixels.extend_from_slice(&screen.bytes[a..b]);
                }
                if let Some(reference) = &relief_reference {
                    let changed = reference
                        .iter()
                        .zip(&pixels)
                        .filter(|(a, b)| a != b)
                        .count();
                    test_steps.push(json!({"step":"parallax","passed":changed>100,"detail":format!("Changing only relief depth from 0 to 0.08 changed {changed} viewport color bytes at a fixed close camera with wind off.")}));
                } else {
                    relief_reference = Some(pixels);
                }
                let root = Path::new(path).parent().unwrap_or(Path::new("."));
                save_screen(
                    &root
                        .join(if frames == 155 {
                            "relief-off.png"
                        } else {
                            "relief-closeup.png"
                        })
                        .to_string_lossy(),
                );
            }
            if frames == 190 {
                set_default_camera();
                let root = Path::new(path).parent().unwrap_or(Path::new("."));
                save_screen(&root.join("winter-preview.png").to_string_lossy());
            }
            if frames == 130 || frames == 140 {
                set_default_camera();
                let pixels = get_screen_data().bytes;
                if let Some(reference) = &wind_reference {
                    let changed = reference
                        .iter()
                        .zip(&pixels)
                        .filter(|(a, b)| a != b)
                        .count();
                    test_steps.push(json!({"step":"wind","passed":changed>100,"detail":format!("Wind animation changed {changed} rendered color bytes with a fixed camera and unchanged tree settings.")}));
                    let root = Path::new(path).parent().unwrap_or(Path::new("."));
                    save_screen(&root.join("wind-preview.png").to_string_lossy());
                } else {
                    wind_reference = Some(pixels);
                }
            }
            if frames == 115 || frames == 125 {
                let root = Path::new(path).parent().unwrap_or(Path::new("."));
                let name = if frames == 115 {
                    "export-dialog.png"
                } else {
                    "foliage-panel.png"
                };
                set_default_camera();
                save_screen(&root.join(name).to_string_lossy());
            }
        }
        if let Some(path) = &shot {
            if frames % 60 == 0 {
                crate::log::log(format!("frame {frames}: worker={} solstice_frame={} use_solstice={} error={:?}", worker.is_some(), solstice_stats.is_some(), use_solstice, preview_error));
            }
            // A screenshot run that never produces its image is reported, not left hanging.
            if get_time() > 90. && !shot_saved {
                crate::log::log("TIMEOUT: no screenshot after 90 s; exiting");
                eprintln!("Grove Studio: timed out waiting for the screenshot. See the log.");
                std::process::exit(3);
            }
            if frames >= 120
                && !shot_saved
                && (solstice_stats.is_some() || worker.is_none() || !use_solstice)
            {
                set_default_camera();
                save_screen(path);
                shot_saved = true;
                crate::log::log(format!("screenshot saved to {path}"));
                if self_test.is_none() {
                    break;
                }
            }
        }
        if let Some(path) = &self_test {
            if frames >= 200 {
                test_steps.push(json!({"step":"hud","passed":hud_drawn,"detail":"Drew the tree workspace, parameters, live triangle counts, LOD controls and status bar."}));
                test_steps.push(json!({"step":"lod","passed":tested_lod,"detail":"Lowest detail mesh has less than half the triangles of the full tree."}));
                if let Some(p) = Path::new(path).parent() {
                    let _ = fs::create_dir_all(p);
                }
                let report = json!({"version":1,"steps":test_steps});
                fs::write(path, serde_json::to_vec_pretty(&report).unwrap())
                    .expect("write self test");
                break;
            }
        }
        next_frame().await;
    }
    crate::log::log("editor loop ended");
}
fn save_screen(path: &str) {
    if let Some(p) = Path::new(path).parent() {
        let _ = fs::create_dir_all(p);
    }
    get_screen_data().export_png(path);
}
fn disc(p: Vec3, rx: f32, rz: f32, c: Color) {
    let mut vertices = vec![Vertex::new2(p, Vec2::ZERO, c)];
    let mut indices = vec![];
    for k in 0..=48 {
        let a = k as f32 / 48. * 6.283185;
        vertices.push(Vertex::new2(
            p + vec3(a.cos() * rx, 0., a.sin() * rz),
            Vec2::ZERO,
            c,
        ));
        if k > 0 {
            indices.extend([0, k as u16, k as u16 + 1]);
        }
    }
    draw_mesh(&Mesh {
        vertices,
        indices,
        texture: None,
    })
}

fn header(ui: &Ui, e: &mut Editor) {
    draw_rectangle(0., 0., W, 72., col(0x172a22));
    tree_icon(35., 18., 29., "oak", mint());
    ui.text("grove", 59., 40., 28., col(0xf1f7e9));
    ui.text("S T U D I O", 143., 39., 11., col(0xa5b99f));
    draw_line(257., 17., 257., 55., 1., col(0x3e5143));
    ui.text("Tree workshop", 282., 31., 15., col(0xf0f4e9));
    ui.text(
        &format!(
            "{}  /  {}",
            e.project_name,
            if e.dirty {
                "Unsaved changes"
            } else {
                "Procedural collection"
            }
        ),
        282.,
        51.,
        11.,
        col(0x9dad9d),
    );
    if ui.button("Undo", Rect::new(816., 21., 64., 32.), 3) {
        e.undo()
    }
    if ui.button(
        if dark_mode() { "Light" } else { "Dark" },
        Rect::new(730., 21., 75., 32.),
        3,
    ) {
        if let Err(error) = toggle_theme() {
            e.notice(error);
        }
    }
    if ui.button("Redo", Rect::new(890., 21., 64., 32.), 3) {
        e.redo()
    }
    if ui.button("Open", Rect::new(964., 21., 65., 32.), 3) {
        e.open_modal(Modal::Open)
    }
    if ui.button("Save project", Rect::new(1039., 21., 118., 32.), 3) {
        e.open_modal(Modal::Save)
    }
    if ui.button("Export model  >", Rect::new(1177., 16., 205., 42.), 1) {
        e.open_modal(Modal::Export)
    }
    if ui.button("?", Rect::new(1394., 23., 27., 28.), 3) {
        e.open_modal(Modal::Help)
    }
}
pub(crate) fn library_matches(query: &str, cwa: bool) -> Vec<(&'static str, &'static str, &'static str)> {
    let base = [
        ("solstice-oak", "Solstice oak", "Crooked, open-grown limbs"),
        ("solstice-birch", "Solstice birch", "Tall trunk and hanging twigs"),
        ("solstice-fir", "Solstice fir", "Whorled conifer branches"),
        ("solstice-bush", "Solstice bush", "Dense multi-stem shrub"),
        ("solstice-willow", "Solstice weeping willow", "Long hanging shoots"),
        ("solstice-old_oak", "Solstice old oak", "Broad, mature specimen"),
    ];
    let query = query.trim().to_lowercase();
    let mut trees = Vec::new();
    if !cwa || !query.is_empty() { trees.extend(base); }
    for (i, &(id, name, group, _)) in crate::catalog::CUSTOM.iter().enumerate() {
        let grove_preset = i == 0 || id.starts_with("solstice-forest-");
        if !query.is_empty() || (cwa && !grove_preset) || (!cwa && grove_preset) { trees.push((id, name, group)); }
    }
    trees.into_iter().filter(|(id, name, group)| query.is_empty() || id.to_lowercase().contains(&query) || name.to_lowercase().contains(&query) || group.to_lowercase().contains(&query)).collect()
}

fn library(ui: &Ui, e: &mut Editor) {
    draw_rectangle(0., 72., 258., 796., col(0xf6f7f2));
    draw_line(257., 72., 257., 868., 1., line());
    ui.text("T R E E   L I B R A R Y", 22., 106., 11., muted());
    for (i, name) in ["Solstice", "CWA"].iter().enumerate() {
        if ui.button(
            name,
            Rect::new(18. + i as f32 * 112., 124., 107., 31.),
            if e.library_cwa == (i == 1) { 2 } else { 0 },
        ) {
            e.library_cwa = i == 1;
            e.library_page = 0;
        }
    }
    let field = Rect::new(14., 160., 229., 28.);
    rounded(field, 5., col(0xeaf0e2));
    if e.library_search_focus { draw_rectangle_lines(field.x, field.y, field.w, field.h, 1.5, green()); }
    let search_label = if e.library_search.is_empty() { "Search all trees..." } else { &e.library_search };
    ui.text(&search_label.chars().take(28).collect::<String>(), 22., 179., 13., if e.library_search.is_empty() { muted() } else { ink() });
    if ui.enabled && is_mouse_button_pressed(MouseButton::Left) { e.library_search_focus = field.contains(ui.mouse); }
    if ui.enabled && e.library_search_focus && e.modal.is_none() {
        let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
        if ctrl && is_key_pressed(KeyCode::A) { e.library_search.clear(); e.library_page = 0; }
        if ctrl && is_key_pressed(KeyCode::V) {
            if let Some(s) = macroquad::miniquad::window::clipboard_get() { e.library_search.extend(s.chars().filter(|c| !c.is_control()).take(64)); e.library_page = 0; }
        }
        while let Some(c) = get_char_pressed() { if !c.is_control() && e.library_search.len() < 128 { e.library_search.push(c); e.library_page = 0; } }
        if is_key_pressed(KeyCode::Backspace) { e.library_search.pop(); e.library_page = 0; }
        if is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Enter) { e.library_search_focus = false; }
    }
    let matches = library_matches(&e.library_search, e.library_cwa);
    let pages = matches.len().div_ceil(LIBRARY_PAGE).max(1);
    e.library_page = e.library_page.min(pages - 1);
    for (i, (id, name, desc)) in matches.iter().skip(e.library_page * LIBRARY_PAGE).take(LIBRARY_PAGE).enumerate() {
        let y = 192. + i as f32 * 50.;
        let active = e.s.species == *id;
        let r = Rect::new(14., y, 229., 46.);
        let over = ui.over(r);
        rounded(
            r,
            8.,
            if active {
                col(0xe5eddd)
            } else if over {
                col(0xedefe6)
            } else {
                col(0xf0f2eb)
            },
        );
        if active {
            draw_rectangle(14., y + 6., 3., 34., green())
        }
        tree_icon(
            48.,
            y + 10.,
            33.,
            if crate::reference::evergreen_id(id) {
                "pine"
            } else {
                id.strip_prefix("solstice-").unwrap_or(id)
            },
            if active { col(0x62834c) } else { col(0x8e9e7e) },
        );
        ui.text(name, 81., y + 22., 14., ink());
        ui.text(desc, 81., y + 39., 10., muted());
        if active {
            draw_circle(225., y + 17., 3., green())
        }
        if over && is_mouse_button_pressed(MouseButton::Left) && !active {
            e.apply_preset(id);
            e.notice(format!("Loaded the {name} preset."));
        }
    }
    if matches.is_empty() { ui.text("No trees found.", 22., 215., 13., muted()); }
    if pages > 1 {
        if ui.button("<", Rect::new(14., 452., 40., 26.), 0) && e.library_page > 0 {
            e.library_page -= 1;
        }
        ui.center(&format!("{} / {}", e.library_page + 1, pages), Rect::new(58., 452., 150., 26.), 11., muted());
        if ui.button(">", Rect::new(211., 452., 40., 26.), 0) && e.library_page + 1 < pages {
            e.library_page += 1;
        }
    }
    draw_line(22., 487., 236., 487., 1., line());
    ui.text("V I E W P O R T   L A Y E R S", 22., 518., 10., muted());
    for (i, (label, on)) in [
        ("Trunk & branches", e.bark),
        ("Foliage", e.leaves),
        ("Ground grid", e.grid),
    ]
    .iter()
    .enumerate()
    {
        let y = 542. + i as f32 * 37.;
        let r = Rect::new(21., y - 18., 215., 32.);
        ui.text(
            if *on { "o" } else { "-" },
            24.,
            y,
            17.,
            if *on { green() } else { muted() },
        );
        ui.text(label, 49., y, 13., ink());
        ui.text(if *on { "ON" } else { "OFF" }, 210., y, 9., muted());
        if ui.over(r) && is_mouse_button_pressed(MouseButton::Left) {
            match i {
                0 => e.bark = !e.bark,
                1 => e.leaves = !e.leaves,
                _ => e.grid = !e.grid,
            }
        }
    }
    rounded(Rect::new(17., 713., 224., 134.), 9., col(0xe8eee1));
    draw_circle(34., 736., 3., green());
    ui.text("Built for your workflow", 46., 741., 13., ink());
    ui.text("One seed. Any number of trees.", 30., 766., 11., muted());
    ui.text("Headless generation + MCP tools", 30., 784., 11., muted());
    if ui.button("Agent workflows  >", Rect::new(29., 800., 200., 32.), 2) {
        e.open_modal(Modal::Automation)
    }
}
fn inspector(ui: &Ui, e: &mut Editor, gpu: bool) {
    let x = 1122.;
    let p = x + 28.;
    draw_rectangle(x, 72., 318., 796., col(0xfafbF7));
    draw_line(x, 72., x, 868., 1., line());
    ui.text("Shape your tree", p, 111., 20., ink());
    ui.text(
        "Small changes. Natural possibilities.",
        p,
        136.,
        12.,
        muted(),
    );
    rounded(Rect::new(p, 157., 262., 35.), 6., col(0xeef1e9));
    if !gpu && e.tab >= 2 {
        e.tab = 0;
    }
    let labels: &[_] = if gpu {
        &["Shape", "Leaf", "Light", "Forest", "Render", "Weather"]
    } else {
        &["Structure", "Foliage"]
    };
    let tab_width = 260. / labels.len() as f32;
    for (i, label) in labels.iter().enumerate() {
        let r = Rect::new(p + 3. + i as f32 * tab_width, 160., tab_width - 4., 29.);
        if e.tab == i as u32 {
            rounded(r, 4., col(0xffffff));
            ui.center(label, r, 11., ink());
        } else {
            ui.center(label, r, 11., muted());
        }
        if ui.over(r) && is_mouse_button_pressed(MouseButton::Left) {
            e.tab = i as u32
        }
    }
    if e.tab == 0 {
        ui.heading("01", "Tree shape", p, 223.);
        if ui.button("Random shape", Rect::new(p + 150., 203., 112., 24.), 0) { e.randomize("shape", None); }
        let shrub = e.s.species == "solstice-bush";
        ui.slider(
            "Trunk length",
            &mut e.s.height,
            if shrub { 0.25 } else { 3. },
            if shrub { 0.65 } else { 40. },
            p,
            242.,
            "m",
        );
        ui.slider("Tree age (relative)", &mut e.s.maturity, if shrub { 0.4 } else { 0.15 }, 1.2, p, 280., "%");
        ui.slider("Canopy spread", &mut e.s.spread, 0.4, 1.6, p, 318., "%");
        ui.slider("Trunk straightness", &mut e.s.trunk_straightness, 0., 1., p, 356., "%");
        let mut branches = e.s.branches as f32;
        if ui.slider("Primary branches", &mut branches, 5., 20., p, 394., "") {
            e.s.branches = branches.round() as u32;
        }
        ui.slider("Branch angle", &mut e.s.branch_angle, 0.5, 1.5, p, 432., "%");
        ui.slider("Branch droop", &mut e.s.branch_droop, 0., 2., p, 470., "%");
        ui.slider(
            "Trunk radius",
            &mut e.s.trunk_radius,
            0.05,
            1.0,
            p,
            508.,
            "m",
        );
        ui.slider(
            "Bark relief",
            &mut e.s.parallax_depth,
            0.,
            0.08,
            p,
            546.,
            "%",
        );
        for (y, field) in [(242., "height"), (280., "maturity"), (318., "spread"), (356., "trunk_straightness"), (394., "branches"), (432., "branch_angle"), (470., "branch_droop"), (508., "trunk_radius"), (546., "parallax_depth")] {
            if ui.button("R", Rect::new(p + 264., y - 16., 22., 23.), 0) { e.randomize("shape", Some(field)); }
        }
        ui.text("R: randomize this setting", p, 590., 11., muted());
    } else if e.tab == 1 {
        ui.heading("01", "Canopy detail", p, 229.);
        if ui.button("Random leaves", Rect::new(p + 150., 209., 112., 24.), 0) { e.randomize("foliage", None); }
        ui.slider("Foliage density", &mut e.s.density, 0.25, 1.8, p, 265., "%");
        ui.slider("Leaf size", &mut e.s.leaf_size, 0.025, 0.4, p, 329., "m");
        if ui.button("R", Rect::new(p + 264., 249., 22., 23.), 0) { e.randomize("foliage", Some("density")); }
        if ui.button("R", Rect::new(p + 264., 313., 22., 23.), 0) { e.randomize("foliage", Some("leaf_size")); }
        ui.heading("02", "Season palette", p, 412.);
        if ui.button("Random season", Rect::new(p + 150., 392., 112., 24.), 0) {
            e.randomize("foliage", Some("season"));
        }
        for (i, (label, c)) in [
            ("Summer", col(0x527a37)),
            ("Autumn", col(0xba7639)),
            ("Spring", col(0x93b865)),
            ("Winter", col(0x9ba6aa)),
        ]
        .iter()
        .enumerate()
        {
            let r = Rect::new(p + i as f32 * 66., 432., 64., 68.);
            if e.s.season == i as u32 {
                rounded(r, 6., col(0xe5eddd));
            }
            draw_circle(r.x + 32., r.y + 22., 12., *c);
            ui.center(label, Rect::new(r.x, r.y + 41., 64., 20.), 11., ink());
            if ui.over(r) && is_mouse_button_pressed(MouseButton::Left) {
                e.s.season = i as u32
            }
        }
        ui.text(
            if e.s.season == 3 {
            if crate::reference::evergreen(&e.s) {
                    "Winter needles. Pine stays evergreen."
                } else {
                    "Winter: bare branches, a few dried leaves."
                }
            } else {
                "Opaque leaf geometry. Two materials."
            },
            p,
            541.,
            11.,
            muted(),
        );
        ui.text("R: randomize this setting", p, 573., 11., muted());
    } else if e.tab == 2 {
        ui.heading("01", "Sun and sky", p, 229.);
        ui.slider(
            "Time of day",
            &mut e.lighting.hours,
            0.,
            23.99,
            p,
            265.,
            "h",
        );
        ui.slider(
            "Sun azimuth",
            &mut e.lighting.azimuth,
            0.,
            360.,
            p,
            329.,
            "°",
        );
        let minutes = (e.lighting.hours * 60.).round() as u32 % 1440;
        let sun = e.lighting.environment().sun_disc.unwrap();
        ui.text(
            &format!(
                "{:02}:{:02} solar time  /  {:.1}° elevation",
                minutes / 60,
                minutes % 60,
                sun.y.asin().to_degrees()
            ),
            p,
            386.,
            12.,
            muted(),
        );
        ui.text("Azimuth starts at scene north (-Z).", p, 366., 12., muted());
        for (i, (label, hours)) in [("Dawn", 5.4), ("Noon", 12.), ("Dusk", 19.2), ("Night", 0.)]
            .into_iter()
            .enumerate()
        {
            if ui.button(label, Rect::new(p + i as f32 * 66., 404., 62., 30.), 0) {
                e.lighting.hours = hours;
                e.view = 0;
                e.debug = solstice_render::DebugView::Lit;
            }
        }
    } else if e.tab == 5 {
        ui.heading("01", "Sky and clouds", p, 218.);
        ui.slider("Cloud cover", &mut e.cloud_cover, 0., 1., p, 240., "%");
        // Skybox presets adjust cloud type and other weather characteristics as well as cover.
        for (i, (_, label)) in preview::SKYBOXES.iter().enumerate() {
            let r = Rect::new(p + (i % 4) as f32 * 66., 281. + (i / 4) as f32 * 31., 62., 27.);
            if ui.button(label, r, if e.skybox == i { 2 } else { 0 }) {
                e.skybox = i;
                e.cloud_cover = preview::skybox(preview::SKYBOXES[i].0).expect("listed skybox").coverage;
            }
        }
        if ui.button(
            if e.sky { "Sky on" } else { "Sky off" },
            Rect::new(p + 132., 312., 62., 27.),
            0,
        ) {
            e.sky = !e.sky;
        }
        if ui.button("Reset", Rect::new(p + 198., 312., 62., 27.), 0) {
            e.skybox = 1;
            e.cloud_cover = preview::skybox("fair").expect("fair weather").coverage;
            e.sky = true;
            e.wind_speed = 1.; e.wind_strength = 1.; e.wind_gust = 0.2; e.leaf_flutter = 1.; e.branch_sway = 1.; e.branch_springiness = 0.5; e.wind_direction = 68.;
        }
        ui.heading("02", "Wind", p, 365.);
        if ui.button(if e.wind { "Wind ON" } else { "Wind OFF" }, Rect::new(p + 150., 347., 102., 27.), if e.wind { 2 } else { 0 }) { e.wind = !e.wind; }
        ui.slider("Wind speed", &mut e.wind_speed, 0., 3., p, 390., "x");
        ui.slider("Wind strength", &mut e.wind_strength, 0., 2., p, 419., "%");
        ui.slider("Branch sway", &mut e.branch_sway, 0., 2., p, 448., "%");
        ui.slider("Springiness", &mut e.branch_springiness, 0., 1., p, 477., "%");
        ui.slider("Gust strength", &mut e.wind_gust, 0., 1., p, 506., "%");
        ui.slider("Leaf flutter", &mut e.leaf_flutter, 0., 2., p, 535., "%");
        ui.slider("Wind direction", &mut e.wind_direction, 0., 360., p, 564., "°");
    } else if e.tab == 4 {
        ui.heading("01", "Leaf lighting", p, 229.);
        if ui.button(if e.leaf_specular { "Leaf specular: ON" } else { "Leaf specular: OFF" }, Rect::new(p, 250., 252., 32.), if e.leaf_specular { 2 } else { 0 }) {
            e.leaf_specular = !e.leaf_specular;
        }
        ui.slider("Leaf roughness", &mut e.leaf_roughness, 0.05, 1., p, 320., "%");
        ui.slider("Leaf translucency", &mut e.leaf_transmission, 0., 1., p, 387., "%");
        ui.slider("Bark moss / dirt", &mut e.s.bark_weathering, 0., 1., p, 450., "%");
        if ui.button("Reset render settings", Rect::new(p, 510., 252., 30.), 0) {
            e.leaf_specular = true;
            e.leaf_roughness = 0.55;
            e.leaf_transmission = 0.8;
        }
        for (i, (label, mode)) in [("Bark color", Modal::BarkColor), ("Bark height", Modal::BarkHeight), ("Bark normal", Modal::BarkNormal), ("Leaf atlas", Modal::LeafAtlas)].into_iter().enumerate() {
            if ui.button(label, Rect::new(p + (i % 2) as f32 * 128., 551. + (i / 2) as f32 * 31., 124., 27.), 0) { e.open_modal(mode); }
        }
    } else {
        ui.heading("01", "Forest composition", p, 222.);
        ui.text(
            "Species shares:",
            p,
            245.,
            11.,
            muted(),
        );
        if ui.button(if e.forest_random { "Random mix" } else { "Even mix" }, Rect::new(p + 138., 229., 124., 24.), if e.forest_random { 2 } else { 0 }) {
            e.forest_random = !e.forest_random;
        }
        let mut count = e.forest_count as f32;
        if ui.slider("Trees in the stand", &mut count, 10., 1000., p, 262., "") {
            e.forest_count = count.round() as u32;
        }
        let groups = crate::catalog::GROUPS;
        let pages = groups.len().div_ceil(FOREST_PAGE);
        for row in 0..FOREST_PAGE {
            let i = e.forest_page * FOREST_PAGE + row;
            if i >= groups.len() {
                break;
            }
            let label = groups[i].0;
            let y = 300. + row as f32 * 34.;
            let previous = e.forest_share[i];
            let mut value = previous;
            if ui.slider(label, &mut value, 0., 1., p, y, "%") {
                // The stand needs at least one group: a share cannot go to zero when it is the last.
                let others: f32 = e
                    .forest_share
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| *j != i)
                    .map(|(_, s)| *s)
                    .sum();
                e.forest_share[i] = if others + value > 0. { value } else { previous };
                if !e.forest {
                    e.forest = true;
                    e.frame();
                }
            }
        }
        if ui.button(if e.forest_floor { "Forest floor: ON" } else { "Forest floor: OFF" }, Rect::new(p, 536., 262., 24.), if e.forest_floor { 2 } else { 0 }) {
            e.forest_floor = !e.forest_floor;
        }
        if ui.button("<", Rect::new(p, 566., 40., 26.), 0) && e.forest_page > 0 {
            e.forest_page -= 1;
        }
        ui.center(
            &format!("{} / {}", e.forest_page + 1, pages),
            Rect::new(p + 46., 566., 170., 26.),
            11.,
            muted(),
        );
        if ui.button(">", Rect::new(p + 222., 566., 40., 26.), 0) && e.forest_page + 1 < pages {
            e.forest_page += 1;
        }
    }    draw_line(p, 600., 1412., 600., 1., line());
    ui.text("VARIATION SEED", p, 616., 10., muted());
    if ui.button("-", Rect::new(p, 625., 31., 32.), 0) {
        e.s.seed = e.s.seed.wrapping_sub(1)
    }
    rounded(Rect::new(p + 39., 625., 96., 32.), 5., col(0xeff2e9));
    ui.center(
        &format!("{:06}", e.s.seed),
        Rect::new(p + 39., 625., 96., 32.),
        14.,
        ink(),
    );
    if ui.button("+", Rect::new(p + 143., 625., 31., 32.), 0) {
        e.reroll()
    }
    if ui.button("New seed", Rect::new(p + 182., 625., 80., 32.), 2) {
        e.s.seed = e.s.seed.wrapping_mul(1664525).wrapping_add(1013904223)
    }
    rounded(Rect::new(p, 666., 262., 128.), 7., col(0xeaf0e2));
    ui.text("SOURCE MESH", p + 14., 690., 10., green());
    let count = e.model.tree.triangles();
    ui.text(
        &format!("{} triangles", commas(count)),
        p + 14.,
        718.,
        22.,
        ink(),
    );
    draw_line(p + 15., 735., p + 247., 735., 4., col(0xd4dfc9));
    draw_line(
        p + 15.,
        735.,
        p + 15. + 232. * (count as f32 / 40000.).min(1.),
        735.,
        4.,
        if count > 30000 {
            col(0xb28b42)
        } else {
            green()
        },
    );
    ui.text(
        "2 materials  /  3 detail levels",
        p + 14.,
        759.,
        11.,
        green(),
    );
    ui.text("Vertex colors · meters · Y up", p + 14., 780., 11., muted());
    if ui.button("Random tree", Rect::new(p, 817., 126., 29.), 2) { e.randomize("all", None); e.frame(); }
    if ui.button("Reset", Rect::new(p + 133., 817., 129., 29.), 0) {
        let name = e.s.species.clone();
        e.s = Settings::preset(&name).unwrap();
        e.frame();
    }
}
fn viewport_ui(ui: &Ui, e: &mut Editor, stats: Option<(u32, u32)>, renderer: &str, gpu: bool, preview_fps: Option<f64>) {
    rounded(
        Rect::new(274., 93., 284., 112.),
        8.,
        Color {
            a: 0.94,
            ..col(0xf7f9f2)
        },
    );
    ui.text(
        "PROCEDURAL COLLECTION  /  01",
        285.,
        110.,
        10.,
        col(0x819176),
    );
    let name = crate::catalog::CUSTOM
        .iter()
        .find(|c| c.0 == e.s.species)
        .map(|c| c.1)
        .unwrap_or(match e.s.species.as_str() {
            "solstice-birch" => "Solstice birch",
            "solstice-fir" => "Solstice fir",
            "solstice-bush" => "Solstice bush",
            "solstice-willow" => "Solstice weeping willow",
            "solstice-old_oak" => "Solstice old oak",
            _ => "Solstice oak",
        });    ui.text(
        if e.forest { "Forest preview" } else { name },
        284.,
        145.,
        29.,
        ink(),
    );
    ui.text(
        &format!(
            "{}  /  SEED {:06}",
            if crate::reference::evergreen(&e.s) {
                "EVERGREEN"
            } else {
                "DECIDUOUS"
            },
            e.s.seed
        ),
        285.,
        169.,
        10.,
        col(0x819176),
    );
    ui.text(renderer, 285., 192., 10., ink());
    let fps = preview_fps.map(|n| format!("Preview {n:.0} FPS  /  UI {} FPS", get_fps()))
        .unwrap_or_else(|| format!("Preview -- FPS  /  UI {} FPS", get_fps()));
    rounded(Rect::new(914., 172., 184., 25.), 5., col(0xf7f9f2));
    ui.center(&fps, Rect::new(914., 172., 184., 25.), 10., ink());
    if gpu && ui.button(if e.game_preview { "Game cards" } else { "Original leaves" }, Rect::new(783., 136., 145., 29.), if e.game_preview { 2 } else { 0 }) {
        e.game_preview = !e.game_preview;
        if !e.game_preview && e.lod > 2 { e.lod = 2; }
        e.view = 0;
    }
    if gpu && e.game_preview && ui.button(if e.trim_cards { "Trim ON" } else { "Trim OFF" }, Rect::new(936., 136., 162., 29.), if e.trim_cards { 2 } else { 0 }) {
        e.trim_cards = !e.trim_cards;
        e.view = 0;
    }
    if gpu && e.game_preview && ((!e.auto_lod && e.lod == 0) || (e.auto_lod && e.distance <= e.scene_height() * 2.8)) {
        rounded(Rect::new(914., 201., 184., 24.), 5., col(0xf7f9f2));
        ui.center("Trim applies from Mid onward", Rect::new(914., 201., 184., 24.), 10., ink());
    }
    if gpu
        && ui.button(
            if e.forest { "Single tree" } else { "Forest" },
            Rect::new(681., 97., 93., 26.),
            if e.forest { 2 } else { 0 },
        )
    {
        e.forest = !e.forest;
        e.view = 0;
        e.frame();
    }
    if !e.forest {
        let labels = ["Shaded", "Wireframe", "Structure"];
        rounded(Rect::new(783., 93., 315., 34.), 7., col(0xf7f9f2));
        for (i, s) in labels.iter().enumerate() {
            let r = Rect::new(787. + i as f32 * 103., 97., 101., 26.);
            if e.view == i as u32 {
                rounded(r, 4., col(0xe1e9d8));
            }
            ui.center(s, r, 11., if e.view == i as u32 { ink() } else { muted() });
            if ui.over(r) && is_mouse_button_pressed(MouseButton::Left) {
                e.view = i as u32;
                if i == 0 {
                    e.debug = solstice_render::DebugView::Lit;
                }
            }
        }
    }
    if ui.button(
        if e.rotate {
            "Pause orbit"
        } else {
            "Auto orbit"
        },
        Rect::new(282., 740., 92., 30.),
        0,
    ) {
        e.rotate = !e.rotate
    }
    if ui.button("Frame  F", Rect::new(384., 740., 85., 30.), 0) {
        e.frame()
    }
    if ui.button(
        if e.wind { "Wind on" } else { "Wind  B" },
        Rect::new(479., 740., 69., 30.),
        if e.wind { 2 } else { 0 },
    ) {
        e.wind = !e.wind;
    }
    let views = [
        (solstice_render::DebugView::Lit, "Lighting"),
        (solstice_render::DebugView::Triangles, "Triangles"),
        (solstice_render::DebugView::Clusters, "Clusters"),
        (solstice_render::DebugView::DiffuseGi, "Indirect light"),
        (solstice_render::DebugView::Voxels, "GI voxels"),
    ];
    let view = views
        .iter()
        .position(|(mode, _)| *mode == e.debug)
        .unwrap_or(0);
    if gpu && ui.button(views[view].1, Rect::new(911., 740., 109., 30.), 0) {
        e.debug = views[(view + 1) % views.len()].0;
        e.view = 0;
    }
    rounded(
        Rect::new(553., 739., 346., 32.),
        7.,
        Color {
            a: 0.94,
            ..col(0xf7f9f2)
        },
    );
    ui.center(
        &match stats {
            Some((clusters, triangles)) => format!(
                "{} drawn triangles  /  {} clusters  /  {}",
                commas(triangles as usize),
                clusters,
                if e.auto_lod {
                    "Auto".into()
                } else {
                    format!("LOD {}", e.lod)
                }
            ),
            None => format!(
                "{} triangles  /  {} vertices",
                commas(e.model.tree.triangles()),
                commas(e.model.tree.vertices())
            ),
        },
        Rect::new(553., 739., 346., 32.),
        11.,
        col(0x5c7055),
    );
    let o = vec2(1061., 730.);
    let right = vec3(e.yaw.cos(), 0., -e.yaw.sin());
    let backward = vec3(
        e.yaw.sin() * e.pitch.cos(),
        e.pitch.sin(),
        e.yaw.cos() * e.pitch.cos(),
    );
    let up = backward.cross(right);
    for (axis, s, c) in [
        (Vec3::Y, "Y", col(0x648558)),
        (Vec3::X, "X", col(0xaa7960)),
        (Vec3::Z, "Z", col(0x688c9c)),
    ] {
        let dx = axis.dot(right) * 32.;
        let dy = -axis.dot(up) * 32.;
        draw_line(o.x, o.y, o.x + dx, o.y + dy, 1.5, c);
        draw_circle(o.x + dx, o.y + dy, 8., col(0xf2f5ec));
        ui.center(s, Rect::new(o.x + dx - 8., o.y + dy - 8., 16., 16.), 9., c)
    }
}
fn lod_strip(ui: &Ui, e: &mut Editor) {
    draw_rectangle(258., 802., 864., 66., col(0xf3f5ee));
    draw_line(258., 802., 1122., 802., 1., line());
    if ui.button(
        "Auto LOD   0",
        Rect::new(280., 813., 168., 44.),
        if e.auto_lod { 2 } else { 0 },
    ) {
        e.auto_lod = true;
    }
    let labels: &[(&str, &str)] = if e.game_preview {
        &[("Near", "Folded leaves"), ("Mid", "Flat leaves"), ("Far", "Spray cards"), ("Distant", "Spray cards")]
    } else { &[("LOD 0", "Close up"), ("LOD 1", "Mid distance"), ("LOD 2", "Far distance")] };
    for (i, (name, desc)) in labels
    .iter()
    .enumerate()
    {
        let r = if e.game_preview { Rect::new(450. + i as f32 * 165., 813., 157., 44.) } else { Rect::new(467. + i as f32 * 209., 813., 199., 44.) };
        let active = !e.auto_lod && e.lod == i as u32;
        rounded(r, 6., if active { col(0xe0ead6) } else { col(0xebeee5) });
        ui.text(
            name,
            r.x + 12.,
            r.y + 18.,
            12.,
            if active { green() } else { ink() },
        );
        ui.text(desc, r.x + 12., r.y + 33., 10., muted());
        let count = if e.game_preview { String::new() } else { commas(e.model.lod_counts[i]) };
        ui.text(
            &count,
            r.x + r.w - 12. - ui.width(&count, 15.),
            r.y + 27.,
            15.,
            ink(),
        );
        if ui.over(r) && is_mouse_button_pressed(MouseButton::Left) {
            e.set_lod(i as u32)
        }
    }
}
fn footer(ui: &Ui, e: &Editor) {
    draw_rectangle(0., 868., W, 32., col(0x21372b));
    draw_circle(19., 884., 3., mint());
    ui.text(&e.status, 30., 889., 11., col(0xc9d8bf));
    ui.text(
        "Drag: orbit    /    Wheel: zoom    /    Middle drag: pan    /    Space: variation",
        793.,
        889.,
        10.,
        col(0x9db296),
    );
}
fn commas(n: usize) -> String {
    if n >= 1000 {
        format!("{},{:03}", n / 1000, n % 1000)
    } else {
        n.to_string()
    }
}

fn modal(ui: &Ui, e: &mut Editor, m: Modal) {
    draw_rectangle(0., 0., W, H, Color::new(0.07, 0.07, 0.08, 0.58));
    let r = Rect::new(408., 216., 624., 470.);
    rounded(
        Rect::new(r.x + 6., r.y + 8., r.w, r.h),
        12.,
        Color::new(0., 0., 0., 0.12),
    );
    rounded(r, 12., col(0xf8faf4));
    let title = match m {
        Modal::Export => "From grove to game.",
        Modal::Save => "Save your tree",
        Modal::Open => "Open a tree project",
        Modal::Automation => "Trees, on autopilot.",
        Modal::Help => "Make yourself at home.",
        Modal::BarkColor => "Bark color image",
        Modal::BarkHeight => "Bark height image",
        Modal::BarkNormal => "Bark normal image",
        Modal::LeafAtlas => "Leaf color atlas",
    };
    ui.text(title, 442., 262., 27., ink());
    if ui.button("x", Rect::new(979., 237., 27., 28.), 2) || is_key_pressed(KeyCode::Escape) {
        e.modal = None;
        return;
    }
    if m == Modal::Automation {
        ui.text(
            "The same Rust generator. With or without the editor.",
            442.,
            290.,
            13.,
            muted(),
        );
        ui.text("HEADLESS GENERATION", 442., 331., 11., green());
        rounded(Rect::new(442., 345., 554., 67.), 5., col(0xe9eee1));
        ui.text(
            "grove-studio --headless --preset solstice-oak --seed 42",
            457.,
            371.,
            12.,
            ink(),
        );
        ui.text("  --out exports/oak.gltf --lod all", 457., 393., 12., ink());
        ui.text("CONNECT YOUR MCP CLIENT", 442., 451., 11., green());
        ui.text(
            "Command: full path to grove-studio.exe",
            442.,
            479.,
            14.,
            ink(),
        );
        ui.text(
            "Arguments: --mcp --output-root <export folder>",
            442.,
            503.,
            12.,
            muted(),
        );
        ui.text(
            "Tools: tree_presets, tree_randomize, tree_generate, tree_preview",
            442.,
            540.,
            13.,
            ink(),
        );
        ui.text(
            "Seeded settings + JSON results. No window required.",
            442.,
            566.,
            12.,
            muted(),
        );
        if ui.button("Back to the workshop", Rect::new(442., 613., 554., 40.), 1) {
            e.modal = None
        }
        return;
    }
    if m == Modal::Help {
        ui.text(
            "Everything you need to find your next tree.",
            442.,
            292.,
            13.,
            muted(),
        );
        for (i, (a, b)) in [
            ("Left / right drag", "Orbit the tree"),
            ("Middle drag / wheel", "Pan / zoom"),
            ("F / G / W", "Frame / grid / wireframe"),
            ("Space / 0, 1, 2, 3", "New variation / detail level"),
            ("B / T", "Wind / light and dark theme"),
            ("Ctrl+S / Ctrl+O", "Save / open settings"),
            ("Ctrl+Z / Ctrl+Y", "Undo / redo"),
        ]
        .iter()
        .enumerate()
        {
            let y = 337. + i as f32 * 38.;
            ui.text(a, 442., y, 14., green());
            ui.text(b, 699., y, 14., ink());
        }
        if ui.button("Let's grow", Rect::new(442., 613., 554., 40.), 1) {
            e.modal = None
        }
        return;
    }
    let material = matches!(m, Modal::BarkColor | Modal::BarkHeight | Modal::BarkNormal | Modal::LeafAtlas);
    let export = m == Modal::Export;
    ui.text(
        if material { "Paste a PNG path, or leave empty to use the generated texture." } else if export {
            "Two materials, vertex colors, and detail levels ready to import."
        } else {
            "Settings are stored as a portable, editable JSON project."
        },
        442.,
        291.,
        12.,
        muted(),
    );
    ui.text(
        if material { "PNG FILE" } else if export {
            "OUTPUT FILE"
        } else {
            "PROJECT FILE"
        },
        442.,
        329.,
        10.,
        green(),
    );
    let field = Rect::new(442., 343., 554., 43.);
    rounded(field, 5., col(0xeaf0e2));
    if e.path_focus {
        draw_rectangle_lines(field.x, field.y, field.w, field.h, 1.5, green())
    }
    let max_chars = 62;
    let displayed = if e.path.chars().count() > max_chars {
        format!(
            "...{}",
            e.path
                .chars()
                .skip(e.path.chars().count() - max_chars)
                .collect::<String>()
        )
    } else {
        e.path.clone()
    };
    ui.text(&displayed, 454., 370., 14., ink());
    if is_mouse_button_pressed(MouseButton::Left) {
        e.path_focus = field.contains(ui.mouse)
    }
    if e.path_focus {
        let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
        if ctrl && is_key_pressed(KeyCode::V) {
            if let Some(s) = macroquad::miniquad::window::clipboard_get() {
                e.path
                    .push_str(&s.chars().filter(|c| !c.is_control()).collect::<String>());
            }
        }
        if ctrl && is_key_pressed(KeyCode::C) {
            macroquad::miniquad::window::clipboard_set(&e.path);
        }
        if (is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl))
            && is_key_pressed(KeyCode::A)
        {
            e.path.clear()
        }
        while let Some(c) = get_char_pressed() {
            if !c.is_control() {
                e.path.push(c)
            }
        }
        if is_key_pressed(KeyCode::Backspace) {
            e.path.pop();
        }
    }
    if material {
        ui.text("Leaf atlas: four variants across, stalk at the bottom of each tile.", 442., 428., 12., muted());
        ui.text("Match bark color, normal and height maps for best parallax.", 442., 453., 12., muted());
        ui.text("Project files reference these images; keep them available.", 442., 478., 12., muted());
        if ui.button("Use generated texture", Rect::new(442., 515., 554., 32.), 0) { e.path.clear(); }
        if ui.button("Apply material", Rect::new(442., 613., 554., 40.), 1) || is_key_pressed(KeyCode::Enter) {
            let mut candidate = e.s.clone();
            let path = if e.path.trim().is_empty() { None } else { Some(e.path.trim().trim_matches('"').to_string()) };
            match m {
                Modal::BarkColor => candidate.bark_color_path = path,
                Modal::BarkHeight => candidate.bark_height_path = path,
                Modal::BarkNormal => candidate.bark_normal_path = path,
                Modal::LeafAtlas => candidate.leaf_atlas_path = path,
                _ => unreachable!(),
            }
            match candidate.validate() {
                Ok(()) => { e.s = candidate; e.rebuild(); e.notice("Material updated."); e.modal = None; }
                Err(err) => e.notice(format!("Could not load material: {err}")),
            }
        }
        if get_time() < e.status_until && e.status.starts_with("Could not") { ui.text(&e.status.chars().take(80).collect::<String>(), 442., 590., 11., col(0xad553f)); }
        return;
    }
    if export {
        ui.text("FORMAT", 442., 423., 10., green());
        for (i, (id, label)) in [
            ("gltf", "glTF 2.0"),
            ("obj", "Wavefront OBJ"),
            ("p3d", "Poseidon P3D"),
        ]
        .iter()
        .enumerate()
        {
            let rr = Rect::new(442. + i as f32 * 186., 436., 182., 38.);
            if ui.button(label, rr, if e.format == *id { 2 } else { 0 }) {
                e.format = id.to_string();
                let p = Path::new(&e.path).with_extension(id);
                e.path = p.to_string_lossy().to_string();
            }
        }
        if ui.button(
            if e.format == "p3d" && e.game_lods { "[x]  Game LOD ladder (alpha cards)" }
            else if e.format == "p3d" { "[ ]  Game LOD ladder (alpha cards)" }
            else if e.all_lods {
                "[x]  Export all three detail levels"
            } else {
                "[ ]  Export all three detail levels"
            },
            Rect::new(442., 491., 554., 33.),
            0,
        ) {
            if e.format == "p3d" { e.game_lods = !e.game_lods; } else { e.all_lods = !e.all_lods; }
        }
        if e.format == "p3d" && e.game_lods && ui.button(
            if e.trim_cards { "[x]  Trim transparent card edges" } else { "[ ]  Trim transparent card edges" },
            Rect::new(442., 525., 554., 25.), 0) { e.trim_cards = !e.trim_cards; }
        if e.format == "p3d" && !e.game_lods && ui.button(
            if e.all_lods { "[x]  Export all three detail levels" } else { "[ ]  Export all three detail levels" },
            Rect::new(442., 525., 554., 25.), 0) { e.all_lods = !e.all_lods; }
        ui.text(
            match e.format.as_str() {
                "gltf" => "Includes a .bin buffer per model. Keep it beside the .gltf file.",
                "p3d" if e.game_lods => "Game cards and trimming can be compared in the viewer.",
                "p3d" => "One native model with texture, material and packing sidecars.",
                _ => "Includes a material file. Import foliage as double sided.",
            },
            442.,
            560.,
            12.,
            muted(),
        );
        ui.text(
            if e.format == "p3d" && e.game_lods && e.trim_cards {
                "Trimming increases triangle counts; measure performance in game."
            } else { "Files with the same names will be replaced." },
            442.,
            580.,
            11.,
            muted(),
        );
    } else {
        ui.text(
            "Tip: use the same project from the command line with --config.",
            442.,
            427.,
            12.,
            muted(),
        );
        ui.text(
            "Settings include the species, seed, shape, and foliage palette.",
            442.,
            453.,
            12.,
            muted(),
        );
        if m == Modal::Save {
            ui.text(
                "An existing file at this location will be replaced.",
                442.,
                490.,
                12.,
                muted(),
            );
        }
    }
    if ui.button(
        if export {
            "Export tree models  >"
        } else if m == Modal::Save {
            "Save project"
        } else {
            "Open project"
        },
        Rect::new(442., 613., 554., 40.),
        1,
    ) || is_key_pressed(KeyCode::Enter)
    {
        let result = if export {
            e.export().map(|paths| {
                format!(
                    "Exported {} model(s) to {}",
                    paths.len(),
                    Path::new(&e.path)
                        .parent()
                        .unwrap_or(Path::new("."))
                        .display()
                )
            })
        } else {
            e.project_action(m == Modal::Save).map(|_| {
                if m == Modal::Save {
                    "Project saved.".into()
                } else {
                    "Project opened.".into()
                }
            })
        };
        match result {
            Ok(s) => {
                e.notice(s);
                e.modal = None
            }
            Err(err) => {
                e.notice(format!("Could not complete: {err}"));
            }
        }
    }
    if get_time() < e.status_until && e.status.starts_with("Could not") {
        ui.text(
            &e.status.chars().take(84).collect::<String>(),
            442.,
            598.,
            11.,
            col(0xad553f),
        );
    }
}
