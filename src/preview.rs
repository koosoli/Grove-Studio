//! Bridge between the editor and the native Solstice render pipeline.
use crate::tree::{self, Part, Settings};
use glam::{Mat4, Vec2, Vec3};
use image::imageops::FilterType;
use solstice_asset::{Instance, LoadedScene, Material, SceneAsset, TextureLayer, VirtualMesh};
use solstice_builder::{build_virtual_mesh, mip_chain, paginate, BuildOptions, Mesh, Vertex};
use solstice_render::{
    Camera, DebugView, Environment, Gpu, Place, Renderer, RendererSettings, TimeOfDay, Weather,
};
use std::{
    collections::HashMap,
    sync::{atomic::{AtomicU32, Ordering}, mpsc, Mutex, OnceLock},
    time::Duration,
};

pub const SIZE: (u32, u32) = (768, 648);
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const TEX_SIZE: u32 = 256;

#[derive(Clone, Debug, PartialEq)]
pub struct SceneKey {
    pub settings: Settings,
    pub lod: Option<u32>,
    pub forest: u128,
    pub count: u32,
    pub ratios: [u32; SPECIES.len()],
    pub grid: bool,
    pub bark: bool,
    pub leaves: bool,
    pub game_cards: bool,
    pub trim_cards: bool,
    pub random_mix: bool,
    pub leaf_specular: bool,
    pub leaf_roughness: f32,
    pub leaf_transmission: f32,
}

pub const SPECIES: [&str; crate::catalog::IDS.len()] = crate::catalog::IDS;


/// Sky presets, by the names the renderer and the CLI use, with the editor's labels.
pub const SKYBOXES: [(&str, &str); 6] = [
    ("clear", "Clear"),
    ("fair", "Fair"),
    ("broken", "Broken"),
    ("overcast", "Overcast"),
    ("storm", "Storm"),
    ("cirrus", "Cirrus"),
];

pub fn skybox(name: &str) -> Result<Weather, String> {
    Weather::by_name(name).ok_or_else(|| {
        format!(
            "Unknown skybox {name:?}. Choose one of: {}",
            SKYBOXES.map(|(id, _)| id).join(", ")
        )
    })
}

pub fn forest_mask(names: &str) -> Result<u128, String> {
    names.split(',').try_fold(0, |mask, name| {
        SPECIES
            .iter()
            .position(|species| species.eq_ignore_ascii_case(name.trim()))
            .map(|index| mask | 1 << index)
            .ok_or_else(|| format!("Unknown forest species {name:?}"))
    })
}

pub fn height_hint(settings: &Settings) -> f32 {
    if settings.species == "solstice-bush" {
        settings.height * 6.
    } else {
        settings.height
    }
}

pub fn scene_height(settings: &Settings, forest: u128) -> f32 {
    if forest == 0 {
        return height_hint(settings);
    }
    SPECIES
        .iter()
        .enumerate()
        .filter(|(i, _)| forest & (1 << i) != 0)
        .map(|(_, species)| {
            if *species == settings.species {
                height_hint(settings)
            } else {
                Settings::preset(species).map(|s| height_hint(&s)).unwrap_or(1.)
            }
        })
        .fold(1., f32::max)
}

/// The stand for group shares: which species are planted, and each one's whole-percent share.
/// A group's share is split evenly across its species.
pub fn stand(shares: &[f32]) -> (u128, [u32; SPECIES.len()]) {
    let mut mask = 0u128;
    let mut ratios = [0u32; SPECIES.len()];
    for (group, (_, members)) in crate::catalog::GROUPS.iter().enumerate() {
        let share = shares.get(group).copied().unwrap_or(0.);
        if share <= 0. {
            continue;
        }
        for member in members.iter() {
            if let Some(index) = SPECIES.iter().position(|s| s == member) {
                mask |= 1 << index;
                ratios[index] = ((share * 100. / members.len() as f32).round() as u32).max(1);
            }
        }
    }
    (mask, ratios)
}
/// Spacing between neighbouring trees of a stand, in metres: close enough for a closed canopy.
pub fn forest_spacing(settings: &Settings, forest: u128) -> f32 {
    scene_height(settings, forest) * 0.42
}

