//! Garden scene, reimplemented around the site measurements of SamG-Coder/solstice-garden
//! (MIT, apps/bake/src/garden/site.rs). Geometry construction is Grove's own implementation.
//! The source plan includes the pond, three paths, seven planting beds and twelve specimens.
use glam::{Mat4, Quat, Vec2, Vec3};
use solstice_asset::{Instance, Material, SceneAsset, Sea, TextureLayer};
use solstice_builder::{build_virtual_mesh, mip_chain, BuildOptions, Mesh, Vertex};
use solstice_procgen::{noise::fbm, surfaces};
use std::sync::OnceLock;

const BRIDGE_X: f32 = 2.2;
const BEDS: [([f32; 2], [f32; 2], f32, bool); 7] = [
    ([7.6, 33.], [7.4, 21.], 2.3, false),
    ([-3.6, 34.5], [-2.6, 24.5], 2.1, false),
    ([-8., 19.], [-13.5, 19.6], 1.7, false),
    ([6.4, 12.4], [10.4, 11.4], 1.5, false),
    ([10.5, -17.5], [13.5, -14.5], 2.4, true),
    ([2.6, -17.6], [-1.5, -17.], 2., true),
    ([-25.5, -9.], [-25., -15.], 2.2, true),
];
// Kind: birch=0, willow=1, old oak=2. These are the upstream specimen positions and scales.
const SPECIMENS: [(usize, f32, f32, f32, f32); 12] = [
    (1, -16.9, -4., 1., 0.4),
    (1, 17.2, -3.4, 0.9, 2.3),
    (1, -9.9, 13., 0.74, 4.1),
    (2, 7.5, -20.5, 1.22, 1.1),
    (2, -27., -12., 0.9, 3.),
    (2, 27.5, 9., 0.86, 5.2),
    (2, -26., 20.5, 0.8, 0.3),
    (2, 16.5, 30., 0.72, 2.2),
    (0, 24.5, -12.5, 1., 0.),
    (0, 27., -15.6, 0.9, 1.7),
    (0, 22.6, -17.2, 0.82, 3.9),
    (0, 29.4, -10.4, 0.76, 5.),
];
fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0., 1.);
    t * t * (3. - 2. * t)
}
fn segment(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    p.distance(
        a + (b - a) * ((p - a).dot(b - a) / (b - a).length_squared().max(1e-6)).clamp(0., 1.),
    )
}
fn rounded_union(a: f32, b: f32, r: f32) -> f32 {
    a.min(b) - (r - (a - b).abs()).max(0.).powi(2) / (4. * r)
}
pub fn pond_distance(x: f32, z: f32) -> f32 {
    let p = Vec2::new(x, z);
    let mut d = p.distance(Vec2::new(-6.6, 0.4)) - 8.3;
    for (cx, cz, r, k) in [
        (9.2, 0.2, 6.1, 2.5),
        (-11.6, 7.2, 4.3, 2.5),
        (13.4, -5.2, 3.1, 2.),
    ] {
        d = rounded_union(d, p.distance(Vec2::new(cx, cz)) - r, k);
    }
    d = rounded_union(
        d,
        segment(p, Vec2::new(-1.5, 0.3), Vec2::new(5.5, 0.3)) - 2.1,
        1.5,
    );
    d + fbm(Vec3::new(x * 0.16, 0., z * 0.16), 3, 801) * 1.1
        + fbm(Vec3::new(x * 0.7, 2., z * 0.7), 3, 803) * 0.28
}
pub fn ground_height(x: f32, z: f32) -> f32 {
    let d = pond_distance(x, z);
    let rough = fbm(Vec3::new(x * 0.9, 0., z * 0.9), 3, 837) * 0.018;
    if d < 0. {
        return -0.28 * smooth(0., 0.55, -d) - 1.17 * smooth(0.25, 5.9, -d)
            + fbm(Vec3::new(x * 0.5, 7., z * 0.5), 3, 839) * 0.06 * smooth(0.3, 2., -d)
            + rough * smooth(0., 0.5, -d);
    }
    let p = Vec3::new(x, 0., z);
    let roll = (fbm(p * 0.022, 4, 831) * 1.1 + fbm(p * 0.075, 3, 833) * 0.22).max(-0.25);
    let hills = smooth(46., 150., Vec2::new(x, z).length())
        * (8.5 + 6. * fbm(p * 0.011 + Vec3::Y * 4., 4, 835));
    0.34 * smooth(0., 1.5, d)
        + 0.06 * smooth(0., 0.25, d)
        + (roll + hills + smooth(10., 70., z) * 0.9) * smooth(1., 16., d)
        + rough
}
pub fn ground_normal(x: f32, z: f32) -> Vec3 {
    Vec3::new(
        ground_height(x - 0.05, z) - ground_height(x + 0.05, z),
        0.1,
        ground_height(x, z - 0.05) - ground_height(x, z + 0.05),
    )
    .normalize()
}
fn bridge_ends() -> (f32, f32) {
    let mut north = -8.;
    let mut south = 8.;
    while pond_distance(BRIDGE_X, north) > 0. && north < 0. {
        north += 0.025;
    }
    while pond_distance(BRIDGE_X, south) > 0. && south > 0. {
        south -= 0.025;
    }
    (north - 0.55, south + 0.55)
}
fn deck(z: f32) -> f32 {
    let (a, b) = bridge_ends();
    let t = ((z - a) / (b - a)).clamp(0., 1.);
    (ground_height(BRIDGE_X, a).max(ground_height(BRIDGE_X, b)) + 0.12)
        + 0.32 * (std::f32::consts::PI * t).sin()
}
pub fn walk_height(x: f32, z: f32) -> f32 {
    let (a, b) = bridge_ends();
    if (x - BRIDGE_X).abs() < 0.95 && (a..=b).contains(&z) {
        deck(z)
    } else {
        ground_height(x, z)
    }
}
pub fn entrance() -> Vec3 {
    Vec3::new(1.6, ground_height(1.6, 23.) + 1.7, 23.)
}
pub fn view(index: usize) -> (Vec3, f32, f32) {
    let (x, z, target) = match index {
        0 => (-17., 15., Vec3::new(2.2, 0.8, 0.3)),
        1 => (11., -8., Vec3::new(-7., 0.7, 1.)),
        _ => {
            return (entrance(), 0.02, -0.1);
        }
    };
    let eye = Vec3::new(x, ground_height(x, z) + 1.7, z);
    let back = eye - target;
    (eye, back.x.atan2(back.z), -(back.y / back.length()).asin())
}
pub fn walk_allowed(x: f32, z: f32) -> bool {
    let (north, south) = bridge_ends();
    let bridge = (x - BRIDGE_X).abs() < 0.9 && (north..=south).contains(&z);
    if !bridge && pond_distance(x, z) < 0.1 {
        return false;
    }
    !SPECIMENS.iter().any(|(kind, tx, tz, scale, _)| {
        Vec2::new(x - tx, z - tz).length() < scale * if *kind == 2 { 0.9 } else { 0.65 }
    })
}
pub fn walk_spawn(x: f32, z: f32) -> Vec3 {
    if walk_allowed(x, z) {
        return Vec3::new(x, walk_height(x, z) + 1.7, z);
    }
    for ring in 1..100 {
        for k in 0..48 {
            let a = k as f32 * std::f32::consts::TAU / 48.;
            let (x, z) = (
                x + a.cos() * ring as f32 * 0.25,
                z + a.sin() * ring as f32 * 0.25,
            );
            if walk_allowed(x, z) {
                return Vec3::new(x, walk_height(x, z) + 1.7, z);
            }
        }
    }
    entrance()
}

