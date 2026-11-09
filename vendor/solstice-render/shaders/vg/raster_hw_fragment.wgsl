// Fragment stage of the hardware visibility pass. Writing through the same function
// as the software rasterizer makes both produce one consistent visibility buffer.

#include "vg/raster_hw.wgsl"
#include "vg/vis_write.wgsl"

@fragment
fn fs_visibility(@builtin(position) position: vec4f, @location(0) @interpolate(flat) id: u32,
                 @location(1) uv: vec2f, @location(2) @interpolate(flat) material: u32) {
    if !card_visible(material, uv) { discard; }
    let pixel = vec2u(position.xy);
    let index = pixel.y * u32(view.viewport.x) + pixel.x;
    write_visibility(index, position.z, id);
}

@fragment
fn fs_depth(@location(1) uv: vec2f, @location(2) @interpolate(flat) material: u32) {
    if !card_visible(material, uv) { discard; }
}