/// Distance that frames a stand of count trees on its square grid.
pub fn frame_distance(settings: &Settings, forest: u128, count: u32) -> f32 {
    let height = scene_height(settings, forest);
    let side = (count.max(1) as f32).sqrt().ceil();
    (side * forest_spacing(settings, forest) * 1.5).max(height * 7.)
}

pub fn camera_for(settings: &Settings, forest: u128, count: u32, distance: Option<f32>) -> Camera {
    let yaw: f32 = 0.65;
    let pitch: f32 = if forest == 0 { 0.15 } else { 0.44 };
    let height = scene_height(settings, forest);
    let distance = distance.unwrap_or(if forest == 0 {
        height * 2.32
    } else {
        frame_distance(settings, forest, count)
    });    Camera {
        position: Vec3::Y * (height * 0.49)
            + Vec3::new(
                yaw.sin() * pitch.cos(),
                pitch.sin(),
                yaw.cos() * pitch.cos(),
            ) * distance,
        yaw,
        pitch: -pitch,
        fov_y: 40_f32.to_radians(),
        near: 0.05,
    }
}

pub struct Request {
    pub scene: SceneKey,
    pub camera: Camera,
    pub debug: DebugView,
    pub wind_time: Option<f32>,
    pub wind_direction: f32,
    pub wind_strength: f32,
    pub wind_gust: f32,
    pub leaf_flutter: f32,
    pub branch_sway: f32,
    pub branch_springiness: f32,
    pub sky: bool,
    pub skybox: Weather,
    pub lighting: Lighting,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lighting {
    pub hours: f32,
    /// Clockwise degrees from scene north (-Z).
    pub azimuth: f32,
}

impl Default for Lighting {
    fn default() -> Self {
        Self {
            hours: 10.5,
            azimuth: 205.,
        }
    }
}

impl Lighting {
    pub fn environment(self) -> Environment {
        let time = TimeOfDay {
            hours: self.hours,
            ..Default::default()
        };
        let mut place = Place::default();
        let sun = time.sky(&place).sun;
        // Rotating the scene's north changes the horizontal angle but preserves solar elevation.
        place.north =
            (sun.x.atan2(-sun.z) - self.azimuth.to_radians()).rem_euclid(std::f32::consts::TAU);
        time.environment(
            &place,
            &Environment {
                sky_intensity: 1.45,
                gi_intensity: 1.4,
                ..Default::default()
            },
        )
    }
}

pub struct Frame {
    pub scene: SceneKey,
    pub debug: DebugView,
    pub pixels: Vec<u8>,
    pub clusters: u32,
    pub triangles: u32,
}

pub struct Worker {
    send: mpsc::Sender<Request>,
    receive: mpsc::Receiver<Result<Frame, String>>,
    busy: bool,
}

impl Worker {
    pub fn start() -> Self {
        let (send, commands) = mpsc::channel();
        let (results, receive) = mpsc::channel();
        std::thread::spawn(move || {
            // GPU validation failures fall back to the editor's interactive renderer.
            let result = std::panic::catch_unwind(|| run_worker(commands, &results));
            let error = match result {
                Ok(Ok(())) => return,
                Ok(Err(error)) => error,
                Err(payload) => payload
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .or_else(|| payload.downcast_ref::<&str>().copied())
                    .unwrap_or("GPU renderer failed")
                    .to_owned(),
            };
            crate::log::log(format!("Solstice worker stopped: {error}"));
            let _ = results.send(Err(error));
        });
        Self {
            send,
            receive,
            busy: false,
        }
    }

    pub fn request(&mut self, request: Request) {
        if !self.busy && self.send.send(request).is_ok() {
            self.busy = true;
        }
    }