fn paths() -> &'static Vec<Vec<Vec2>> {
    static PATHS: OnceLock<Vec<Vec<Vec2>>> = OnceLock::new();
    PATHS.get_or_init(|| {
        let (north, south) = bridge_ends();
        let plans = vec![
            vec![
                (6., 70.),
                (3.5, 52.),
                (0.4, 38.),
                (3.4, 25.),
                (1.2, 14.5),
                (BRIDGE_X, south + 3.),
                (BRIDGE_X, south),
            ],
            vec![
                (BRIDGE_X, north),
                (BRIDGE_X, north - 2.5),
                (3.6, -10.5),
                (7.2, -14.6),
                (13.5, -19.),
                (23., -24.5),
                (36., -33.),
                (52., -46.),
                (70., -62.),
            ],
            vec![
                (1.5, 15.2),
                (-5., 14.6),
                (-12., 15.6),
                (-18.6, 12.2),
                (-21.4, 4.5),
                (-21., -4.5),
                (-17.5, -11.),
                (-10.5, -14.2),
                (-3., -12.6),
                (3.2, -10.2),
            ],
        ];
        plans
            .into_iter()
            .map(|points| {
                let p: Vec<_> = points.into_iter().map(|(x, z)| Vec2::new(x, z)).collect();
                let mut out = Vec::new();
                for i in 0..p.len() - 1 {
                    let [a, b, c, d] = [
                        p[i.saturating_sub(1)],
                        p[i],
                        p[i + 1],
                        p[(i + 2).min(p.len() - 1)],
                    ];
                    let steps = (b.distance(c) / 0.4).ceil().max(2.) as usize;
                    for k in 0..steps {
                        let t = k as f32 / steps as f32;
                        out.push(
                            (b * 2.
                                + (c - a) * t
                                + (a * 2. - b * 5. + c * 4. - d) * t * t
                                + (-a + b * 3. - c * 3. + d) * t * t * t)
                                * 0.5,
                        );
                    }
                }
                out.push(*p.last().unwrap());
                out
            })
            .collect()
    })
}
pub fn path_distance(x: f32, z: f32) -> f32 {
    let p = Vec2::new(x, z);
    paths()
        .iter()
        .flat_map(|p| p.windows(2))
        .map(|s| segment(p, s[0], s[1]))
        .fold(f32::MAX, f32::min)
}
fn bed_at(x: f32, z: f32) -> Option<bool> {
    let p = Vec2::new(x, z);
    BEDS.iter()
        .find(|(a, b, r, _)| segment(p, Vec2::from(*a), Vec2::from(*b)) < *r)
        .map(|b| b.3)
}
/// Classify grass by the original garden's land use: lawn, meadow, rough margin.
pub fn grass_kind(x: f32, z: f32) -> Option<usize> {
    let pond = pond_distance(x, z);
    if pond < 0.5 || path_distance(x, z) < 1.3 || bed_at(x, z).is_some() {
        return None;
    }
    Some(if pond < 2.4 {
        2
    } else if Vec2::new(x, z).length() < 34. {
        0
    } else {
        1
    })
}
pub fn placements(seed: u32, count: u32) -> Vec<(usize, Mat4)> {
    use std::collections::HashMap;
    let mut out = Vec::new();
    for (kind, x, z, scale, yaw) in SPECIMENS {
        out.push((
            kind,
            Mat4::from_scale_rotation_translation(
                Vec3::splat(scale),
                Quat::from_rotation_y(yaw),
                Vec3::new(x, ground_height(x, z), z),
            ),
        ));
    }
    let mut state = seed.wrapping_add(8911);
    let mut random = || {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        state as f32 / u32::MAX as f32
    };
    let mut cells: HashMap<(i32, i32), Vec<Vec2>> = HashMap::new();
    let target = count.clamp(32, 2000) as usize;
    for _ in 0..100000 {
        if out.len() >= target + 12 {
            break;
        }
        let p = Vec2::new((random() * 2. - 1.) * 166., (random() * 2. - 1.) * 166.);
        let radius = p.length();
        let rim = 43. + 9. * (p.y / radius.max(1.)).abs();
        let wander = fbm(Vec3::new(p.x * 0.02, 13., p.y * 0.02), 3, 851) * 9.;
        let depth = smooth(rim, rim + 16., radius + wander);
        if depth <= 0. || random() > depth || path_distance(p.x, p.y) < 2.95 {
            continue;
        }
        let key = ((p.x / 6.).floor() as i32, (p.y / 6.).floor() as i32);
        let spacing = 10.5 - 4.5 * depth;
        let crowded = (-2..=2).any(|z| {
            (-2..=2).any(|x| {
                cells.get(&(key.0 + x, key.1 + z)).is_some_and(|points| {
                    points
                        .iter()
                        .any(|q| q.distance_squared(p) < spacing * spacing)
                })
            })
        });
        if crowded {
            continue;
        }
        cells.entry(key).or_default().push(p);
        let stand = fbm(Vec3::new(p.x * 0.03, 7., p.y * 0.03), 3, 853);
        let kind = if depth < 0.8 && stand > 0.02 {
            2
        } else if depth < 0.9 && stand < -0.1 {
            0
        } else if depth > 0.6 && stand > 0.05 {
            3
        } else {
            4
        };
        let scale = 0.8 + random() * 0.35;
        out.push((
            kind,
            Mat4::from_scale_rotation_translation(
                Vec3::splat(scale),
                Quat::from_rotation_y(random() * 6.28),
                Vec3::new(p.x, ground_height(p.x, p.y), p.y),
            ),
        ));
    }
    out
}

