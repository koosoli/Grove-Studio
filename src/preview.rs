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
    pub grass: Option<crate::grass::Settings>,
    pub scene_grass: Option<crate::grass::Settings>,
    pub garden: bool,
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
    pub forest_floor: bool,
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
        settings.height * settings.maturity * 6.
    } else if settings.species == "solstice-willow" || settings.species == "solstice-old_oak" {
        // These specimen presets describe the trunk height; their rising limbs and wide
        // crowns extend well beyond it, so frame and space them as larger trees.
        settings.height * settings.maturity * 1.9
    } else {
        settings.height * settings.maturity
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
    pub size: (u32,u32),
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
    pub size: (u32,u32),
    pub scene: SceneKey,
    pub debug: DebugView,
    pub pixels: Vec<u8>,
    pub clusters: u32,
    pub triangles: u32,
    /// Most recently resolved GPU timings, when timestamp queries are available.
    pub gpu_ms: Option<[f32; solstice_render::profiler::SECTIONS.len()]>,
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
    let image = image::RgbaImage::from_raw(frame.size.0, frame.size.1, frame.pixels.clone())
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
    let mut active: Option<(SceneKey, Renderer)> = None;
    let (bake_send, bake_receive) = mpsc::channel();
    let mut baking = false;
    let mut targets: Option<((u32, u32), wgpu::Texture, wgpu::Buffer)> = None;
    while let Ok(request) = requests.recv() {
        let size = (
            request.size.0.clamp(256, 2048),
            request.size.1.clamp(256, 1440),
        );
        if targets.as_ref().is_none_or(|(old, _, _)| *old != size) {
            let extent = wgpu::Extent3d {
                width: size.0,
                height: size.1,
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
            let row_bytes = (size.0 * 4).div_ceil(256) * 256;
            debug_assert_eq!(row_bytes % wgpu::COPY_BYTES_PER_ROW_ALIGNMENT, 0);
            let staging = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Grove viewport transfer"),
                size: u64::from(row_bytes) * u64::from(size.1),
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            targets = Some((size, texture, staging));
        }
        let (_, texture, staging) = targets.as_ref().unwrap();
        let view = texture.create_view(&Default::default());
        let row_bytes = (size.0 * 4).div_ceil(256) * 256;
        let extent = wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        };

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
            crate::log::log(format!(
                "Solstice: building first scene ({} trees)",
                request.scene.count
            ));
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
        renderer.settings_mut().vegetation_wind_parameters = glam::Vec4::new(
            request.wind_strength,
            request.wind_gust,
            request.leaf_flutter,
            request.branch_sway,
        );
        renderer.settings_mut().vegetation_wind_dynamics =
            glam::Vec4::new(request.branch_springiness, 0., 0., 0.);
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
            renderer.render(&gpu, &request.camera, &environment, &view, size, 1. / 60.);
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
                    rows_per_image: Some(size.1),
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
        let pixels = {
            let mapped = staging
                .slice(..)
                .get_mapped_range()
                .map_err(|e| e.to_string())?;
            mapped
                .chunks_exact(row_bytes as usize)
                .flat_map(|row| row[..size.0 as usize * 4].iter().copied())
                .collect()
        };
        staging.unmap();
        let stats = renderer.stats();
        if results
            .send(Ok(Frame {
                size,
                scene: active_key.clone(),
                debug: request.debug,
                pixels,
                clusters: stats.clusters,
                triangles: stats.triangles,
                gpu_ms: renderer.section_times(),
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
    let mut color = load(&tree::bark_texture(settings)?)?.into_rgba8();
    for pixel in color.pixels_mut() {
        pixel.0[3] = 255;
    }
    let (height, normal) = tree::relief_textures(settings)?;
    let normal = load(&normal)?
        .into_rgb8()
        .pixels()
        .flat_map(|p| [p[0], p[1]])
        .collect();
    let data = load(&height)?
        .into_luma8()
        .pixels()
        .flat_map(|p| [128, p[0]])
        .collect();
    Ok(layer(color.into_raw(), normal, data))
}

pub(crate) fn leaf_rgba(settings: &Settings) -> Vec<u8> {
    if let Some(path) = &settings.leaf_atlas_path {
        match image::open(path) {
            Ok(image) => return image.resize_exact(TEX_SIZE, TEX_SIZE, FilterType::Triangle).into_rgba8().into_raw(),
            Err(err) => crate::log::log(format!("Could not load leaf atlas {path}: {err}; using generated atlas")),
        }
    }
    let mut color = crate::reference::leaf_atlas(settings).expect("known species").to_vec();
    if settings.season == 0 { return color; }
    // Recolor the actual species atlas while retaining its veins and mottling.
    let linear = |b: u8| ((b as f32 / 255. + 0.055) / 1.055).powf(2.4);
    let centers: [[f32;3];4] = std::array::from_fn(|variant| {
        let i = (128 * 256 + variant * 64 + 32) * 4;
        [linear(color[i]), linear(color[i+1]), linear(color[i+2])]
    });
    for (i, p) in color.chunks_exact_mut(4).enumerate() {
        let variant = (i % 256) / 64;
        let target = tree::leaf_color(settings, variant);
        for ch in 0..3 {
            let detail = (linear(p[ch]) / centers[variant][ch].max(0.001)).clamp(0.4, 1.8);
            p[ch] = (tree::srgb(target[ch] * detail) * 255.).round().clamp(0.,255.) as u8;
        }
    }
    color
}

/// Leaf relief for the original or game atlas. Spray cards keep neutral normals so tiny
/// baked silhouettes do not turn into glossy bumps; the individual-leaf bands use veins.
pub(crate) fn leaf_surface_maps(settings: &Settings, game: bool, edge: usize) -> (Vec<u8>, Vec<u8>) {
    let source = crate::reference::leaf_maps(settings).expect("known species");
    let mut normals = vec![128; edge * edge * 2];
    let mut data = vec![128; edge * edge * 2];
    if settings.leaf_atlas_path.is_some() { return (normals, data); }
    for y in 0..edge {
        let v = (y as f32 + 0.5) / edge as f32;
        let along = if !game { Some(v) } else {
            [crate::gamelod::OPAQUE, crate::gamelod::CUTOUT].iter()
                .find(|r| v >= r.0 && v <= r.1).map(|r| (v-r.0)/(r.1-r.0))
        };
        let Some(along) = along else { continue; };
        for x in 0..edge {
            let sx = (x * 256 / edge).min(255);
            let sy = ((along * 256.) as usize).min(255);
            let src = (sy*256+sx)*2; let dst = (y*edge+x)*2;
            normals[dst..dst+2].copy_from_slice(&source.normal_mips[0][src..src+2]);
            // Preserve the editor roughness slider's scale with subtle local variation.
            let r = source.material_mips[0][src] as f32;
            data[dst] = (128. + (r-128.) * 0.25).clamp(96.,160.) as u8;
        }
    }
    (normals, data)
}

fn leaf_layer(settings: &Settings) -> TextureLayer {
    let (normal, data) = leaf_surface_maps(settings, false, TEX_SIZE as usize);
    layer(leaf_rgba(settings), normal, data)
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

/// Seeded forest placement with local minimum spacing and a smaller understory age class.
/// Species counts remain the same as the ordered placement. A bounded retry gradually relaxes
/// spacing for unusually dense stands, so every requested instance is still placed.
pub fn natural_forest(count: u32, weights: &[u32], spacing: f32, seed: u32) -> Vec<(usize, Mat4)> {
    let mut placed = place_forest(count, weights, spacing);
    shuffle_forest(&mut placed, seed);
    if placed.is_empty() { return placed; }
    let mut state = u64::from(seed) ^ 0x6a09e667f3bcc909;
    let mut random = || {
        state ^= state >> 12; state ^= state << 25; state ^= state >> 27;
        ((state.wrapping_mul(0x2545f4914f6cdd1d) >> 40) as u32) as f32 / (1u32 << 24) as f32
    };
    let edge = (count as f32).sqrt().ceil() * spacing;
    let min_distance = spacing * 0.45;
    let cell_size = min_distance.max(0.001);
    let mut grid: HashMap<(i32,i32), Vec<Vec2>> = HashMap::new();
    for (_, transform) in &mut placed {
        let mut point = Vec2::ZERO;
        for attempt in 0..160 {
            point = Vec2::new((random() - 0.5) * edge, (random() - 0.5) * edge);
            let separation = min_distance * (1. - (attempt / 40) as f32 * 0.3).max(0.);
            let cell = ((point.x / cell_size).floor() as i32, (point.y / cell_size).floor() as i32);
            let mut clear = true;
            for dx in -1..=1 { for dy in -1..=1 {
                if grid.get(&(cell.0 + dx,cell.1 + dy)).is_some_and(|points| points.iter().any(|p| p.distance_squared(point) < separation * separation)) { clear = false; }
            }}
            if clear || attempt == 159 { break; }
        }
        let cell = ((point.x / cell_size).floor() as i32, (point.y / cell_size).floor() as i32);
        grid.entry(cell).or_default().push(point);
        let young = random() < 0.18;
        let scale = if young { 0.45 + random() * 0.25 } else { 0.82 + random() * 0.36 };
        let rotation = glam::Quat::from_rotation_y(random() * std::f32::consts::TAU);
        *transform = Mat4::from_scale_rotation_translation(Vec3::splat(scale), rotation, Vec3::new(point.x, 0., point.y));
    }
    placed
}

/// Procedural soil, moss and fallen leaves for the forest preview (sRGB RGBA).
fn forest_floor_texture(seed: u32, season: u32) -> Vec<u8> {
    let edge = TEX_SIZE as usize;
    let noise = |x: usize, y: usize| -> u32 {
        let mut h = (x as u32).wrapping_mul(0x8da6b343) ^ (y as u32).wrapping_mul(0xd8163841) ^ seed;
        h ^= h >> 13; h = h.wrapping_mul(0x85ebca6b); h ^ (h >> 16)
    };
    let mut pixels = vec![0u8; edge * edge * 4];
    for y in 0..edge { for x in 0..edge {
        let fine = (noise(x,y) % 15) as i32 - 7;
        let patch = noise(x/12,y/12) % 100;
        let base = if patch < 27 { [45, 59, 31] } else { [65, 53, 40] };
        let i = (y * edge + x) * 4;
        for c in 0..3 { pixels[i+c] = (base[c] + fine).clamp(0,255) as u8; }
        pixels[i+3] = 255;
    }}
    for n in 0..260usize {
        let h = noise(n, 7919); let cx = (h as usize) % edge; let cy = ((h >> 8) as usize) % edge;
        let angle = (h >> 16) as f32 / 65535. * std::f32::consts::TAU;
        let (sn,cs) = angle.sin_cos();
        let color: [u8;3] = if season == 2 { [104,91,49] } else if season == 1 { [143,94,45] } else { [104,76,45] };
        for dy in -5i32..=5 { for dx in -5i32..=5 {
            let u = dx as f32 * cs + dy as f32 * sn; let v = -dx as f32 * sn + dy as f32 * cs;
            if u*u / 20. + v*v / 3.0 <= 1. {
                let x = (cx as i32 + dx).rem_euclid(edge as i32) as usize;
                let y = (cy as i32 + dy).rem_euclid(edge as i32) as usize;
                let i = (y * edge + x) * 4;
                pixels[i..i+3].copy_from_slice(&color);
            }
        }}
    }
    pixels
}

/// A lightweight clump of individual, opaque understory leaves for forest context.
fn understory_mesh() -> Mesh {
    let mut mesh = Mesh { vertices: Vec::new(), indices: Vec::new() };
    for k in 0..12u32 {
        let angle = k as f32 * 2.399963;
        let (sn, cs) = angle.sin_cos();
        let radial = Vec3::new(cs, 0., sn);
        let side = Vec3::new(-sn, 0., cs);
        let length = 0.22 + (k % 5) as f32 * 0.04;
        let base = Vec3::new(0., 0.02, 0.);
        let middle = base + radial * length * 0.55 + Vec3::Y * (0.16 + (k % 3) as f32 * 0.03);
        let tip = base + radial * length + Vec3::Y * 0.13;
        let first = mesh.vertices.len() as u32;
        for (position, uv) in [(base, Vec2::new(0.5,0.)), (middle-side*0.035, Vec2::new(0.,0.5)), (middle+side*0.035, Vec2::new(1.,0.5)), (tip,Vec2::new(0.5,1.))] {
            mesh.vertices.push(Vertex { position, normal: Vec3::Y, uv });
        }
        mesh.indices.extend([first,first+1,first+2, first+1,first+3,first+2]);
    }
    mesh
}

/// How many species assets to keep. Editing a tree's settings makes new ones, so the cache is
/// cleared when it grows past this, instead of growing without limit.
const SPECIES_CACHE_LIMIT: usize = 64;

/// Built geometry and textures for one species at one LOD: the bark and the foliage.
#[derive(Clone)]
struct SpeciesAsset {
    bark: (VirtualMesh, TextureLayer, Material),
    leaves: (VirtualMesh, TextureLayer, Material),
    branches: Vec<tree::Branch>,
}

/// Retain a source stem id in the unused word of each packed vertex. The builder's edge
/// collapses select source positions; coincident positions use their shared ancestor so they
/// cannot tear apart when the tree moves.
fn attach_branch_ids(mesh: &mut VirtualMesh, source: &Part, branches: &[tree::Branch]) {
    if branches.len() >= u16::MAX as usize || mesh.quant_step <= 0. { return; }
    let mut depths = vec![0usize; branches.len()];
    for (i, b) in branches.iter().enumerate() { if let Some(p) = b.parent { depths[i] = depths[p] + 1; } }
    let mut owners: HashMap<[u16; 3], u32> = HashMap::new();
    for vertex in &source.v {
        let p = (Vec3::from_array(vertex.p.to_array()) - mesh.quant_origin) / mesh.quant_step;
        let key = [p.x.round().clamp(0.,65535.) as u16, p.y.round().clamp(0.,65535.) as u16, p.z.round().clamp(0.,65535.) as u16];
        owners.entry(key).and_modify(|owner| {
            let mut a = *owner as usize; let mut b = vertex.branch as usize;
            while depths[a] > depths[b] { a = branches[a].parent.unwrap(); }
            while depths[b] > depths[a] { b = branches[b].parent.unwrap(); }
            while a != b { a = branches[a].parent.unwrap(); b = branches[b].parent.unwrap(); }
            *owner = a as u32;
        }).or_insert(vertex.branch);
    }
    // Coarse DAG levels can move entire leaf pieces away from their original positions.
    // Associate such positions with the nearest source stem within a bounded neighbourhood.
    let cell_size = (mesh.quant_step * 128.).max(0.12);
    let mut cells: HashMap<(i32,i32,i32), Vec<(Vec3,u32)>> = HashMap::new();
    for vertex in &source.v {
        let p = Vec3::from_array(vertex.p.to_array());
        let cell = (p / cell_size).floor().as_ivec3();
        cells.entry((cell.x, cell.y, cell.z)).or_default().push((p, vertex.branch));
    }
    for vertex in &mut mesh.vertices {
        if let Some(owner) = owners.get(&[vertex.data[0], vertex.data[1], vertex.data[2]]) {
            vertex.data[7] = *owner as u16 + 1;
            continue;
        }
        let p = mesh.quant_origin + Vec3::new(vertex.data[0] as f32, vertex.data[1] as f32, vertex.data[2] as f32) * mesh.quant_step;
        let cell = (p / cell_size).floor().as_ivec3();
        let mut best = f32::MAX;
        let mut owner = None;
        for radius in 1i32..=3 {
            for dx in -radius..=radius { for dy in -radius..=radius { for dz in -radius..=radius {
                if let Some(points) = cells.get(&(cell.x+dx, cell.y+dy, cell.z+dz)) {
                    for (candidate, branch) in points {
                        let distance = p.distance_squared(*candidate);
                        if distance < best { best = distance; owner = Some(*branch); }
                    }
                }
            }}}
            if best <= (radius as f32 * cell_size).powi(2) { break; }
        }
        if let Some(owner) = owner {
            vertex.data[7] = owner as u16 + 1;
        }
    }
    // One disconnected leaf or twig component must move rigidly even when a coarse DAG
    // vertex was matched to a different nearby shoot. Use the shared ancestor throughout
    // the connected component; this favours stable attachment over spurious fine motion.
    for cluster in &mesh.clusters {
        let count = cluster.vertex_count() as usize;
        let start = cluster.vertex_offset as usize;
        let mut roots: Vec<usize> = (0..count).collect();
        fn root(roots: &mut [usize], mut n: usize) -> usize {
            while roots[n] != n { roots[n] = roots[roots[n]]; n = roots[n]; } n
        }
        for packed in &mesh.triangles[cluster.triangle_offset as usize..cluster.triangle_offset as usize + cluster.triangle_count() as usize] {
            let tri = [(*packed & 255) as usize, ((*packed >> 8) & 255) as usize, ((*packed >> 16) & 255) as usize];
            let a = root(&mut roots, tri[0]);
            for &v in &tri[1..] { let b = root(&mut roots, v); if a != b { roots[b] = a; } }
        }
        let mut common = vec![None::<usize>; count];
        for v in 0..count {
            let r = root(&mut roots, v);
            let id = mesh.vertices[start+v].data[7];
            let mut b = if id == 0 { 0 } else { id as usize - 1 };
            if let Some(mut a) = common[r] {
                while depths[a] > depths[b] { a = branches[a].parent.unwrap(); }
                while depths[b] > depths[a] { b = branches[b].parent.unwrap(); }
                while a != b { a = branches[a].parent.unwrap(); b = branches[b].parent.unwrap(); }
                common[r] = Some(a);
            } else { common[r] = Some(b); }
        }
        for v in 0..count {
            let r = root(&mut roots, v);
            mesh.vertices[start+v].data[7] = common[r].unwrap_or(0) as u16 + 1;
        }
    }
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
    let branches = game.as_ref().map_or(&tree.branches, |ladder| &ladder.branches);
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
        let mut mesh = build_virtual_mesh(&source, &opts);
        attach_branch_ids(&mut mesh, data, branches);
        crate::log::log(format!(
            "Solstice: {} {} meshed in {:.2} s",
            settings.species,
            if foliage { "leaves" } else { "bark" },
            meshing.elapsed().as_secs_f32()
        ));
        let layer = if foliage && game_cards {
            let atlas = image::RgbaImage::from_raw(crate::gamelod::ATLAS, crate::gamelod::ATLAS, game.as_ref().unwrap().atlas.clone()).unwrap();
            let color = image::imageops::resize(&atlas, TEX_SIZE, TEX_SIZE, FilterType::Triangle).into_raw();
            let (normal, data) = leaf_surface_maps(settings, true, TEX_SIZE as usize);
            layer(color, normal, data)
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
                    Material::TWO_SIDED | Material::FOLIAGE | 128 | ((leaf_transmission * 100.).round() as u32) << 16 | if (game_cards && lod != 0) || (!game_cards && settings.leaf_atlas_path.is_some()) { solstice_render::scene::MATERIAL_ALPHA_TEST } else { 0 } | if !leaf_specular { solstice_render::scene::MATERIAL_NO_SPECULAR } else { 0 }
                } else {
                    ((settings.parallax_depth * 1000.).round() as u32) << 8
                },
            texture_layer: 0,
            normal_strength: if foliage { if settings.leaf_atlas_path.is_some() { 0. } else { 0.5 } } else { 1. },
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
        branches: branches.clone(),
    };
    let mut cache = cache.lock().map_err(|e| e.to_string())?;
    if cache.len() >= SPECIES_CACHE_LIMIT {
        cache.clear();
    }
    cache.insert(key, asset.clone());
    Ok(asset)
}
/// A compact preview composition inspired by Garden's willow/oak pond and birch grove.
/// This deliberately does not import its billion-triangle terrain and planting bake.
fn garden_placements(seed: u32) -> Vec<(usize, Mat4)> {
    let mut placed = Vec::new();
    let mut add = |kind: usize, x: f32, z: f32, scale: f32| {
        placed.push((
            kind,
            Mat4::from_scale_rotation_translation(
                Vec3::splat(scale),
                glam::Quat::from_rotation_y((x * 1.7 + z * 0.3 + seed as f32).sin() * 3.),
                Vec3::new(x, 0., z),
            ),
        ))
    };
    for (x, z) in [(-13., -7.), (15., 5.), (-9., 15.)] {
        add(1, x, z, 1.);
    }
    for (x, z) in [(-26., -22.), (23., -20.), (28., 23.)] {
        add(2, x, z, 1.);
    }
    for i in 0..10 {
        add(
            0,
            -30. + (i % 3) as f32 * 6.,
            8. + (i / 3) as f32 * 7.,
            0.85 + (i % 3) as f32 * 0.08,
        );
    }
    for i in 0..16 {
        add(3, -27. + i as f32 * 3.6, -32., 1.);
    }
    placed
}

fn in_pond(x: f32, z: f32, margin: f32) -> bool {
    ((x + 3.) / (8. + margin)).powi(2) + ((z + 3.) / (5. + margin)).powi(2) < 1.
        || ((x - 3.) / (6. + margin)).powi(2) + ((z - 3.) / (7. + margin)).powi(2) < 1.
}

fn add_scene_grass(
    scene: &mut SceneAsset,
    settings: &crate::grass::Settings,
    stand: bool,
    garden: bool,
) -> Result<(), String> {
    settings.validate()?;
    let mut patch = settings.clone();
    patch.size = 4.;
    // Scene coverage uses a sparse budget, separate from the isolated patch editor.
    // In particular, a full-density 4 m lawn exceeds the authoring allocation limit.
    patch.density = (settings.density * 0.2).clamp(0.05, 0.2);
    // Sparse, wider blades at the scene scale. The dedicated patch editor retains all LODs.
    let mesh = patch.mesh(2)?;
    let index = scene.meshes.len() as u32;
    scene
        .meshes
        .push(build_virtual_mesh(&mesh, &BuildOptions::default()));
    let texture = scene.textures.len() as i32;
    scene.textures.push(patch.atlas()?);
    let material = scene.materials.len() as u32;
    scene.materials.push(Material {
        texture_layer: texture,
        roughness: 0.75,
        flags: Material::TWO_SIDED
            | Material::FOLIAGE
            | solstice_render::scene::MATERIAL_WIND
            | solstice_render::scene::MATERIAL_GRASS
            | 128
            | (65 << 16),
        normal_strength: 0.3,
        ..Default::default()
    });
    let half = if stand { 8 } else { 2 };
    for z in -half..half {
        for x in -half..half {
            let (x, z) = (x as f32 * 4. + 2., z as f32 * 4. + 2.);
            // Leave the pond and its bank bare instead of growing grass through water.
            if garden && in_pond(x, z, 3.) {
                continue;
            }
            let hash = ((x / 4.).floor() as i64 * 73856093)
                ^ ((z / 4.).floor() as i64 * 19349663)
                ^ i64::from(settings.seed);
            let quarter = (hash ^ (hash >> 13)).rem_euclid(4) as f32;
            scene.instances.push(Instance {
                mesh: index,
                material,
                transform: Mat4::from_rotation_translation(
                    glam::Quat::from_rotation_y(quarter * std::f32::consts::FRAC_PI_2),
                    Vec3::new(x, 0., z),
                ),
            });
        }
    }
    Ok(())
}

fn add_garden_site(scene: &mut SceneAsset) {
    let mut pond = Mesh::default();
    // A union of small surface tiles gives the two-lobed pond one surface without overlap.
    for z in -12..12 {
        for x in -14..14 {
            let (x, z) = (x as f32, z as f32);
            if !in_pond(x + 0.5, z + 0.5, 0.) {
                continue;
            }
            let first = pond.vertices.len() as u32;
            for (dx, dz) in [(0., 0.), (0., 1.), (1., 1.), (1., 0.)] {
                pond.vertices.push(Vertex {
                    position: Vec3::new(x + dx, 0.025, z + dz),
                    normal: Vec3::Y,
                    uv: Vec2::new(x + dx, z + dz),
                });
            }
            pond.indices
                .extend([first, first + 1, first + 2, first, first + 2, first + 3]);
        }
    }
    let index = scene.meshes.len() as u32;
    scene
        .meshes
        .push(build_virtual_mesh(&pond, &BuildOptions::default()));
    let material = scene.materials.len() as u32;
    scene.materials.push(Material {
        base_color: [0.055, 0.15, 0.17, 1.],
        roughness: 0.18,
        normal_strength: 0.,
        ..Default::default()
    });
    scene.instances.push(Instance {
        mesh: index,
        material,
        transform: Mat4::IDENTITY,
    });
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
    let grid: Vec<u8> = if key.garden || (key.scene_grass.is_some() && (key.forest==0 || !key.forest_floor)) {
        (0..TEX_SIZE*TEX_SIZE).flat_map(|i| {let v=((i*17+i/256*7)%19) as u8; [62+v,81+v,39+v/2,255]}).collect()
    } else if key.forest != 0 && key.forest_floor {
        forest_floor_texture(key.settings.seed, key.settings.season)
    } else { (0..TEX_SIZE)
        .flat_map(|y| {
            (0..TEX_SIZE).flat_map(move |x| {
                if key.grid && (x < 3 || y < 3) {
                    [110, 110, 110, 255]
                } else {
                    [159, 159, 159, 255]
                }
            })
        })
        .collect() };
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

    if let Some(grass) = &key.grass {
        if !key.leaves { return Ok(paginate(&scene)); }
        let mesh = grass.mesh(key.lod.unwrap_or(0).min(2))?;
        let index = scene.meshes.len() as u32;
        scene.meshes.push(build_virtual_mesh(&mesh, &opts));
        let texture_layer = scene.textures.len() as i32;
        scene.textures.push(grass.atlas()?);
        scene.materials.push(Material { base_color:[1.;4], texture_layer, roughness:0.7,
            flags:Material::TWO_SIDED | Material::FOLIAGE | solstice_render::scene::MATERIAL_WIND | solstice_render::scene::MATERIAL_GRASS | 128 | (65 << 16),
            normal_strength:0.3, ..Default::default() });
        scene.instances.push(Instance { mesh:index, material:index, transform:Mat4::IDENTITY });
        return Ok(paginate(&scene));
    }
    let mut trees = Vec::new();
    let mut understory = Vec::new();
    if key.garden {
        for id in ["solstice-birch","solstice-willow","solstice-old_oak","solstice-bush"] { let mut t=Settings::preset(id)?; t.season=key.settings.season; trees.push(t); }
    } else if key.forest == 0 {
        trees.push(key.settings.clone());
    } else {
        for (i, species) in SPECIES.iter().enumerate() {
            if key.forest & (1 << i) != 0 {
                let mut tree = if key.settings.species == *species {
                    key.settings.clone()
                } else {
                    Settings::preset(species)?
                };
                tree.season = key.settings.season;
                trees.push(tree);
            }
        }
    }
    let placed = if key.garden { garden_placements(key.settings.seed) } else if key.forest == 0 { Vec::new() } else {
        let weights: Vec<u32> = SPECIES.iter().enumerate()
            .filter(|(index, _)| key.forest & (1 << index) != 0)
            .map(|(index, _)| key.ratios[index]).collect();
        let spacing = forest_spacing(&key.settings, key.forest);
        if key.random_mix { natural_forest(key.count, &weights, spacing, key.settings.seed) }
        else { place_forest(key.count, &weights, spacing) }
    };
    let young_kinds: std::collections::HashSet<usize> = placed.iter()
        .filter(|(_, transform)| transform.x_axis.truncate().length() < 0.7)
        .map(|(kind, _)| *kind).collect();
    let mut assets = Vec::new();
    let mut young_assets = vec![None; trees.len()];
    let mut hierarchies = Vec::new();
    BUILD_TOTAL.store((trees.len() + young_kinds.len()) as u32, Ordering::Relaxed);
    BUILD_DONE.store(0, Ordering::Relaxed);
    for (kind, settings) in trees.iter().enumerate() {
        for is_young in [false, true] {
            if is_young && !young_kinds.contains(&kind) { continue; }
            let mut stage = settings.clone();
            if is_young { stage.maturity = (settings.maturity * 0.55).clamp(if settings.species == "solstice-bush" { 0.4 } else { 0.15 }, 1.2); }
            let asset = species_asset(&stage, key.lod.unwrap_or(0), key.game_cards, key.trim_cards, key.leaf_specular, key.leaf_roughness, key.leaf_transmission)?;
            let first = scene.meshes.len() as u32;
            for (mesh, layer, material) in [&asset.bark, &asset.leaves] {
                scene.meshes.push(mesh.clone());
                let texture_layer = scene.textures.len() as i32;
                scene.textures.push(layer.clone());
                scene.materials.push(Material { texture_layer, ..*material });
            }
            if is_young { young_assets[kind] = Some(first); } else { assets.push(first); }
            hierarchies.push((first, asset.branches));
            BUILD_DONE.fetch_add(1, Ordering::Relaxed);
        }
    }
    // Hierarchy records share the already-bound material buffer; only actual draw materials
    // point at these records, so no extra per-stage storage binding is required.
    for (first, branches) in hierarchies {
        if branches.is_empty() || branches.len() >= u16::MAX as usize { continue; }
        let base = scene.materials.len();
        for branch in &branches {
            let start = branch.points[0];
            let end = *branch.points.last().unwrap();
            scene.materials.push(Material { base_color: [start[0],start[1],start[2], branch.parent.map_or(0., |p| (p+1) as f32)],
                emissive: end, roughness: branch.radii.first().copied().unwrap_or(0.01), ..Default::default() });
        }
        for index in [first as usize, first as usize + 1] {
            scene.materials[index].flags |= 1 << 24;
            scene.materials[index].emissive[0] = base as f32;
        }
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
    if key.forest == 0 && !key.garden {
        place(assets[0], Mat4::IDENTITY);
    } else {
        let spacing = forest_spacing(&key.settings, key.forest);
        for (kind, mut transform) in placed {
            let scale = transform.x_axis.truncate().length();
            let first = if scale < 0.7 {
                // The prototype is genuinely grown young; retain only mild instance variation.
                let (_, rotation, translation) = transform.to_scale_rotation_translation();
                transform = Mat4::from_scale_rotation_translation(Vec3::splat((scale / 0.55).clamp(0.9, 1.1)), rotation, translation);
                young_assets[kind].unwrap_or(assets[kind])
            } else { assets[kind] };
            place(first, transform);
            if key.forest_floor {
                let position = transform.w_axis.truncate();
                for k in 0..2 {
                    let phase = (position.x * 13.31 + position.z * 19.17 + k as f32 * 2.4).sin();
                    let offset = Vec3::new(phase * spacing * 0.22, 0., (phase * 8.3).cos() * spacing * 0.22);
                    understory.push(Mat4::from_scale_rotation_translation(Vec3::splat(0.75 + phase.abs() * 0.7), glam::Quat::from_rotation_y(phase * 3.), position + offset));
                }
            }
        }
    }
    if !understory.is_empty() {
        let mesh = build_virtual_mesh(&understory_mesh(), &opts);
        let index = scene.meshes.len() as u32;
        scene.meshes.push(mesh);
        let color: Vec<u8> = (0..TEX_SIZE*TEX_SIZE).flat_map(|_| [54, 101, 39, 255]).collect();
        scene.textures.push(layer(color, vec![128; n], vec![128; n]));
        let material_index = scene.materials.len() as u32;
        scene.materials.push(Material { base_color: [1.;4], roughness: 0.85, texture_layer: index as i32,
            flags: Material::TWO_SIDED | Material::FOLIAGE, normal_strength: 0., ..Default::default() });
        for transform in understory { scene.instances.push(Instance { mesh: index, material: material_index, transform }); }
    }
    if let Some(grass)=&key.scene_grass {
        add_scene_grass(&mut scene,grass,key.garden || key.forest!=0,key.garden)?;
    }
    if key.garden { add_garden_site(&mut scene); }
    Ok(paginate(&scene))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaf_seasons_keep_species_detail_and_game_sprays_keep_neutral_normals() {
        let mut s = Settings::preset("solstice-birch").unwrap();
        let summer = leaf_rgba(&s); s.season = 1;
        let autumn = leaf_rgba(&s);
        assert_ne!(summer, autumn);
        assert!(autumn.chunks_exact(4).map(|p| p[1]).max().unwrap() > autumn.chunks_exact(4).map(|p| p[1]).min().unwrap() + 15);
        let (normal, data) = leaf_surface_maps(&s, true, 256);
        assert_eq!(normal.len(), 256*256*2); assert_eq!(data.len(), normal.len());
        assert!(normal[..100*256*2].iter().any(|&v| v != 128));
        assert!(normal[140*256*2..].iter().all(|&v| v == 128));
    }

    #[test]
    fn foliage_phase_is_shared_within_each_piece_and_keeps_atlas_coordinates() {
        let v = |x: f32, u: f32, v: f32| crate::tree::V { p: macroquad::prelude::vec3(x, v, 0.), n: macroquad::prelude::Vec3::Y, uv: macroquad::prelude::vec2(u,v), c: [1.;4], branch: 0 };
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
        let asset = species_asset(&s, 2, false, false, true, 0.55, 0.8).unwrap();
        for mesh in [&asset.bark.0, &asset.leaves.0] {
            let assigned = mesh.vertices.iter().filter(|v| v.data[7] > 0).count();
            assert!(assigned > mesh.vertices.len() * 95 / 100, "only {assigned}/{} vertices retained branch ownership", mesh.vertices.len());
            assert!(mesh.vertices.iter().any(|v| v.data[7] > 1));
        }
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
    fn natural_stand_is_seeded_spaced_and_keeps_species_counts() {
        let a = natural_forest(80, &[3,1], 6., 123);
        assert_eq!(a, natural_forest(80, &[3,1], 6., 123));
        assert_ne!(a, natural_forest(80, &[3,1], 6., 124));
        assert_eq!(a.iter().filter(|(kind,_)| *kind == 0).count(), 60);
        for (i, (_, transform)) in a.iter().enumerate() {
            for (_, other) in &a[..i] {
                assert!(transform.w_axis.truncate().distance(other.w_axis.truncate()) >= 6. * 0.13);
            }
        }
        assert!(a.iter().any(|(_, transform)| transform.x_axis.truncate().length() < 0.7));
    }

    #[test]
    fn forest_uses_grown_saplings_instead_of_shrunk_adult_meshes() {
        let mut settings = Settings::preset("solstice-bush").unwrap();
        settings.seed = 123;
        let mut ratios = [0; SPECIES.len()];
        let index = SPECIES.iter().position(|s| *s == "solstice-bush").unwrap();
        ratios[index] = 1;
        let key = SceneKey { settings, grass:None, scene_grass:None, garden:false, lod: Some(2), forest: 1u128 << index, count: 40, ratios,
            grid: false, bark: true, leaves: true, game_cards: false, trim_cards: false,
            random_mix: true, forest_floor: false, leaf_specular: true, leaf_roughness: 0.55, leaf_transmission: 0.8 };
        let scene = build_scene(&key).unwrap().scene;
        assert_eq!(scene.meshes.len(), 5, "ground plus adult and young bark/foliage");
        let young: Vec<_> = scene.instances.iter().filter(|i| i.mesh == 3).collect();
        assert!(!young.is_empty(), "the young prototype is instantiated");
        assert!(young.iter().all(|i| (0.89..=1.11).contains(&i.transform.x_axis.truncate().length())), "young leaves must not be scaled down with an adult mesh");
    }

    #[test]
    fn grass_composes_with_forest_and_keeps_branch_material_records_separate() {
        let settings=Settings::preset("solstice-bush").unwrap();
        let index=SPECIES.iter().position(|s|*s=="solstice-bush").unwrap();
        let mut ratios=[0;SPECIES.len()]; ratios[index]=1;
        let key=SceneKey { settings,grass:None,scene_grass:Some(crate::grass::Settings::default()),garden:false,
            lod:Some(2),forest:1u128<<index,count:4,ratios,grid:false,bark:true,leaves:true,
            game_cards:false,trim_cards:true,random_mix:false,forest_floor:false,
            leaf_specular:true,leaf_roughness:0.55,leaf_transmission:0.8 };
        let scene=build_scene(&key).unwrap().scene;
        let grass:Vec<_>=scene.instances.iter().filter(|i| scene.materials[i.material as usize].flags & solstice_render::scene::MATERIAL_GRASS != 0).collect();
        assert_eq!(grass.len(),256);
        assert_eq!(scene.instances.len(),1+4*2+256,"ground, bark, leaves and grass coexist");
        assert!(grass.iter().all(|i|i.material as usize>i.mesh as usize),"grass must not index a branch hierarchy as its material");
        for instance in &scene.instances {
            assert!((instance.mesh as usize)<scene.meshes.len());
            let texture=scene.materials[instance.material as usize].texture_layer;
            assert!(texture>=0 && (texture as usize)<scene.base_textures.len());
        }
        let isolated=build_scene(&SceneKey{forest:0,scene_grass:None,..key}).unwrap().scene;
        assert_eq!(isolated.instances.len(),3,"switching off the stand/grass retains a single tree");
    }

    #[test]
    fn garden_grass_leaves_water_clear_and_specimens_are_repeatable() {
        let placed=garden_placements(42);
        assert_eq!(placed,garden_placements(42));
        assert_eq!(placed.len(),32);
        assert!((0..4).all(|kind| placed.iter().any(|(k,_)|*k==kind)));
        let mut scene=SceneAsset::default();
        // Simulate hierarchy records preceding the grass draw material.
        scene.materials.resize(10,Material::default());
        add_scene_grass(&mut scene,&crate::grass::Settings::preset("lawn").unwrap(),true,true).unwrap();
        assert!(!scene.instances.is_empty() && scene.instances.len()<256);
        assert!(scene.instances.iter().all(|i| {let p=i.transform.w_axis; !in_pond(p.x,p.z,3.)}));
        assert!(scene.instances.iter().all(|i|i.material==10));
        add_garden_site(&mut scene);
        assert_eq!(scene.meshes.len(),2);
        assert_eq!(scene.instances.last().unwrap().material,11);
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