    pub fn poll(&mut self) -> Option<Result<Frame, String>> {
        let frame = self.receive.try_recv().ok()?;
        self.busy = false;
        Some(frame)
    }
}

pub fn capture(request: Request, path: &std::path::Path) -> Result<Frame, String> {
    let mut worker = Worker::start();
    worker.request(request);
    let frame = worker
        .receive
        .recv_timeout(Duration::from_secs(120))
        .map_err(|error| format!("Waiting for Solstice preview: {error}"))??;
    let image = image::RgbaImage::from_raw(SIZE.0, SIZE.1, frame.pixels.clone())
        .ok_or("Solstice returned an incomplete frame")?;
    let mut buffer = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut buffer, image::ImageOutputFormat::Png)
        .map_err(|e| e.to_string())?;
    tree::write(path, buffer.into_inner())?;
    Ok(frame)
}

fn run_worker(
    requests: mpsc::Receiver<Request>,
    results: &mpsc::Sender<Result<Frame, String>>,
) -> Result<(), String> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let gpu = pollster::block_on(Gpu::request(
        &instance,
        None,
        wgpu::PowerPreference::HighPerformance,
    ))
    .map_err(|error| error.to_string())?;
    crate::log::log("Solstice: GPU ready");
    let extent = wgpu::Extent3d {
        width: SIZE.0,
        height: SIZE.1,
        depth_or_array_layers: 1,
    };
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Grove viewport"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let row_bytes = SIZE.0 * 4;
    debug_assert_eq!(row_bytes % wgpu::COPY_BYTES_PER_ROW_ALIGNMENT, 0);
    let staging = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Grove viewport transfer"),
        size: u64::from(row_bytes) * u64::from(SIZE.1),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut active: Option<(SceneKey, Renderer)> = None;
    let (bake_send, bake_receive) = mpsc::channel();
    let mut baking = false;
    while let Ok(request) = requests.recv() {
        let mut ready = None;
        if baking {
            if let Ok((key, result)) = bake_receive.try_recv() {
                baking = false;
                if key == request.scene {
                    ready = Some((key, result?));
                }
            }
        }
        if active.is_none() {
            crate::log::log(format!("Solstice: building first scene ({} trees)", request.scene.count));
            ready = Some((request.scene.clone(), build_scene(&request.scene)?));
            crate::log::log("Solstice: first scene built");
        } else if !baking
            && active
                .as_ref()
                .is_some_and(|(key, _)| key != &request.scene)
            && ready.is_none()
        {
            let key = request.scene.clone();
            let sender = bake_send.clone();
            baking = true;
            std::thread::spawn(move || {
                let result = std::panic::catch_unwind(|| build_scene(&key))
                    .unwrap_or_else(|_| Err("Scene preparation failed".into()));
                let _ = sender.send((key, result));
            });
        }
        let new_scene = ready.is_some();
        if let Some((key, scene)) = ready {
            let settings = RendererSettings {
                lod_threshold: if key.lod.is_some() { 0. } else { 1. },
                render_scale: 0.8,
                // Cloud passes are only built when the renderer is created, so they run from
                // the start and "Sky off" selects the clear skybox instead.
                clouds: true,
                probe_rays: 6,
                particle_pool: 512,
                ..Default::default()
            };
            active = Some((key, Renderer::new(&gpu, &scene, FORMAT, settings)));
        }
        let (active_key, renderer) = active.as_mut().unwrap();
        let environment = request.lighting.environment();
        renderer.settings_mut().debug_view = request.debug;
        renderer.settings_mut().vegetation_wind = request
            .wind_time
            .map(|t| t % 10000. + ((request.wind_direction / 22.5).round() % 16.) * 10000.);
        renderer.settings_mut().vegetation_wind_parameters = glam::Vec4::new(request.wind_strength, request.wind_gust, request.leaf_flutter, request.branch_sway);
        renderer.settings_mut().vegetation_wind_dynamics = glam::Vec4::new(request.branch_springiness, 0., 0., 0.);
        renderer.settings_mut().show_sky = request.sky;
        renderer.settings_mut().clouds = request.sky;
        // A fixed skybox: the renderer's own weather cycle stays off while the editor drives it.
        let weather = if request.sky {
            request.skybox
        } else {
            skybox("clear")?
        };
        renderer.set_weather(weather, 0.);
        renderer.settings_mut().temporal_accumulation = request.debug == DebugView::Lit;
        // Allow the initial GI and visibility histories to accumulate before displaying a scene.
        for _ in 0..if new_scene { 5 } else { 1 } {
            renderer.render(&gpu, &request.camera, &environment, &view, SIZE, 1. / 60.);
            if new_scene {
                // Let the asynchronous culling statistics map and get collected by the
                // following frame, including when this is a single offscreen capture.
                gpu.device
                    .poll(wgpu::PollType::Wait {
                        submission_index: None,
                        timeout: Some(Duration::from_secs(15)),
                    })
                    .map_err(|error| error.to_string())?;
            }
        }
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row_bytes),
                    rows_per_image: Some(SIZE.1),
                },
            },
            extent,
        );
        gpu.queue.submit([encoder.finish()]);
        let (mapped, ready) = mpsc::sync_channel(1);
        staging
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = mapped.send(result);
            });
        gpu.device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(15)),
            })
            .map_err(|error| error.to_string())?;
        ready
            .recv_timeout(Duration::from_secs(1))
            .map_err(|error| error.to_string())?
            .map_err(|error| error.to_string())?;
        let pixels = staging
            .slice(..)
            .get_mapped_range()
            .map_err(|error| error.to_string())?
            .to_vec();
        staging.unmap();
        let stats = renderer.stats();
        if results
            .send(Ok(Frame {
                scene: active_key.clone(),
                debug: request.debug,
                pixels,
                clusters: stats.clusters,
                triangles: stats.triangles,
            }))
            .is_err()
        {
            break;
        }
    }
    Ok(())
}

