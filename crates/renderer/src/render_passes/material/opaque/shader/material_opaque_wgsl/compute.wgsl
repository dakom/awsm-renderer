{% include "all_material_shared_wgsl/color_space.wgsl" %}
{% include "all_material_shared_wgsl/debug.wgsl" %}
{% include "all_material_shared_wgsl/math.wgsl" %}
{% include "all_material_shared_wgsl/meta.wgsl" %}
{% include "all_material_shared_wgsl/projection.wgsl" %}
{% include "all_material_shared_wgsl/textures.wgsl" %}
{% include "pbr_shared_wgsl/lighting/lights.wgsl" %}
{% include "pbr_shared_wgsl/lighting/brdf.wgsl" %}
{% include "pbr_shared_wgsl/lighting/unlit.wgsl" %}
{% include "pbr_shared_wgsl/material.wgsl" %}
{% include "pbr_shared_wgsl/material_color.wgsl" %}
{% include "material_opaque_wgsl/helpers/mipmap.wgsl" %}
{% include "material_opaque_wgsl/helpers/standard.wgsl" %}
{% include "material_opaque_wgsl/helpers/normal.wgsl" %}

const ATTRIBUTE_POSITION_OFFSET: u32 = {{ position_offset }}u;
const ATTRIBUTE_NORMAL_OFFSET: u32 = {{ normal_offset }}u;

// Mirrors the CPU-side `CameraBuffer` layout. The extra inverse matrices and frustum rays give
// us everything needed to reconstruct world-space positions from a depth value inside this
// compute pass.
struct CameraUniform {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    inv_view: mat4x4<f32>,
    position: vec3<f32>,
    frame_count: u32,
    frustum_rays: array<vec4<f32>, 4>,
};

@group(0) @binding(0) var<storage, read> mesh_metas: array<MaterialMeshMeta>;
@group(0) @binding(1) var visibility_data_tex: texture_2d<f32>;
@group(0) @binding(2) var opaque_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(3) var<storage, read> materials: array<PbrMaterialRaw>; // TODO - just raw data, derive PbrMaterialRaw if that's what we have?
@group(0) @binding(4) var<storage, read> attribute_indices: array<u32>;
@group(0) @binding(5) var<storage, read> attribute_data: array<f32>;
@group(0) @binding(6) var<storage, read> model_transforms: array<mat4x4<f32>>;
@group(0) @binding(7) var<uniform> camera: CameraUniform;
@group(0) @binding(8) var depth_tex: texture_depth_2d;
{% for b in texture_bindings %}
    @group({{ b.group }}u) @binding({{ b.binding }}u) var atlas_tex_{{ b.atlas_index }}: texture_2d_array<f32>;
{% endfor %}
{% for s in sampler_bindings %}
    @group({{ s.group }}u) @binding({{ s.binding }}u) var atlas_sampler_{{ s.sampler_index }}: sampler;
{% endfor %}

fn get_model_transform(byte_offset: u32) -> mat4x4<f32> {
    return model_transforms[byte_offset / 64u];
}

fn normal_matrix_from_model(model: mat4x4<f32>) -> mat3x3<f32> {
    let upper = mat3x3<f32>(
        model[0].xyz,
        model[1].xyz,
        model[2].xyz,
    );
    return transpose(inverse_mat3(upper));
}

fn inverse_mat3(m: mat3x3<f32>) -> mat3x3<f32> {
    let a00 = m[0][0]; let a01 = m[0][1]; let a02 = m[0][2];
    let a10 = m[1][0]; let a11 = m[1][1]; let a12 = m[1][2];
    let a20 = m[2][0]; let a21 = m[2][1]; let a22 = m[2][2];

    let det = a00 * (a11 * a22 - a12 * a21) -
              a01 * (a10 * a22 - a12 * a20) +
              a02 * (a10 * a21 - a11 * a20);

    let inv_det = 1.0 / det;

    return mat3x3<f32>(
        vec3<f32>(
            (a11 * a22 - a12 * a21) * inv_det,
            (a02 * a21 - a01 * a22) * inv_det,
            (a01 * a12 - a02 * a11) * inv_det
        ),
        vec3<f32>(
            (a12 * a20 - a10 * a22) * inv_det,
            (a00 * a22 - a02 * a20) * inv_det,
            (a02 * a10 - a00 * a12) * inv_det
        ),
        vec3<f32>(
            (a10 * a21 - a11 * a20) * inv_det,
            (a01 * a20 - a00 * a21) * inv_det,
            (a00 * a11 - a01 * a10) * inv_det
        )
    );
}


const f32_max = 2139095039u;

const ambient = vec3<f32>(0.3); // TODO - make this settable, or get from IBL