pub fn terrain() -> Mesh {
    let mut mesh = Mesh::default();
    for z in -45i32..45 {
        for x in -45i32..45 {
            let steps = if x.abs() < 15 && z.abs() < 15 { 8 } else { 1 };
            let size = 4. / steps as f32;
            for dz in 0..steps {
                for dx in 0..steps {
                    let (x, z) = (
                        x as f32 * 4. + dx as f32 * size,
                        z as f32 * 4. + dz as f32 * size,
                    );
                    let base = mesh.vertices.len() as u32;
                    for (dx, dz) in [(0., 0.), (0., size), (size, size), (size, 0.)] {
                        let (x, z) = (x + dx, z + dz);
                        mesh.vertices.push(Vertex {
                            position: Vec3::new(x, ground_height(x, z), z),
                            normal: ground_normal(x, z),
                            uv: Vec2::new(x, z),
                        });
                    }
                    mesh.indices
                        .extend([base, base + 1, base + 2, base, base + 2, base + 3]);
                }
            }
        }
    }
    mesh
}
pub fn turf() -> TextureLayer {
    static TURF: OnceLock<TextureLayer> = OnceLock::new();
    TURF.get_or_init(|| surfaces::grass(0).bake(256)).clone()
}
pub fn virtual_terrain() -> solstice_asset::VirtualMesh {
    static TERRAIN: OnceLock<solstice_asset::VirtualMesh> = OnceLock::new();
    TERRAIN
        .get_or_init(|| build_virtual_mesh(&terrain(), &BuildOptions::default()))
        .clone()
}
fn quad(mesh: &mut Mesh, p: [Vec3; 4], n: Vec3) {
    let first = mesh.vertices.len() as u32;
    for (position, uv) in p.into_iter().zip([Vec2::ZERO, Vec2::Y, Vec2::ONE, Vec2::X]) {
        mesh.vertices.push(Vertex {
            position,
            normal: n,
            uv,
        });
    }
    mesh.indices
        .extend([first, first + 1, first + 2, first, first + 2, first + 3]);
}
fn box_mesh(mesh: &mut Mesh, center: Vec3, size: Vec3) {
    let r = size * 0.5;
    for (n, u, v) in [
        (Vec3::Y, Vec3::X * r.x, -Vec3::Z * r.z),
        (-Vec3::Y, Vec3::X * r.x, Vec3::Z * r.z),
        (Vec3::X, Vec3::Z * r.z, -Vec3::Y * r.y),
        (-Vec3::X, Vec3::Z * r.z, Vec3::Y * r.y),
        (Vec3::Z, Vec3::X * r.x, Vec3::Y * r.y),
        (-Vec3::Z, Vec3::X * r.x, -Vec3::Y * r.y),
    ] {
        let c = center + n * r;
        quad(mesh, [c - u - v, c + u - v, c + u + v, c - u + v], n);
    }
}
fn beam(mesh: &mut Mesh, a: Vec3, b: Vec3, width: f32) {
    let first = mesh.vertices.len();
    box_mesh(mesh, Vec3::ZERO, Vec3::new(width, a.distance(b), width));
    let rotation = Quat::from_rotation_arc(Vec3::Y, (b - a).normalize());
    for v in &mut mesh.vertices[first..] {
        v.position = rotation * v.position + (a + b) * 0.5;
        v.normal = rotation * v.normal;
    }
}
fn leaf(mesh: &mut Mesh, root: Vec3, direction: Vec3, length: f32, width: f32) {
    let side = direction.cross(Vec3::Y).normalize_or(Vec3::X) * width * 0.5;
    let middle = root + direction * length * 0.5 + Vec3::Y * length * 0.12;
    // Triangulate across the curved middle, preserving the actual leaf width.
    let base=mesh.vertices.len() as u32;
    let n=side.cross(direction).normalize_or(Vec3::Y);
    for (position,uv) in [root,middle-side,root+direction*length,middle+side].into_iter().zip([Vec2::new(0.5,0.),Vec2::new(0.,0.5),Vec2::new(0.5,1.),Vec2::new(1.,0.5)]) {
        mesh.vertices.push(Vertex{position,normal:n,uv});
    }
    mesh.indices.extend([base,base+3,base+1,base+1,base+3,base+2]);
}
fn append(
    scene: &mut SceneAsset,
    mesh: Mesh,
    color: [f32; 4],
    texture: Option<TextureLayer>,
    foliage: bool,
) {
    if mesh.indices.is_empty() {
        return;
    }
    let index = scene.meshes.len() as u32;
    scene
        .meshes
        .push(build_virtual_mesh(&mesh, &BuildOptions::default()));
    let material = scene.materials.len() as u32;
    let texture_layer = if let Some(texture) = texture {
        let i = scene.textures.len() as i32;
        scene.textures.push(texture);
        i
    } else {
        -1
    };
    scene.materials.push(Material {
        base_color: color,
        texture_layer,
        roughness: 0.83,
        normal_strength: if texture_layer >= 0 { 0.7 } else { 0. },
        flags: Material::TWO_SIDED | if foliage { Material::FOLIAGE } else { 0 },
        ..Default::default()
    });
    scene.instances.push(Instance {
        mesh: index,
        material,
        transform: Mat4::IDENTITY,
    });
}
fn weathered_wood() -> TextureLayer {
    let mut color = Vec::new();
    let mut normals = Vec::new();
    let mut material = Vec::new();
    for y in 0..256 {
        for x in 0..256 {
            let a = x as f32 * 0.8 + (y as f32 * 0.06).sin() * 0.4;
            let grain = a.sin() * 0.6 + (a * 3.7).sin() * 0.25;
            let value = (grain * 17.) as i16;
            color.extend([
                ((106 + value) as u8),
                ((99 + value) as u8),
                ((86 + value) as u8),
                255,
            ]);
            normals.extend([(128. + a.cos() * 8.) as u8, 128]);
            material.extend([210, 255]);
        }
    }
    TextureLayer {
        color_mips: mip_chain(color, 256, 4),
        normal_mips: mip_chain(normals, 256, 2),
        material_mips: mip_chain(material, 256, 2),
    }
}
fn boulder(mesh: &mut Mesh, center: Vec3, radius: f32) {
    let start = mesh.vertices.len() as u32;
    for y in 0..=8 {
        for x in 0..=12 {
            let a = x as f32 / 12. * std::f32::consts::TAU;
            let b = y as f32 / 8. * std::f32::consts::PI;
            let n = Vec3::new(a.cos() * b.sin(), b.cos(), a.sin() * b.sin());
            let r = radius * (1. + 0.12 * (a * 3. + b * 2.).sin());
            mesh.vertices.push(Vertex {
                position: center + n * Vec3::new(1.3, 0.7, 1.) * r,
                normal: n,
                uv: Vec2::new(x as f32 / 12., y as f32 / 8.),
            });
        }
    }
    for y in 0..8 {
        for x in 0..12 {
            let a = start + y * 13 + x;
            mesh.indices
                .extend([a, a + 13, a + 1, a + 1, a + 13, a + 14]);
        }
    }
}

