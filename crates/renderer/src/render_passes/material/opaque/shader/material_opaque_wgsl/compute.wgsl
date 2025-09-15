{% include "pbr_shared_wgsl/color_space.wgsl" %}
{% include "pbr_shared_wgsl/projection.wgsl" %}
{% include "pbr_shared_wgsl/material.wgsl" %}
{% include "pbr_shared_wgsl/textures.wgsl" %}
{% include "pbr_shared_wgsl/debug.wgsl" %}
{% include "pbr_shared_wgsl/meta.wgsl" %}
{% include "material_opaque_wgsl/attribute.wgsl" %}

@group(0) @binding(0) var<storage, read> mesh_metas: array<MaterialMeshMeta>;
@group(0) @binding(1) var visibility_data_tex: texture_2d<f32>;
@group(0) @binding(2) var opaque_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(3) var<storage, read> materials: array<MaterialRaw>;
@group(0) @binding(4) var<storage, read> attribute_indices: array<u32>;
@group(0) @binding(5) var<storage, read> attribute_data: array<f32>;


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
    let material_meta_offset = bitcast<u32>(visibility_data.y);
    let barycentric = vec3<f32>(visibility_data.z, visibility_data.w, 1.0 - visibility_data.z - visibility_data.w);

    var color = vec4<f32>(0.0, 0.0, 0.0, 0.0);

    // only calculate color if material_offset is valid
    if (triangle_id != f32_max) {
        let mesh_meta = mesh_metas[material_meta_offset / meta_size_in_bytes];
        let material_offset = mesh_meta.material_offset;
        let material = get_material(material_offset);
        let vertex_attribute_stride = mesh_meta.vertex_attribute_stride / 4; // 4 bytes per float
        let attribute_indices_offset = mesh_meta.vertex_attribute_indices_offset / 4;
        let attribute_data_offset = mesh_meta.vertex_attribute_data_offset / 4;
        color = calculate_color(attribute_indices_offset, attribute_data_offset, triangle_id, material, barycentric, vertex_attribute_stride);

        // DEBUG
        // if material.base_color_tex_info.attribute_uv_index == 0u {
        //     color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
        // }
    }

    // Write to output texture
    textureStore(opaque_tex, coords, color);
}
fn calculate_color_debug(attribute_indices_offset: u32, attribute_data_offset: u32, triangle_id: u32, material: Material, barycentric: vec3<f32>, vertex_attribute_stride: u32) -> vec4<f32> {

    let triangle_indices = get_triangle_indices(attribute_indices_offset, triangle_id);

    if (triangle_indices.x == 0u && barycentric.x > 0.8) {
            let uv = get_uv(attribute_data_offset, material.base_color_tex_info.attribute_uv_index, 0u, vertex_attribute_stride);
            return vec4<f32>(uv.x, uv.y, 1.0, 1.0);  // Blue tint for vertex 0
        } else if (triangle_indices.x == 1u && barycentric.x > 0.8) {
            let uv = get_uv(attribute_data_offset, material.base_color_tex_info.attribute_uv_index, 1u, vertex_attribute_stride);
            return vec4<f32>(uv.x, uv.y, 0.0, 1.0);  // No blue for vertex 1
        } else if (triangle_indices.x == 2u && barycentric.x > 0.8) {
            let uv = get_uv(attribute_data_offset, material.base_color_tex_info.attribute_uv_index, 2u, vertex_attribute_stride);
            return vec4<f32>(1.0, uv.x, uv.y, 1.0);  // Red tint for vertex 2
        } else if (triangle_indices.x == 3u && barycentric.x > 0.8) {
            let uv = get_uv(attribute_data_offset, material.base_color_tex_info.attribute_uv_index, 3u, vertex_attribute_stride);
            return vec4<f32>(0.0, 1.0, uv.y, 1.0);  // Green tint for vertex 3
        } else {
            // Normal interpolation for comparison
            let uv0 = get_uv(attribute_data_offset, material.base_color_tex_info.attribute_uv_index, triangle_indices.x, vertex_attribute_stride);
            let uv1 = get_uv(attribute_data_offset, material.base_color_tex_info.attribute_uv_index, triangle_indices.y, vertex_attribute_stride);
            let uv2 = get_uv(attribute_data_offset, material.base_color_tex_info.attribute_uv_index, triangle_indices.z, vertex_attribute_stride);
            let interpolated_uv = barycentric.x * uv0 + barycentric.y * uv1 + barycentric.z * uv2;
            return vec4<f32>(interpolated_uv.x, interpolated_uv.y, 0.0, 1.0);
        }




    // DEBUG: Show each triangle's indices more clearly
        // Scale up the indices to make them visible
        // return vec4<f32>(
        //     f32(triangle_indices.x) / 4.0,  // Red channel for first index
        //     f32(triangle_indices.y) / 4.0,  // Green channel for second index
        //     f32(triangle_indices.z) / 4.0,  // Blue channel for third index
        //     1.0
        // );

    // DEBUG: Show actual triangle indices to see the pattern // Use different colors for different index combinations

    // // DEBUG: Visualize triangle winding order
    //     // Check if indices are in ascending order vs mixed order
    //     let is_ascending = (triangle_indices.x < triangle_indices.y) && (triangle_indices.y < triangle_indices.z);
    //     let is_descending = (triangle_indices.x > triangle_indices.y) && (triangle_indices.y > triangle_indices.z);

    //     if (is_ascending) {
    //         return vec4<f32>(0.0, 1.0, 0.0, 1.0);  // Green for ascending order
    //     } else if (is_descending) {
    //         return vec4<f32>(1.0, 0.0, 0.0, 1.0);  // Red for descending order
    //     } else {
    //         return vec4<f32>(0.0, 0.0, 1.0, 1.0);  // Blue for mixed order
    //     }

}

fn calculate_color(attribute_indices_offset: u32, attribute_data_offset: u32, triangle_id: u32, material: Material, barycentric: vec3<f32>, vertex_attribute_stride: u32) -> vec4<f32> {
    let triangle_indices = get_triangle_indices(attribute_indices_offset, triangle_id);

    var color = texture_load_base_color(material, base_color_tex_uv(attribute_data_offset, triangle_indices, barycentric, material.base_color_tex_info, vertex_attribute_stride));

    return color;
}