fn to_mesh(part: &Part) -> Mesh {
    Mesh {
        vertices: part
            .v
            .iter()
            .map(|v| Vertex {
                position: Vec3::from_array(v.p.to_array()),
                normal: Vec3::from_array(v.n.to_array()),
                uv: Vec2::from_array(v.uv.to_array()),
            })
            .collect(),
        indices: part.i.clone(),
    }
}

/// Give each disconnected preview leaf a stable phase without changing its fractional atlas UV.
/// The renderer's repeat sampler keeps the same texture; game exports never use this encoding.
fn foliage_mesh(part: &Part, game_cards: bool) -> Mesh {
    let mut parent: Vec<usize> = (0..part.v.len()).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i { parent[i] = parent[parent[i]]; i = parent[i]; }
        i
    }
    for triangle in part.i.chunks_exact(3) {
        let a = root(&mut parent, triangle[0] as usize);
        for &index in &triangle[1..] {
            let b = root(&mut parent, index as usize);
            if a != b { parent[b] = a; }
        }
    }
    let mut anchors = vec![None::<usize>; part.v.len()];
    for i in 0..part.v.len() {
        let r = root(&mut parent, i);
        if anchors[r].is_none_or(|old| part.v[i].uv.y < part.v[old].uv.y) { anchors[r] = Some(i); }
    }
    let mut mesh = to_mesh(part);
    for i in 0..part.v.len() {
        let r = root(&mut parent, i);
        if let Some(anchor) = anchors[r] {
            let p = part.v[anchor].p;
            let x = (p.x * 1000.).round() as i32 as u32;
            let y = (p.y * 1000.).round() as i32 as u32;
            let z = (p.z * 1000.).round() as i32 as u32;
            let hash = x.wrapping_mul(0x9e3779b1) ^ y.wrapping_mul(0x85ebca6b) ^ z.wrapping_mul(0xc2b2ae35);
            let phase = ((hash ^ (hash >> 16)) % 16) as f32;
            mesh.vertices[i].uv.x += phase + if game_cards { 0. } else { 16. };
        }
    }
    mesh
}

fn layer(color: Vec<u8>, normal: Vec<u8>, data: Vec<u8>) -> TextureLayer {
    TextureLayer {
        color_mips: mip_chain(color, TEX_SIZE, 4),
        normal_mips: mip_chain(normal, TEX_SIZE, 2),
        material_mips: mip_chain(data, TEX_SIZE, 2),
    }
}