fn scenery() -> SceneAsset {
    let mut scene = SceneAsset {
        texture_size: 256,
        ..Default::default()
    };
    let mut paths_mesh = Mesh::default();
    for path in paths() {
        for pair in path.windows(2) {
            let side = Vec2::new(pair[1].y - pair[0].y, pair[0].x - pair[1].x).normalize() * 0.75;
            let p = [
                pair[0] - side,
                pair[1] - side,
                pair[1] + side,
                pair[0] + side,
            ]
            .map(|p| Vec3::new(p.x, ground_height(p.x, p.y) + 0.025, p.y));
            quad(&mut paths_mesh, p, Vec3::Y);
        }
    }
    // Gravel's texture coordinates are in metres, rather than restarting at every sample.
    for v in &mut paths_mesh.vertices {
        v.uv = Vec2::new(v.position.x, v.position.z) * 3.;
    }
    append(
        &mut scene,
        paths_mesh,
        [0.72, 0.68, 0.6, 1.],
        Some(surfaces::gravel(0).bake(256)),
        false,
    );
    let mut soil = Mesh::default();
    for z in -80..80 {
        for x in -80..80 {
            let (x, z) = (x as f32 * 0.5, z as f32 * 0.5);
            if bed_at(x, z).is_none() && pond_distance(x + 0.25, z + 0.25) > 0.8 {
                continue;
            }
            let p = [(0., 0.), (0., 0.5), (0.5, 0.5), (0.5, 0.)]
                .map(|(a, b)| Vec3::new(x + a, ground_height(x + a, z + b) + 0.018, z + b));
            quad(&mut soil, p, Vec3::Y);
        }
    }
    append(
        &mut scene,
        soil,
        [0.5, 0.45, 0.39, 1.],
        Some(surfaces::dirt(0).bake(256)),
        false,
    );
    let mut timber = Mesh::default();
    let (north, south) = bridge_ends();
    let count = ((south - north) / 0.17).ceil() as usize;
    for k in 0..=count {
        let z = north + (south - north) * k as f32 / count as f32;
        box_mesh(
            &mut timber,
            Vec3::new(BRIDGE_X, deck(z) - 0.04, z),
            Vec3::new(1.85, 0.08, 0.15),
        );
    }
    for side in [-1., 1.] {
        let x = BRIDGE_X + side * 0.88;
        for i in 0..5 {
            let z = north + (south - north) * i as f32 / 4.;
            box_mesh(
                &mut timber,
                Vec3::new(x, deck(z) + 0.43, z),
                Vec3::new(0.09, 1.05, 0.09),
            );
        }
        for k in 0..count {
            let a = north + (south - north) * k as f32 / count as f32;
            let b = north + (south - north) * (k + 1) as f32 / count as f32;
            beam(
                &mut timber,
                Vec3::new(x, deck(a) + 0.92, a),
                Vec3::new(x, deck(b) + 0.92, b),
                0.08,
            );
        }
    }
    // A board-built bench next to the great oak's path.
    let bench = Vec3::new(10.8, ground_height(10.8, -20.), -20.);
    for x in [-0.72, 0.72] {
        for z in [-0.23, 0.23] {
            box_mesh(
                &mut timber,
                bench + Vec3::new(x, 0.23, z),
                Vec3::new(0.09, 0.46, 0.09),
            );
        }
    }
    for z in [-0.2, 0., 0.2] {
        box_mesh(
            &mut timber,
            bench + Vec3::new(0., 0.48, z),
            Vec3::new(1.9, 0.065, 0.17),
        );
    }
    for y in [0.7, 0.9] {
        box_mesh(
            &mut timber,
            bench + Vec3::new(0., y, -0.28),
            Vec3::new(1.9, 0.15, 0.065),
        );
    }
    append(&mut scene, timber, [1.; 4], Some(weathered_wood()), false);
    let mut rocks = Mesh::default();
    for (x, z, r) in [
        (-18., 2., 0.85),
        (-14., 11., 0.6),
        (15., 5., 0.75),
        (13., -8., 0.9),
        (-10., -10., 0.5),
    ] {
        boulder(
            &mut rocks,
            Vec3::new(x, ground_height(x, z) + r * 0.15, z),
            r,
        );
    }
    append(
        &mut scene,
        rocks,
        [0.75, 0.73, 0.69, 1.],
        Some(surfaces::rock(13).bake(256)),
        false,
    );
    let mut green = Mesh::default();
    let mut white = Mesh::default();
    let mut violet = Mesh::default();
    let mut yellow = Mesh::default();
    // Sunny borders in flower, broad foliage in the oak's shade, and tall pond margins.
    for z in -76..76 {
        for x in -76..76 {
            let (x, z) = (x as f32 * 0.55, z as f32 * 0.55);
            let hash = ((x * 17.7 + z * 31.3).sin() * 4381.2).fract().abs();
            let bed = bed_at(x, z);
            let shore = pond_distance(x, z);
            if bed.is_none() && !(0.1..1.5).contains(&shore) {
                continue;
            }
            if path_distance(x, z) < 1.1 || hash < 0.3 {
                continue;
            }
            let root = Vec3::new(x, ground_height(x, z) + 0.01, z);
            let height = if bed == Some(true) {
                0.35
            } else if bed.is_some() {
                0.45 + hash * 0.55
            } else {
                0.7 + hash * 0.65
            };
            for i in 0..6 {
                let a = i as f32 * 2.39996 + hash * 6.;
                let out = Vec3::new(a.cos(), 0.5, a.sin()).normalize();
                leaf(
                    &mut green,
                    root,
                    out,
                    height,
                    if bed == Some(true) { 0.17 } else { 0.025 },
                );
            }
            if bed == Some(false) {
                let top = root + Vec3::Y * height;
                beam(&mut green, root, top, 0.008);
                let flower = if hash > 0.8 {
                    &mut white
                } else if hash > 0.55 {
                    &mut yellow
                } else {
                    &mut violet
                };
                for i in 0..8 {
                    let a = i as f32 * 0.7854;
                    let out = Vec3::new(a.cos(), 0.1, a.sin());
                    leaf(flower, top, out, 0.045, 0.025);
                }
            }
        }
    }
    // Lily pads and white flowers over the basin, away from the bridge.
    for i in 0..130 {
        let a = i as f32 * 2.39996;
        let r = 2. + ((i * 37 % 101) as f32 / 101.) * 13.;
        let (x, z) = (-4. + a.cos() * r, a.sin() * r * 0.65);
        if pond_distance(x, z) > -1.2 || (x - BRIDGE_X).abs() < 1.3 {
            continue;
        }
        let center = Vec3::new(x, 0.035, z);
        let radius = 0.12 + (i % 4) as f32 * 0.035;
        for k in 1..12 {
            let p = |k: usize| {
                center
                    + Vec3::new(
                        (k as f32 * 0.483).cos() * radius,
                        0.,
                        (k as f32 * 0.483).sin() * radius,
                    )
            };
            quad(&mut green, [center, p(k), p(k + 1), center], Vec3::Y);
        }
        if i % 4 == 0 {
            for k in 0..9 {
                let a = k as f32 * 0.698;
                leaf(
                    &mut white,
                    center + Vec3::Y * 0.05,
                    Vec3::new(a.cos(), 0.4, a.sin()),
                    0.07,
                    0.035,
                );
            }
        }
    }
    append(&mut scene, green, [0.07, 0.16, 0.035, 1.], None, true);
    append(&mut scene, white, [0.82, 0.81, 0.72, 1.], None, true);
    append(&mut scene, violet, [0.29, 0.13, 0.45, 1.], None, true);
    append(&mut scene, yellow, [0.8, 0.59, 0.03, 1.], None, true);
    scene
}

