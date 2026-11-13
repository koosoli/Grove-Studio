// Grove grass preview: opaque blades bend in a continuous field. The engine's runtime
// grass renderer still supplies its own placement, interaction and ring LODs.
fn grass_preview_offset(p: vec3f, origin: vec2f, clock: f32, params: vec4f) -> vec3f {
    if clock <= 0.0 || params.x <= 0.0 || p.y <= 0.0 { return vec3f(0.0); }
    let encoded = clock - 1.0;
    let direction = floor(encoded / 10000.0) * 0.3926990817;
    let t = encoded % 10000.0;
    let gust = vegetation_gust(origin + p.xz, t, direction, params.y);
    let phase = dot(origin + p.xz, vec2f(1.3, 0.9));
    let angle = clamp(p.y * 0.4 * params.x * gust * (0.7 + 0.3 * sin(t*2.1 + phase)), -0.7, 0.7);
    let dir = vec2f(sin(direction), cos(direction));
    return vec3f(dir.x*p.y*sin(angle), p.y*(cos(angle)-1.0), dir.y*p.y*sin(angle));
}

// Grove stores the actual stem tree in otherwise unused material records. The unused high
// half of each packed vertex's d word names its source stem (zero means no hierarchy).
fn hierarchy_local(i: Instance, p: vec3f, vertex: PackedVertex, clock: f32, params: vec4f, dynamics: vec4f, vector: bool) -> vec3f {
    let material = wind_records[instance_material(i)];
    var id = vertex.d >> 16u;
    if (material.flags & 0x01000000u) == 0u || id == 0u || clock <= 0.0 || params.x <= 0.0 || params.w <= 0.0 { return p; }
    let base = u32(material.emissive.x + 0.5);
    let encoded = clock - 1.0;
    let direction = floor(encoded / 10000.0) * 0.3926990817;
    let t = encoded % 10000.0;
    let world_dir = vec3f(sin(direction), 0.0, cos(direction));
    let local_dir = normalize(vec3f(dot(vec3f(i.world_0.x, i.world_1.x, i.world_2.x), world_dir), dot(vec3f(i.world_0.y, i.world_1.y, i.world_2.y), world_dir), dot(vec3f(i.world_0.z, i.world_1.z, i.world_2.z), world_dir)));
    var result = p;
    for (var depth = 0u; depth < 16u && id > 0u; depth++) {
        let branch = wind_records[base + id - 1u];
        let parent = u32(branch.base_color.w + 0.5);
        if parent == 0u { break; } // the trunk is bent separately around its base
        let pivot = branch.base_color.xyz;
        let stem = branch.emissive - pivot;
        let length_stem = max(length(stem), 0.0001);
        let cross_axis = cross(stem / length_stem, local_dir);
        let axis_length = length(cross_axis);
        if axis_length > 0.0001 {
            let axis = cross_axis / axis_length;
            let spring = clamp(dynamics.x, 0.0, 1.0);
            let natural = mix(3.5, 1.7, spring) * (1.0 + min(branch.roughness * 3.0, 1.0));
            let damping = mix(0.8, 0.42, spring);
            let phase = dot(pivot, vec3f(0.73, 0.27, 0.91)) + f32(id) * 0.618 + dot(vec2f(i.world_0.w, i.world_2.w), vec2f(0.37, 0.29));
            let response = spring_response(t * 1.31 + phase, 1.31, natural, damping);
            let flexibility = clamp(length_stem / max(branch.roughness * 35.0, 0.1), 0.2, 1.0);
            let gust = vegetation_gust(vec2f(i.world_0.w, i.world_2.w), t, direction, params.y);
            let angle = clamp(0.045 * params.x * params.w * gust * flexibility * response * axis_length, -0.12, 0.12);
            let relative = result - select(pivot, vec3f(0.0), vector);
            let rotated = relative * cos(angle) + cross(axis, relative) * sin(angle) + axis * dot(axis, relative) * (1.0 - cos(angle));
            result = rotated + select(pivot, vec3f(0.0), vector);
        }
        id = parent;
    }
    return result;
}

fn transform_vegetation(i: Instance, p: vec3f, uv: vec2f, vertex: PackedVertex, clock: f32, params: vec4f, dynamics: vec4f) -> vec3f {
    let material = wind_records[instance_material(i)];
    if (material.flags & 0x02000000u) != 0u { return transform_point(i, p) + grass_preview_offset(p, vec2f(i.world_0.w,i.world_2.w), clock, params); }
    let hierarchical = (material.flags & 0x01000000u) != 0u && (vertex.d >> 16u) != 0u;
    let local = hierarchy_local(i, p, vertex, clock, params, dynamics, false);
    let legacy = vec4f(params.xyz, select(params.w, 0.0, hierarchical));
    return transform_point(i, local) + vegetation_wind(i, local, uv, clock, vec2f(i.world_0.w, i.world_2.w), legacy, dynamics);
}

fn transform_vegetation_prev(i: Instance, p: vec3f, uv: vec2f, vertex: PackedVertex, clock: f32, params: vec4f, dynamics: vec4f) -> vec3f {
    let material = wind_records[instance_material(i)];
    if (material.flags & 0x02000000u) != 0u { return transform_point_prev(i, p) + grass_preview_offset(p, vec2f(i.prev_world_0.w,i.prev_world_2.w), clock, params); }
    let hierarchical = (material.flags & 0x01000000u) != 0u && (vertex.d >> 16u) != 0u;
    let local = hierarchy_local(i, p, vertex, clock, params, dynamics, false);
    let legacy = vec4f(params.xyz, select(params.w, 0.0, hierarchical));
    return transform_point_prev(i, local) + vegetation_wind(i, local, uv, clock, vec2f(i.prev_world_0.w, i.prev_world_2.w), legacy, dynamics);
}