fn bark_layer(settings: &Settings) -> Result<TextureLayer, String> {
    let load = |bytes: &[u8]| {
        image::load_from_memory(bytes)
            .map(|img| img.resize_exact(TEX_SIZE, TEX_SIZE, FilterType::Triangle))
            .map_err(|err| err.to_string())
    };
    let mut color = load(tree::bark_texture(settings))?.into_rgba8();
    for pixel in color.pixels_mut() {
        pixel.0[3] = 255;
    }
    let (height, normal) = tree::relief_textures(settings);
    let normal = load(normal)?
        .into_rgb8()
        .pixels()
        .flat_map(|p| [p[0], p[1]])
        .collect();
    let data = load(height)?
        .into_luma8()
        .pixels()
        .flat_map(|p| [128, p[0]])
        .collect();
    Ok(layer(color.into_raw(), normal, data))
}

pub(crate) fn leaf_rgba(settings: &Settings) -> Vec<u8> {
    if settings.season == 0 {
        if let Some(atlas) = crate::reference::leaf_atlas(settings) {
            return atlas.to_vec();
        }
    }
    let mut color = Vec::new();
    for y in 0..TEX_SIZE {
        for x in 0..TEX_SIZE {
            let across = ((x as f32 + 0.5) / (TEX_SIZE / 4) as f32).fract() * 2. - 1.;
            let along = y as f32 / TEX_SIZE as f32;
            let rib = (-across.abs() * 19.).exp();
            let phase = along * 8. - across.abs() * 1.8;
            let veins = (-(phase - phase.round()).abs() * 12.).exp() * (1. - across.abs());
            let shade = 1. + 0.23 * rib + 0.13 * veins - 0.12 * across.abs().powi(3);
            let c = tree::leaf_color(settings, (x / (TEX_SIZE / 4)) as usize);
            color.extend(
                c[..3]
                    .iter()
                    .map(|v| (tree::srgb(v * shade) * 255.).round() as u8),
            );
            color.push(255);
        }
    }
    color
}

fn leaf_layer(settings: &Settings) -> TextureLayer {
    let n = (TEX_SIZE * TEX_SIZE * 2) as usize;
    layer(leaf_rgba(settings), vec![128; n], vec![128; n])
}

/// Places `count` trees on a jittered square grid. Each tree goes to the species furthest below
/// its share of `weights`, so the mix follows the percentages at any count, and the species are
/// spread evenly rather than in blocks. Returns each tree's species index and transform.
pub fn place_forest(count: u32, weights: &[u32], spacing: f32) -> Vec<(usize, Mat4)> {
    let mut out = Vec::with_capacity(count as usize);
    if count == 0 || weights.iter().all(|&w| w == 0) {
        return out;
    }
    let side = (count as f32).sqrt().ceil() as u32;
    let mut placed = vec![0u64; weights.len()];
    for i in 0..count {
        let kind = (0..weights.len())
            .filter(|&k| weights[k] > 0)
            .min_by_key(|&k| placed[k] * 1000 / weights[k] as u64)
            .unwrap_or(0);
        placed[kind] += 1;
        let (x, z) = ((i % side) as f32 - side as f32 / 2., (i / side) as f32 - side as f32 / 2.);
        let fi = i as f32;
        let jitter = Vec3::new(
            ((fi + 0.3) * 17.19).sin() * 0.35,
            0.,
            ((fi + 0.7) * 23.71).cos() * 0.35,
        );
        let translation = spacing * (Vec3::new(x, 0., z) + jitter);
        let rotation = glam::Quat::from_rotation_y(fi * 2.399963);
        let scale = 0.85 + ((fi + 0.5) * 9.13).sin().abs() * 0.3;
        out.push((
            kind,
            Mat4::from_scale_rotation_translation(Vec3::splat(scale), rotation, translation),
        ));
    }
    out
}

/// Shuffle assigned species, preserving the exact weighted counts and tree positions.
pub fn shuffle_forest(placed: &mut [(usize, Mat4)], seed: u32) {
    let mut state = u64::from(seed) ^ 0x9e3779b97f4a7c15;
    for i in (1..placed.len()).rev() {
        state ^= state >> 12;
        state ^= state << 25;
        state ^= state >> 27;
        let j = (state.wrapping_mul(0x2545f4914f6cdd1d) % (i as u64 + 1)) as usize;
        let k = placed[i].0;
        placed[i].0 = placed[j].0;
        placed[j].0 = k;
    }
}