pub fn add_site(scene: &mut SceneAsset) {
    static SCENERY: OnceLock<SceneAsset> = OnceLock::new();
    let source = SCENERY.get_or_init(scenery);
    let (mesh, material, texture) = (
        scene.meshes.len() as u32,
        scene.materials.len() as u32,
        scene.textures.len() as i32,
    );
    scene.meshes.extend(source.meshes.iter().cloned());
    scene.textures.extend(source.textures.iter().cloned());
    scene
        .materials
        .extend(source.materials.iter().map(|m| Material {
            texture_layer: if m.texture_layer < 0 {
                -1
            } else {
                m.texture_layer + texture
            },
            ..*m
        }));
    scene
        .instances
        .extend(source.instances.iter().map(|i| Instance {
            mesh: i.mesh + mesh,
            material: i.material + material,
            ..*i
        }));
    // Existing water/reflection pipeline, with a calm surface clipped by the dug terrain.
    scene.sea=Some(Sea{level:0.,ocean:"absorption 2.1 1.5 2.0\nscattering 0.012 0.017 0.008\nturbidity 0.5\nfoam 0\nchoppiness 0\nswell height=0 period=1 from=0\nfetch 0.01\n".into()});
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pond_bridge_paths_and_specimens_share_one_site() {
        assert!(pond_distance(-6.6, 0.4) < -5.);
        let (a, b) = bridge_ends();
        assert!((3.5..10.).contains(&(b - a)));
        assert!(walk_height(BRIDGE_X, (a + b) * 0.5) > 0.3);
        assert!(ground_height(BRIDGE_X, (a + b) * 0.5) < 0.);
        assert!(path_distance(1.6, 23.) < 3. && walk_allowed(1.6, 23.));
        for (_, x, z, _, _) in SPECIMENS {
            assert!(pond_distance(x, z) > 1.);
            assert!(ground_height(x, z) > 0.);
        }
        assert_eq!(placements(42, 100), placements(42, 100));
        assert_eq!(placements(42, 100).len(), 112);
        assert!(grass_kind(-6.6, 0.4).is_none());
    }
    #[test]
    fn scenery_has_reflection_water_and_valid_draw_materials() {
        let mut reed=Mesh::default();
        leaf(&mut reed,Vec3::ZERO,Vec3::new(1.,0.5,0.).normalize(),1.,0.025);
        let area:f32=reed.indices.chunks_exact(3).map(|t| {
            let [a,b,c]=[reed.vertices[t[0] as usize].position,reed.vertices[t[1] as usize].position,reed.vertices[t[2] as usize].position];
            (b-a).cross(c-a).length()*0.5
        }).sum();
        assert!(area<0.025,"curving a narrow leaf must not inflate it into a broad triangle");
        let mut scene = SceneAsset {
            texture_size: 256,
            ..Default::default()
        };
        add_site(&mut scene);
        assert!(
            scene.meshes.len() >= 8,
            "paths, soil, timber, rocks and planting are distinct assets"
        );
        let water = scene.sea.as_ref().unwrap();
        assert!(solstice_render::Ocean::parse(&water.ocean).is_ok());
        for instance in &scene.instances {
            assert!((instance.mesh as usize) < scene.meshes.len());
            let m = &scene.materials[instance.material as usize];
            assert!(m.texture_layer < 0 || (m.texture_layer as usize) < scene.textures.len());
        }
        let spawn = walk_spawn(-6.6, 0.4);
        assert!(
            walk_allowed(spawn.x, spawn.z),
            "switching to Walk above the pond finds a safe bank or bridge"
        );
    }
}
