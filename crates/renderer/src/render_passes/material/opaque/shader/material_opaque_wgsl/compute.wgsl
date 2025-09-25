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
    // early return if nothing was drawn at this pixel
    if (triangle_id == f32_max) {
        return;
    }
    let material_meta_offset = bitcast<u32>(visibility_data.y);
    let barycentric = vec3<f32>(visibility_data.z, visibility_data.w, 1.0 - visibility_data.z - visibility_data.w);


    let mesh_meta = mesh_metas[material_meta_offset / meta_size_in_bytes];
    let material_offset = mesh_meta.material_offset;
    let material = get_material(material_offset);
    // early return if this shader pass isn't meant to run for this material
    if !should_run(material) {
        return;
    }
    let vertex_attribute_stride = mesh_meta.vertex_attribute_stride / 4; // 4 bytes per float
    let attribute_indices_offset = mesh_meta.vertex_attribute_indices_offset / 4;
    let attribute_data_offset = mesh_meta.vertex_attribute_data_offset / 4;

    let color = calculate_color(attribute_indices_offset, attribute_data_offset, triangle_id, material, barycentric, vertex_attribute_stride);

    // Write to output texture
    textureStore(opaque_tex, coords, color);
}

fn should_run(material: Material) -> bool {
    {%- match uv_sets %}
        {% when Some with (uv_sets) %}
            return material_uses_uv_count(material, {{ uv_sets }});
        {% when None %}
            return !material_has_any_uvs(material);
    {% endmatch %}
}

// TODO!
fn material_has_any_uvs(material: Material) -> bool {
    return false;
}

// TODO!
fn material_uses_uv_count(material: Material, uv_set_count: u32) -> bool {
    return true;
}

fn calculate_color(attribute_indices_offset: u32, attribute_data_offset: u32, triangle_id: u32, material: Material, barycentric: vec3<f32>, vertex_attribute_stride: u32) -> vec4<f32> {
    let triangle_indices = get_triangle_indices(attribute_indices_offset, triangle_id);

    var color = texture_load_base_color(material, base_color_tex_uv(attribute_data_offset, triangle_indices, barycentric, material.base_color_tex_info, vertex_attribute_stride));

    return color;
}