/// How many species assets to keep. Editing a tree's settings makes new ones, so the cache is
/// cleared when it grows past this, instead of growing without limit.
const SPECIES_CACHE_LIMIT: usize = 64;

/// Built geometry and textures for one species at one LOD: the bark and the foliage.
#[derive(Clone)]
struct SpeciesAsset {
    bark: (VirtualMesh, TextureLayer, Material),
    leaves: (VirtualMesh, TextureLayer, Material),
}

static SPECIES_CACHE: OnceLock<Mutex<HashMap<String, SpeciesAsset>>> = OnceLock::new();
static BUILD_DONE: AtomicU32 = AtomicU32::new(0);
static BUILD_TOTAL: AtomicU32 = AtomicU32::new(0);

/// Progress of the species being built for the current scene: (built, total). Both are zero
/// when no stand is being built.
pub fn build_progress() -> (u32, u32) {
    (
        BUILD_DONE.load(Ordering::Relaxed),
        BUILD_TOTAL.load(Ordering::Relaxed),
    )
}

/// The species' geometry and textures, grown and meshed on first use and reused after that.
fn species_asset(settings: &Settings, lod: u32, game_cards: bool, trim_cards: bool, leaf_specular: bool, leaf_roughness: f32, leaf_transmission: f32) -> Result<SpeciesAsset, String> {
    let key = format!("{settings:?}|{lod}|{game_cards}|{trim_cards}|{leaf_specular}|{leaf_roughness}|{leaf_transmission}");
    let cache = SPECIES_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(hit) = cache.lock().map_err(|e| e.to_string())?.get(&key) {
        return Ok(hit.clone());
    }
    let started = std::time::Instant::now();
    let mut game = game_cards.then(|| crate::gamelod::ladder(settings));
    if trim_cards { if let Some(ladder) = game.as_mut() { crate::gamelod::trim_cards(ladder); } }
    let tree = tree::generate(settings, lod.min(2));
    crate::log::log(format!(
        "Solstice: grew {} LOD {lod} in {:.2} s ({} triangles)",
        settings.species,
        started.elapsed().as_secs_f32(),
        tree.triangles()
    ));
    let opts = BuildOptions::default();
    let part = |foliage: bool, data: &Part| -> Result<(VirtualMesh, TextureLayer, Material), String> {
        let meshing = std::time::Instant::now();
        let source = if foliage { foliage_mesh(data, game_cards) } else { to_mesh(data) };
        let mesh = build_virtual_mesh(&source, &opts);
        crate::log::log(format!(
            "Solstice: {} {} meshed in {:.2} s",
            settings.species,
            if foliage { "leaves" } else { "bark" },
            meshing.elapsed().as_secs_f32()
        ));
        let layer = if foliage && game_cards {
            let atlas = image::RgbaImage::from_raw(crate::gamelod::ATLAS, crate::gamelod::ATLAS, game.as_ref().unwrap().atlas.clone()).unwrap();
            let color = image::imageops::resize(&atlas, TEX_SIZE, TEX_SIZE, FilterType::Triangle).into_raw();
            let n = (TEX_SIZE * TEX_SIZE * 2) as usize;
            layer(color, vec![128; n], vec![128; n])
        } else if foliage {
            leaf_layer(settings)
        } else {
            bark_layer(settings)?
        };
        let material = Material {
            base_color: [1.; 4],
            roughness: if foliage { leaf_roughness } else { 0.92 },
            flags: solstice_render::scene::MATERIAL_WIND
                | if foliage {
                    Material::TWO_SIDED | Material::FOLIAGE | 128 | ((leaf_transmission * 100.).round() as u32) << 16 | if game_cards { solstice_render::scene::MATERIAL_ALPHA_TEST } else { 0 } | if !leaf_specular { solstice_render::scene::MATERIAL_NO_SPECULAR } else { 0 }
                } else {
                    ((settings.parallax_depth * 1000.).round() as u32) << 8
                },
            texture_layer: 0,
            normal_strength: if foliage { 0. } else { 1. },
            ..Default::default()
        };
        Ok((mesh, layer, material))
    };
    let (bark, leaves) = if let Some(ref ladder) = game {
        let level = &ladder.levels[(lod as usize).min(ladder.levels.len()-1)];
        (&level.bark, &level.leaves)
    } else { (&tree.bark, &tree.leaves) };
    let asset = SpeciesAsset {
        bark: part(false, bark)?,
        leaves: part(true, leaves)?,
    };
    let mut cache = cache.lock().map_err(|e| e.to_string())?;
    if cache.len() >= SPECIES_CACHE_LIMIT {
        cache.clear();
    }
    cache.insert(key, asset.clone());
    Ok(asset)
}
fn build_scene(key: &SceneKey) -> Result<LoadedScene, String> {
    let mut scene = SceneAsset {
        texture_size: TEX_SIZE,
        ..Default::default()
    };
    let opts = BuildOptions::default();
    let ground = Mesh {
        vertices: [(-128., -128.), (-128., 128.), (128., 128.), (128., -128.)]
            .map(|(x, z)| Vertex {
                position: Vec3::new(x, -0.01, z),
                normal: Vec3::Y,
                uv: Vec2::new(x, z),
            })
            .to_vec(),
        indices: vec![0, 1, 2, 0, 2, 3],
    };
    let n = (TEX_SIZE * TEX_SIZE * 2) as usize;
    let grid = (0..TEX_SIZE)
        .flat_map(|y| {
            (0..TEX_SIZE).flat_map(move |x| {
                if key.grid && (x < 3 || y < 3) {
                    [110, 110, 110, 255]
                } else if key.forest != 0 {
                    [109, 128, 91, 255]
                } else {
                    [159, 159, 159, 255]
                }
            })
        })
        .collect();
    scene.textures.push(layer(grid, vec![128; n], vec![128; n]));
    scene.meshes.push(build_virtual_mesh(&ground, &opts));
    scene.materials.push(Material {
        base_color: [1.; 4],
        roughness: 1.,
        texture_layer: 0,
        normal_strength: 0.,
        ..Default::default()
    });
    scene.instances.push(Instance {
        mesh: 0,
        material: 0,
        transform: Mat4::IDENTITY,
    });

    let mut trees = Vec::new();
    if key.forest == 0 {
        trees.push(key.settings.clone());
    } else {
        for (i, species) in SPECIES.iter().enumerate() {
            if key.forest & (1 << i) != 0 {
                trees.push(if key.settings.species == *species {
                    key.settings.clone()
                } else {
                    Settings::preset(species)?
                });
            }
        }
    }
    let mut assets = Vec::new();
    BUILD_TOTAL.store(trees.len() as u32, Ordering::Relaxed);
    BUILD_DONE.store(0, Ordering::Relaxed);
    for settings in &trees {
        let asset = species_asset(settings, key.lod.unwrap_or(0), key.game_cards, key.trim_cards, key.leaf_specular, key.leaf_roughness, key.leaf_transmission)?;
        let first = scene.meshes.len() as u32;
        for (mesh, layer, material) in [&asset.bark, &asset.leaves] {
            scene.meshes.push(mesh.clone());
            let texture_layer = scene.textures.len() as i32;
            scene.textures.push(layer.clone());
            scene.materials.push(Material {
                texture_layer,
                ..*material
            });
        }
        assets.push(first);
        BUILD_DONE.fetch_add(1, Ordering::Relaxed);
    }
    let mut place = |first, transform| {
        for (offset, visible) in [(0, key.bark), (1, key.leaves)] {
            if visible {
                let index = first + offset;
                scene.instances.push(Instance {
                    mesh: index,
                    material: index,
                    transform,
                });
            }
        }
    };
    if key.forest == 0 {
        place(assets[0], Mat4::IDENTITY);
    } else {
        let spacing = forest_spacing(&key.settings, key.forest);
        let weights: Vec<u32> = SPECIES
            .iter()
            .enumerate()
            .filter(|(index, _)| key.forest & (1 << index) != 0)
            .map(|(index, _)| key.ratios[index])
            .collect();
        let mut placed = place_forest(key.count, &weights, spacing);
        if key.random_mix { shuffle_forest(&mut placed, key.settings.seed); }
        for (kind, transform) in placed {
            place(assets[kind], transform);
        }
    }
    Ok(paginate(&scene))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foliage_phase_is_shared_within_each_piece_and_keeps_atlas_coordinates() {
        let v = |x: f32, u: f32, v: f32| crate::tree::V { p: macroquad::prelude::vec3(x, v, 0.), n: macroquad::prelude::Vec3::Y, uv: macroquad::prelude::vec2(u,v), c: [1.;4] };
        let part = Part { v: vec![v(0.,0.1,0.),v(0.1,0.2,0.),v(0.05,0.15,1.),v(2.,0.1,0.),v(2.1,0.2,0.),v(2.05,0.15,1.)], i: vec![0,1,2,3,4,5] };
        let mesh = foliage_mesh(&part, false);
        for group in mesh.vertices.chunks_exact(3) {
            assert_eq!(group[0].uv.x.floor(), group[1].uv.x.floor());
            assert_eq!(group[1].uv.x.floor(), group[2].uv.x.floor());
        }
        assert_ne!(mesh.vertices[0].uv.x.floor(), mesh.vertices[3].uv.x.floor());
        for (original, encoded) in part.v.iter().zip(&mesh.vertices) {
            assert!((original.uv.x - encoded.uv.x.fract()).abs() < 0.00001);
        }
    }

    #[test]
    fn species_assets_are_reused_not_rebuilt() {
        let s = Settings::preset("solstice-bush").unwrap();
        let first = std::time::Instant::now();
        species_asset(&s, 2, false, false, true, 0.55, 0.8).unwrap();
        let first = first.elapsed();
        let again = std::time::Instant::now();
        species_asset(&s, 2, false, false, true, 0.55, 0.8).unwrap();
        assert!(again.elapsed() < first / 2 + std::time::Duration::from_millis(5), "first {first:?}, again {:?}", again.elapsed());
    }

    #[test]
    fn forest_places_the_requested_count_in_the_requested_shares() {
        for count in [1u32, 37, 200, 600] {
            let placed = place_forest(count, &[3, 1, 0], 6.);
            assert_eq!(placed.len(), count as usize);
            let threes = placed.iter().filter(|(k, _)| *k == 0).count() as i64;
            assert!((threes - count as i64 * 3 / 4).abs() <= 1, "{threes} of {count}");
            assert!(placed.iter().all(|(k, _)| *k != 2));
        }
        assert!(place_forest(10, &[0, 0], 6.).is_empty());
        assert_eq!(place_forest(50, &[1, 1], 6.), place_forest(50, &[1, 1], 6.));
    }

    #[test]
    fn random_mix_is_repeatable_and_preserves_shares_and_positions() {
        let ordered = place_forest(40, &[3, 1], 6.);
        let mut a = ordered.clone();
        let mut b = ordered.clone();
        shuffle_forest(&mut a, 42); shuffle_forest(&mut b, 42);
        assert_eq!(a, b);
        assert_ne!(a, ordered);
        assert_eq!(a.iter().filter(|(k, _)| *k == 0).count(), 30);
        assert!(a.iter().zip(&ordered).all(|(a,b)| a.1 == b.1));
    }

    #[test]
    fn solar_time_and_azimuth_control_different_axes() {
        let east = Lighting {
            hours: 10.5,
            azimuth: 90.,
        }
        .environment();
        let west = Lighting {
            hours: 10.5,
            azimuth: 270.,
        }
        .environment();
        let (east, west) = (east.sun_disc.unwrap(), west.sun_disc.unwrap());
        assert!(east.x > 0.3 && west.x < -0.3);
        assert!((east.y - west.y).abs() < 1e-5);
        let dark = Lighting {
            hours: 0.,
            azimuth: 90.,
        }
        .environment();
        assert!(dark.sun_disc.unwrap().y < 0. && dark.sun_illuminance < 0.1);
    }
}
