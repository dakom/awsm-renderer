{% include "pbr_shared_wgsl/color_space.wgsl" %}
{% include "pbr_shared_wgsl/projection.wgsl" %}
{% include "pbr_shared_wgsl/material.wgsl" %}
{% include "pbr_shared_wgsl/textures.wgsl" %}
{% include "pbr_shared_wgsl/debug.wgsl" %}
{% include "material_opaque_wgsl/attribute.wgsl" %}

@group(0) @binding(0) var visibility_data_tex: texture_2d<f32>;
@group(0) @binding(1) var opaque_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(2) var<storage, read> materials: array<MaterialRaw>;
@group(0) @binding(3) var<storage, read> attribute_indices: array<u32>;
@group(0) @binding(4) var<storage, read> attribute_data: array<f32>;

{% for texture_binding_string in texture_binding_strings %}
    {{texture_binding_string}}
{% endfor %}

// TODO - if material_offset goes beyond this, then we need to refactor things
// simple fix would be to add another render target for proper u32 (e.g. just red channel texture)
const f32_max = 2139095039u;

// TODO - bind material uniform buffer, load material properties

@compute @workgroup_size(8, 8)
fn main(
    @builtin(global_invocation_id) gid: vec3<u32>
) {
    let coords = vec2<i32>(gid.xy);
    let dimensions = textureDimensions(opaque_tex);
    
    // Bounds check
    if (coords.x >= i32(dimensions.x) || coords.y >= i32(dimensions.y)) {
        return;
    }

    let visibility_data = textureLoad(visibility_data_tex, coords, 0);

    let triangle_id = bitcast<u32>(visibility_data.x);
    let material_offset = bitcast<u32>(visibility_data.y);
    let barycentric = vec3<f32>(visibility_data.z, visibility_data.w, 1.0 - visibility_data.z - visibility_data.w);

    var color = vec4<f32>(0.0, 0.0, 0.0, 0.0);

    // only calculate color if material_offset is valid
    if (material_offset != f32_max) {
        color = calculate_color(triangle_id, material_offset, barycentric);
    }

    // Write to output texture
    textureStore(opaque_tex, coords, color);
}

fn calculate_color(triangle_id: u32, material_offset: u32, barycentric: vec3<f32>) -> vec4<f32> {
    let triangle_indices = get_triangle_indices(triangle_id);
    let material = get_material(material_offset);

    var color = texture_load_base_color(material, base_color_tex_uv(triangle_indices, barycentric, material.base_color_tex_info));

    return color;
}