@compute @workgroup_size(8, 8)
fn main(
    @builtin(global_invocation_id) gid: vec3<u32>
) {
    let coords = vec2<i32>(gid.xy);
    let screen_dims = textureDimensions(opaque_tex);
    let screen_dims_i32 = vec2<i32>(i32(screen_dims.x), i32(screen_dims.y));

    // Bounds check
    if (coords.x >= screen_dims_i32.x || coords.y >= screen_dims_i32.y) {
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
    let model_transform = get_model_transform(mesh_meta.transform_offset);
    let normal_matrix = normal_matrix_from_model(model_transform);

    let pbr_material = pbr_get_material(material_offset);

    // Skip work when the mesh doesn't provide enough UV data for the material.
    if !pbr_should_run(pbr_material) {
        return;
    }

    let vertex_attribute_stride = mesh_meta.vertex_attribute_stride / 4; // 4 bytes per float
    let attribute_indices_offset = mesh_meta.vertex_attribute_indices_offset / 4;
    let attribute_data_offset = mesh_meta.vertex_attribute_data_offset / 4;

    let standard_coordinates = get_standard_coordinates(coords, screen_dims);

    // get the vertex indices for this triangle
    let base_triangle_index = attribute_indices_offset + (triangle_index * 3u);
    let triangle_indices = vec3<u32>(attribute_indices[base_triangle_index], attribute_indices[base_triangle_index + 1], attribute_indices[base_triangle_index + 2]);

    {% if normals %}
        let normal = get_normal(
            triangle_indices,
            barycentric,
            attribute_data_offset,
            vertex_attribute_stride,
            pbr_material,
            normal_matrix,
        );

        // DEBUGGING, JUST SHOW NORMAL AS COLOR
        // textureStore(opaque_tex, coords, vec4<f32>(normal * 0.5 + 0.5, 1.0));
        // return;
    {% else %}
        let normal = vec3<f32>(1.0, 1.0, 1.0);
    {% endif %}



    let texture_lods = pbr_get_mipmap_levels(
        pbr_material,
        coords,
        triangle_index,
        barycentric,
        attribute_indices_offset,
        attribute_data_offset,
        vertex_attribute_stride,
        screen_dims_i32,
        model_transform
    );

    let material_color = pbr_get_material_color(
        triangle_indices,
        attribute_data_offset,
        triangle_index,
        pbr_material,
        barycentric,
        vertex_attribute_stride,
        texture_lods,
        normal
    );

    var color = vec3<f32>(0.0);

    // TODO - lighting
    let n_lights = 2u;
    for(var i = 0u; i < n_lights; i = i + 1u) {
        let light_brdf = light_to_brdf(get_light(i), normal, standard_coordinates.world_position);

        if (light_brdf.n_dot_l > 0.0001) {
            color += brdf(material_color, light_brdf, standard_coordinates.surface_to_camera);
        } else {
            color += unlit(material_color, ambient, standard_coordinates.surface_to_camera);
        }
    }

    //color = unlit(material_color, vec3<f32>(1.0), standard_coordinates.surface_to_camera);

    // Write to output texture
    textureStore(opaque_tex, coords, vec4<f32>(color, material_color.base.a));
}

// Decide whether we have enough UV inputs to evaluate every texture referenced by the material.
// Each branch checks the number of TEXCOORD sets exposed by the mesh (see `attributes.rs`) against
// what the material expects, and returns false when sampling would read garbage data.
fn pbr_should_run(material: PbrMaterial) -> bool {
    {%- match uv_sets %}
        {% when Some with (uv_sets) %}
            return pbr_material_uses_uv_count(material, {{ uv_sets }});
        {% when None %}
            return !pbr_material_has_any_uvs(material);
    {% endmatch %}
}

fn pbr_material_has_any_uvs(material: PbrMaterial) -> bool {
    // When the mesh supplies zero UV sets we can only shade materials that also skip every UV-backed texture.
    return material.has_base_color_texture ||
        material.has_metallic_roughness_texture ||
        material.has_normal_texture ||
        material.has_occlusion_texture ||
        material.has_emissive_texture;
}

fn pbr_material_uses_uv_count(material: PbrMaterial, uv_set_count: u32) -> bool {
    // Validate every texture's UV requirements individually so that a single mismatched binding aborts shading.
    if !texture_fits_uv_budget(material.has_base_color_texture, material.base_color_tex_info, uv_set_count) {
        return false;
    }

    if !texture_fits_uv_budget(material.has_metallic_roughness_texture, material.metallic_roughness_tex_info, uv_set_count) {
        return false;
    }

    if !texture_fits_uv_budget(material.has_normal_texture, material.normal_tex_info, uv_set_count) {
        return false;
    }

    if !texture_fits_uv_budget(material.has_occlusion_texture, material.occlusion_tex_info, uv_set_count) {
        return false;
    }

    if !texture_fits_uv_budget(material.has_emissive_texture, material.emissive_tex_info, uv_set_count) {
        return false;
    }

    return true;
}

fn texture_fits_uv_budget(has_texture: bool, info: TextureInfo, uv_set_count: u32) -> bool {
    if !has_texture {
        return true;
    }

    // Reject textures that reference UV sets the mesh never uploaded.
    return info.attribute_uv_set_index < uv_set_count;
}

fn get_triangle_indices(attribute_indices_offset: u32, triangle_index: u32) -> vec3<u32> {
    let base = attribute_indices_offset + (triangle_index * 3u);
    return vec3<u32>(
        attribute_indices[base],
        attribute_indices[base + 1u],
        attribute_indices[base + 2u],
    );
}
