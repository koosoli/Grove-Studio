// Scene data as laid out in GPU buffers. Twin of solstice-asset's `gpu` module and
// solstice-render's `scene` module; change them together.

struct Cluster {
    center: vec3f,
    radius: f32,
    lod_center: vec3f,
    lod_radius: f32,
    lod_error: f32,
    vertex_offset: u32,
    triangle_offset: u32,
    counts: u32, // vertex_count | triangle_count << 8 | CLUSTER_CHILDREN_MISSING
}

struct Node {
    center: vec3f,
    radius: f32,
    lod_center: vec3f,
    lod_radius: f32,
    max_parent_error: f32,
    first: u32,
    count: u32,
    is_leaf: u32, // 0 for an interior node. See the NODE_ constants in vg/cull.wgsl.
}

// x y | z nx | ny u | v _
struct PackedVertex {
    a: u32,
    b: u32,
    c: u32,
    d: u32,
}

struct Instance {
    // Affine transforms stored as the top three rows of the matrix.
    world_0: vec4f,
    world_1: vec4f,
    world_2: vec4f,
    prev_world_0: vec4f,
    prev_world_1: vec4f,
    prev_world_2: vec4f,
    normal_0: vec4f,
    normal_1: vec4f,
    normal_2: vec4f,
    quant_origin: vec3f,
    quant_step: f32,
    uv_origin: vec2f,
    uv_step: vec2f,
    root_node: u32,
    // Material index, with the INSTANCE_ flags in its top bits so passes need not look them up.
    material: u32,
    max_scale: f32,
    // Texture layer << 16 | (log2(uv scale) + 128) * 256, or NO_TEXTURE.
    texture_info: u32,
    // How brightly the material's own light shines here: 1 as it is, 0 dark.
    glow: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

struct Material {
    base_color: vec4f,
    emissive: vec3f,
    flags: u32, // MATERIAL_TWO_SIDED | MATERIAL_FOLIAGE
    roughness: f32,
    metallic: f32,
    uv_scale: f32,
    normal_strength: f32,
    texture_layer: i32,
    macro_variation: f32,
    // Which texture array holds the layer right now, and where in it.
    texture_tier: u32,
    texture_slot: u32,
}

struct Light {
    position: vec3f,
    range: f32,
    intensity: vec3f,
    _pad: u32,
}

// One selected cluster. The index of this entry is what the visibility buffer stores.
struct VisibleCluster {
    instance: u32,
    cluster: u32,
}

// Twins of Material::TWO_SIDED and Material::FOLIAGE.
const MATERIAL_TWO_SIDED: u32 = 1u;
const MATERIAL_FOLIAGE: u32 = 2u;
// Twins of the INSTANCE_ flags in scene.rs.
const INSTANCE_TWO_SIDED: u32 = 0x80000000u;
const INSTANCE_MOVABLE: u32 = 0x40000000u;
const INSTANCE_HIDDEN: u32 = 0x20000000u;
const INSTANCE_STAYS_IN_GI: u32 = 0x10000000u;
const INSTANCE_WIND: u32 = 0x08000000u;
const INSTANCE_WIND_LEAF: u32 = 0x04000000u;
const INSTANCE_MATERIAL_MASK: u32 = 0x03ffffffu;

fn instance_material(i: Instance) -> u32 {
    return i.material & INSTANCE_MATERIAL_MASK;
}

fn instance_movable(i: Instance) -> bool {
    return (i.material & INSTANCE_MOVABLE) != 0u;
}

fn instance_hidden(i: Instance) -> bool {
    return (i.material & INSTANCE_HIDDEN) != 0u;
}

fn instance_stays_in_gi(i: Instance) -> bool {
    return (i.material & INSTANCE_STAYS_IN_GI) != 0u;
}

fn instance_two_sided(i: Instance) -> bool {
    return (i.material & INSTANCE_TWO_SIDED) != 0u;
}

fn cluster_triangle_count(c: Cluster) -> u32 {
    return (c.counts >> 8u) & 0xffu;
}

fn cluster_vertex_count(c: Cluster) -> u32 {
    return c.counts & 0xffu;
}

fn transform_point(i: Instance, p: vec3f) -> vec3f {
    let h = vec4f(p, 1.0);
    return vec3f(dot(i.world_0, h), dot(i.world_1, h), dot(i.world_2, h));
}

fn transform_point_prev(i: Instance, p: vec3f) -> vec3f {
    let h = vec4f(p, 1.0);
    return vec3f(dot(i.prev_world_0, h), dot(i.prev_world_1, h), dot(i.prev_world_2, h));
}

fn vegetation_angle(i: Instance, p: vec3f, clock: f32, origin_xz: vec2f, params: vec4f) -> f32 {
    if clock <= 0.0 || params.x <= 0.0 || (i.material & INSTANCE_WIND) == 0u {
        return 0.0;
    }
    let encoded = clock - 1.0;
    let t = encoded % 10000.0;
    let phase = dot(origin_xz, vec2f(0.37, 0.29));
    // Slow, coherent bending: the whole crown follows the trunk, with only a slight
    // spatial phase variation. Growing the angle with height keeps the foot planted.
    let spatial = dot(p.xz, vec2f(0.025, 0.02));
    let sway = sin(t * 0.55 + phase + spatial) * 0.7 + sin(t * 0.93 + phase * 1.7 + spatial) * 0.3;
    let gust = 1.0 + params.y * (0.5 + 0.5 * sin(t * 0.19 + phase));
    let height = pow(clamp(max(p.y, 0.0) / 12.0, 0.0, 3.0), 0.6);
    return clamp(0.025 * params.x * gust * height * sway, -0.3, 0.3);
}

// Steady-state response of a damped oscillator to a sinusoidal wind component. This
// introduces a spring-like phase lag without frame-rate-dependent integration.
fn spring_response(phase: f32, drive: f32, natural: f32, damping: f32) -> f32 {
    let real = natural * natural - drive * drive;
    let imag = 2.0 * damping * natural * drive;
    let gain = min(natural * natural / max(length(vec2f(real, imag)), 0.001), 1.6);
    return gain * sin(phase - atan2(imag, real));
}

fn vegetation_wind(i: Instance, p: vec3f, uv: vec2f, clock: f32, origin_xz: vec2f, params: vec4f, dynamics: vec4f) -> vec3f {
    if clock <= 0.0 || params.x <= 0.0 || (i.material & INSTANCE_WIND) == 0u {
        return vec3f(0.0);
    }
    let encoded = clock - 1.0;
    let direction = floor(encoded / 10000.0) * 0.3926990817;
    let t = encoded % 10000.0;
    let dir = vec2f(sin(direction), cos(direction));
    let side = vec2f(-dir.y, dir.x);
    let phase = dot(origin_xz, vec2f(0.37, 0.29));
    let angle = vegetation_angle(i, p, clock, origin_xz, params);
    let world = transform_point(i, p) - vec3f(i.world_0.w, i.world_1.w, i.world_2.w);
    let along = dot(world.xz, dir);
    let bend_x = along * (cos(angle) - 1.0) + world.y * sin(angle);
    let bend_y = world.y * (cos(angle) - 1.0) - along * sin(angle);
    var bend = vec3f(dir.x * bend_x, bend_y, dir.y * bend_x);
    // A low-amplitude, continuous crown field adds locally different branch responses.
    // It fades in away from the trunk; wood and leaf bases at the same place use the
    // same displacement. Without the source branch hierarchy this is an approximation,
    // not a separate rotation about each branch joint.
    let reach = smoothstep(0.25, 2.5, length(p.xz));
    let exposure = clamp(max(p.y, 0.0) / 8.0, 0.0, 1.0);
    let branch_phase = dot(p.xz, vec2f(0.72, 0.53)) + p.y * 0.21 + phase;
    let branch_gust = 1.0 + params.y * (0.5 + 0.5 * sin(t * 0.19 + phase));
    let spring = clamp(dynamics.x, 0.0, 1.0);
    let natural = mix(3.5, 1.7, spring) * mix(1.0, 1.2, reach);
    let damping = mix(0.8, 0.42, spring);
    let along_response = spring_response(t * 1.31 + branch_phase, 1.31, natural, damping);
    let cross_response = spring_response(t * 1.73 + branch_phase * 1.3, 1.73, natural, damping);
    let branch_sway = dir * along_response + side * 0.45 * cross_response;
    let branch_motion = 0.085 * params.x * params.w * branch_gust * reach * exposure * branch_sway;
    bend += vec3f(branch_motion.x, 0.0, branch_motion.y);
    // Each preview leaf has a stable phase in the integer portion of its U coordinate.
    // At v=0 its flutter is zero, so the stalk follows exactly the same bend as its branch.
    // Baked spray cards represent clusters, so they deliberately receive no tip flutter.
    if (i.material & INSTANCE_WIND_LEAF) != 0u {
        let id = floor(uv.x + 0.001);
        let individual = id * 2.39996323 + phase + dot(p.xz, vec2f(0.71, 1.13));
        var tip = 0.0;
        if id >= 16.0 { tip = clamp(uv.y, 0.0, 1.0); }
        else if uv.y >= 0.27 && uv.y <= 0.48 { tip = clamp((uv.y - 0.27) / 0.21, 0.0, 1.0); }
        let flutter = dir * sin(t * 4.0 + individual) + side * sin(t * 3.1 + individual * 1.37);
        let exposure = clamp(max(p.y, 0.0) / 4.0, 0.0, 1.0);
        let leaf_motion = 0.004 * params.x * params.z * (1.0 + params.y) * exposure * tip * tip * flutter;
        bend += vec3f(leaf_motion.x, 0.0, leaf_motion.y);
    }
    return bend;
}

fn transform_wind_normal(i: Instance, n: vec3f, p: vec3f, clock: f32, params: vec4f) -> vec3f {
    let normal = transform_normal(i, n);
    let angle = vegetation_angle(i, p, clock, vec2f(i.world_0.w, i.world_2.w), params);
    let direction = floor(max(clock - 1.0, 0.0) / 10000.0) * 0.3926990817;
    let dir = vec2f(sin(direction), cos(direction));
    let along = dot(normal.xz, dir);
    let bent_x = along * cos(angle) + normal.y * sin(angle);
    return normalize(vec3f(normal.x + dir.x * (bent_x - along), normal.y * cos(angle) - along * sin(angle), normal.z + dir.y * (bent_x - along)));
}

fn transform_vegetation(i: Instance, p: vec3f, uv: vec2f, clock: f32, params: vec4f, dynamics: vec4f) -> vec3f {
    return transform_point(i, p) + vegetation_wind(i, p, uv, clock, vec2f(i.world_0.w, i.world_2.w), params, dynamics);
}

fn vegetation_padding(i: Instance, center: vec3f, radius: f32, clock: f32, params: vec4f) -> f32 {
    if clock <= 0.0 || params.x <= 0.0 || (i.material & INSTANCE_WIND) == 0u {
        return 0.0;
    }
    let height = pow(clamp(max(center.y + radius, 0.0) / 12.0, 0.0, 3.0), 0.6);
    let max_angle = min(0.025 * params.x * (1.0 + params.y) * height, 0.3);
    let sweep = 2.0 * sin(max_angle * 0.5) * (length(center) + radius) * i.max_scale;
    let flutter = 0.006 * params.x * params.z * (1.0 + params.y);
    let branches = 0.16 * params.x * params.w * (1.0 + params.y);
    return sweep + flutter + branches;
}

fn transform_normal(i: Instance, n: vec3f) -> vec3f {
    let h = vec4f(n, 0.0);
    return normalize(vec3f(dot(i.normal_0, h), dot(i.normal_1, h), dot(i.normal_2, h)));
}

fn decode_position(i: Instance, v: PackedVertex) -> vec3f {
    let grid = vec3f(f32(v.a & 0xffffu), f32(v.a >> 16u), f32(v.b & 0xffffu));
    return i.quant_origin + grid * i.quant_step;
}

fn sign_extend_16(x: u32) -> f32 {
    return f32(i32(x << 16u) >> 16u);
}

fn decode_normal(v: PackedVertex) -> vec3f {
    let e = vec2f(sign_extend_16(v.b >> 16u), sign_extend_16(v.c & 0xffffu)) / 32767.0;
    return oct_decode(e);
}

fn decode_uv(i: Instance, v: PackedVertex) -> vec2f {
    return i.uv_origin + vec2f(f32(v.c >> 16u), f32(v.d & 0xffffu)) * i.uv_step;
}
