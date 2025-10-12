{% include "all_material_shared_wgsl/color_space.wgsl" %}
{% include "all_material_shared_wgsl/debug.wgsl" %}
{% include "all_material_shared_wgsl/math.wgsl" %}
{% include "all_material_shared_wgsl/meta.wgsl" %}
{% include "all_material_shared_wgsl/projection.wgsl" %}
{% include "all_material_shared_wgsl/textures.wgsl" %}
{% include "pbr_shared_wgsl/lighting/brdf.wgsl" %}
{% include "pbr_shared_wgsl/lighting/unlit.wgsl" %}
{% include "pbr_shared_wgsl/material.wgsl" %}
{% include "pbr_shared_wgsl/material_color.wgsl" %}

@group(0) @binding(0) var<storage, read> mesh_metas: array<MaterialMeshMeta>;
@group(0) @binding(1) var visibility_data_tex: texture_2d<f32>;
@group(0) @binding(2) var opaque_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(3) var<storage, read> materials: array<PbrMaterialRaw>; // TODO - just raw data, derive PbrMaterialRaw if that's what we have?
@group(0) @binding(4) var<storage, read> attribute_indices: array<u32>;
@group(0) @binding(5) var<storage, read> attribute_data: array<f32>;
{% for b in texture_bindings %}
    @group({{ b.group }}u) @binding({{ b.binding }}u) var atlas_tex_{{ b.atlas_index }}: texture_2d_array<f32>;
{% endfor %}


const f32_max = 2139095039u;

const ambient = vec3<f32>(1.0); // TODO - make this settable, or get from IBL

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

    let triangle_index = bitcast<u32>(visibility_data.x);
    // early return if nothing was drawn at this pixel
    if (triangle_index == f32_max) {
        return;
    }
    let material_meta_offset = bitcast<u32>(visibility_data.y);
    let barycentric = vec3<f32>(visibility_data.z, visibility_data.w, 1.0 - visibility_data.z - visibility_data.w);


    let mesh_meta = mesh_metas[material_meta_offset / meta_size_in_bytes];
    let material_offset = mesh_meta.material_offset;

    let pbr_material = pbr_get_material(material_offset);

    // early return if this shader pass isn't meant to run for this material
    // if !pbr_should_run(pbr_material) {
    //     return;
    // }

    let vertex_attribute_stride = mesh_meta.vertex_attribute_stride / 4; // 4 bytes per float
    let attribute_indices_offset = mesh_meta.vertex_attribute_indices_offset / 4;
    let attribute_data_offset = mesh_meta.vertex_attribute_data_offset / 4;

    let material_color = pbr_get_material_color(attribute_indices_offset, attribute_data_offset, triangle_index, pbr_material, barycentric, vertex_attribute_stride);

    // TODO - get surface_to_camera
    //let surface_to_camera = normalize(camera.position - input.world_position);
    let surface_to_camera = vec3<f32>(0.0, 0.0, 1.0);

    var color = vec3<f32>(0.0);

    // TODO - lighting
    // something like:
    // for(var i = 0u; i < n_lights; i = i + 1u) {
    //     let light_brdf = light_to_brdf(get_light(i), normal, input.world_position);

    //     if (light_brdf.n_dot_l > 0.0001) {
    //         color += brdf(input, material, light_brdf, ambient, surface_to_camera);
    //     } else {
    //         color += ambient * material.base_color.rgb;
    //     }
    // }
    //
    // For now, just color with full material color (emissive etc.) but unlit

    color = unlit(material_color, ambient, surface_to_camera);

    // Write to output texture
    textureStore(opaque_tex, coords, vec4<f32>(color, material_color.base.a));
}

fn pbr_should_run(material: PbrMaterial) -> bool {
    {%- match uv_sets %}
        {% when Some with (uv_sets) %}
            return pbr_material_uses_uv_count(material, {{ uv_sets }});
        {% when None %}
            return !pbr_material_has_any_uvs(material);
    {% endmatch %}
}

// TODO!
fn pbr_material_has_any_uvs(material: PbrMaterial) -> bool {
    if material.has_base_color_texture {
        return true;
    }

    return false;
}

// TODO!
fn pbr_material_uses_uv_count(material: PbrMaterial, uv_set_count: u32) -> bool {
    if material.has_base_color_texture {
        return uv_set_count > 0;
    }
    return false;
}
