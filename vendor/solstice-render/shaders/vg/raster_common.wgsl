// Bindings shared by the software and hardware rasterizers.

#include "common/math.wgsl"
#include "common/scene.wgsl"
#include "vg/pass_view.wgsl"

@group(0) @binding(0) var<uniform> view: PassView;
@group(0) @binding(1) var<storage, read> instances: array<Instance>;
@group(0) @binding(2) var<storage, read> clusters: array<Cluster>;
@group(0) @binding(3) var<storage, read> vertices: array<PackedVertex>;
@group(0) @binding(4) var<storage, read> triangles: array<u32>;
@group(0) @binding(5) var<storage, read> visible: array<VisibleCluster>;
@group(0) @binding(6) var<storage, read> state: CullStateRead;
@group(0) @binding(7) var<storage, read> raster_materials: array<Material>;
@group(0) @binding(8) var raster_sampler: sampler;
@group(0) @binding(9) var raster_base_color: texture_2d_array<f32>;
@group(0) @binding(10) var raster_middle_color: texture_2d_array<f32>;
@group(0) @binding(11) var raster_top_color: texture_2d_array<f32>;

fn card_visible(material_id: u32, uv: vec2f) -> bool {
    let material = raster_materials[material_id];
    if (material.flags & 32u) == 0u || material.texture_layer < 0 { return true; }
    let coord = uv * material.uv_scale;
    let slot = i32(material.texture_slot);
    var alpha: f32;
    switch material.texture_tier {
        case 2u: { alpha = textureSampleLevel(raster_top_color, raster_sampler, coord, slot, 0.0).a; }
        case 1u: { alpha = textureSampleLevel(raster_middle_color, raster_sampler, coord, slot, 0.0).a; }
        default: { alpha = textureSampleLevel(raster_base_color, raster_sampler, coord, slot, 0.0).a; }
    }
    return alpha >= 0.5;
}

// Visibility ID: visible-cluster index in the high bits, triangle in the low 7.
fn encode_id(visible_index: u32, triangle: u32) -> u32 {
    return (visible_index << 7u) | triangle;
}

fn local_vertex(packed_triangle: u32, corner: u32) -> u32 {
    return (packed_triangle >> (corner * 8u)) & 0xffu;
}
