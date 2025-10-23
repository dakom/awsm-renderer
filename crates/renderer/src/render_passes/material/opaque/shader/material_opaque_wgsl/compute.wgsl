/*************** START color_space.wgsl ******************/
{% include "all_material_shared_wgsl/color_space.wgsl" %}
/*************** END color_space.wgsl ******************/
/*************** START debug.wgsl ******************/
{% include "all_material_shared_wgsl/debug.wgsl" %}
/*************** END debug.wgsl ******************/
/*************** START math.wgsl ******************/
{% include "all_material_shared_wgsl/math.wgsl" %}
/*************** END math.wgsl ******************/
/*************** START mesh_meta.wgsl ******************/
{% include "all_material_shared_wgsl/mesh_meta.wgsl" %}
/*************** END mesh_meta.wgsl ******************/
/*************** START normal.wgsl ******************/
{% include "all_material_shared_wgsl/normal.wgsl" %}
/*************** END normal.wgsl ******************/
/*************** START projection.wgsl ******************/
{% include "all_material_shared_wgsl/projection.wgsl" %}
/*************** END projection.wgsl ******************/
/*************** START textures.wgsl ******************/
{% include "all_material_shared_wgsl/textures.wgsl" %}
/*************** END textures.wgsl ******************/
/*************** START transforms.wgsl ******************/
{% include "all_material_shared_wgsl/transforms.wgsl" %}
/*************** END transforms.wgsl ******************/
/*************** START positions.wgsl ******************/
{% include "all_material_shared_wgsl/positions.wgsl" %}
/*************** END positions.wgsl ******************/
/*************** START lights.wgsl ******************/
{% include "pbr_shared_wgsl/lighting/lights.wgsl" %}
/*************** END lights.wgsl ******************/
/*************** START brdf.wgsl ******************/
{% include "pbr_shared_wgsl/lighting/brdf.wgsl" %}
/*************** END brdf.wgsl ******************/
/*************** START unlit.wgsl ******************/
{% include "pbr_shared_wgsl/lighting/unlit.wgsl" %}
/*************** END unlit.wgsl ******************/
/*************** START material.wgsl ******************/
{% include "pbr_shared_wgsl/material.wgsl" %}
/*************** END material.wgsl ******************/
/*************** START material_color.wgsl ******************/
{% include "pbr_shared_wgsl/material_color.wgsl" %}
/*************** END material_color.wgsl ******************/
/*************** START mipmap.wgsl ******************/
{% include "material_opaque_wgsl/helpers/mipmap.wgsl" %}
/*************** END mipmap.wgsl ******************/
/*************** START standard.wgsl ******************/
{% include "material_opaque_wgsl/helpers/standard.wgsl" %}
/*************** END standard.wgsl ******************/
/*************** START skybox.wgsl ******************/
{% include "material_opaque_wgsl/helpers/skybox.wgsl" %}
/*************** END skybox.wgsl ******************/

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

@group(0) @binding(0) var<storage, read> mesh_metas: array<MeshMeta>;
@group(0) @binding(1) var visibility_data_tex: texture_2d<f32>;
@group(0) @binding(2) var opaque_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(3) var<storage, read> materials: array<PbrMaterialRaw>;
@group(0) @binding(4) var<storage, read> attribute_indices: array<u32>;
@group(0) @binding(5) var<storage, read> attribute_data: array<f32>;
@group(0) @binding(6) var<storage, read> model_transforms: array<mat4x4<f32>>;
@group(0) @binding(7) var<storage, read> normal_matrices: array<f32>;
@group(0) @binding(8) var<uniform> camera: CameraUniform;
@group(0) @binding(9) var skybox_tex: texture_cube<f32>;
@group(0) @binding(10) var skybox_sampler: sampler;
@group(0) @binding(11) var ibl_filtered_env_tex: texture_cube<f32>;
@group(0) @binding(12) var ibl_filtered_env_sampler: sampler;
@group(0) @binding(13) var ibl_irradiance_tex: texture_cube<f32>;
@group(0) @binding(14) var ibl_irradiance_sampler: sampler;
@group(0) @binding(15) var<uniform> ibl_info: IblInfo;
@group(0) @binding(16) var brdf_lut_tex: texture_2d<f32>;
@group(0) @binding(17) var brdf_lut_sampler: sampler;
@group(0) @binding(18) var depth_tex: texture_depth_2d;
@group(0) @binding(19) var<storage, read> visibility_data: array<f32>;
@group(0) @binding(20) var geometry_normal_tex: texture_2d<f32>;
@group(0) @binding(21) var geometry_tangent_tex: texture_2d<f32>;
{% for i in 0..texture_atlas_len %}
    @group(1) @binding({{ i }}u) var atlas_tex_{{ i }}: texture_2d_array<f32>;
{% endfor %}
{% for i in 0..sampler_atlas_len %}
    @group(2) @binding({{ i }}u) var atlas_sampler_{{ i }}: sampler;
{% endfor %}


@compute @workgroup_size(8, 8)
fn main(
    @builtin(global_invocation_id) gid: vec3<u32>
) {
    let coords = vec2<i32>(gid.xy);
    let screen_dims = textureDimensions(opaque_tex);
    let screen_dims_i32 = vec2<i32>(i32(screen_dims.x), i32(screen_dims.y));
    let screen_dims_f32 = vec2<f32>(f32(screen_dims.x), f32(screen_dims.y));
    let pixel_center = vec2<f32>(f32(coords.x) + 0.5, f32(coords.y) + 0.5);

    // Bounds check
    if (coords.x >= screen_dims_i32.x || coords.y >= screen_dims_i32.y) {
        return;
    }

    let visibility_data = textureLoad(visibility_data_tex, coords, 0);

    let triangle_index = bitcast<u32>(visibility_data.x);
    // early return if nothing was drawn at this pixel
    if (triangle_index == F32_MAX) {
        let color = sample_skybox(coords, screen_dims_f32, camera, skybox_tex, skybox_sampler);
        textureStore(opaque_tex, coords, color);
        return;
    }
    let material_meta_offset = bitcast<u32>(visibility_data.y);
    let barycentric = vec3<f32>(visibility_data.z, visibility_data.w, 1.0 - visibility_data.z - visibility_data.w);


    let mesh_meta = mesh_metas[material_meta_offset / META_SIZE_IN_BYTES];


    let material_offset = mesh_meta.material_offset;
    let pbr_material = pbr_get_material(material_offset);

    // Skip work when the mesh doesn't provide enough UV data for the material.
    if !pbr_should_run(pbr_material) {
        return;
    }

    let vertex_attribute_stride = mesh_meta.vertex_attribute_stride / 4; // 4 bytes per float
    let attribute_indices_offset = mesh_meta.vertex_attribute_indices_offset / 4;
    let attribute_data_offset = mesh_meta.vertex_attribute_data_offset / 4;
    let visibility_data_offset = mesh_meta.visibility_data_offset / 4;

    let transforms = get_transforms(mesh_meta);

    let standard_coordinates = get_standard_coordinates(coords, screen_dims);

    // Get the vertex indices for this triangle
    let base_triangle_index = attribute_indices_offset + (triangle_index * 3u);
    let triangle_indices = vec3<u32>(attribute_indices[base_triangle_index], attribute_indices[base_triangle_index + 1], attribute_indices[base_triangle_index + 2]);

    // Load world-space normal directly from geometry pass output (already transformed with morphs/skins)
    let world_normal = textureLoad(geometry_normal_tex, coords, 0).xyz;


    let os_vertices = get_object_space_vertices(visibility_data_offset, triangle_index);

    {% match mipmap %}
        {% when MipmapMode::None %}
            let texture_lods = PbrMaterialMipLevels(
                0.0, // base_color
                0.0, // metallic_roughness
                0.0, // normal
                0.0, // occlusion
                0.0  // emissive
            );

            let material_color = pbr_get_material_color(
                triangle_indices,
                attribute_data_offset,
                triangle_index,
                pbr_material,
                barycentric,
                vertex_attribute_stride,
                texture_lods,
                world_normal,
                transforms.world_normal,
                os_vertices
            );
        {% when MipmapMode::Lod %}
            let projected_vertices = project_vertices(os_vertices, transforms.world_model, screen_dims_f32);

            let mip_cache = build_mip_cache_with_barycentric(
                projected_vertices,
                pixel_center
            );

            let texture_lods = pbr_get_mipmap_levels(
                mip_cache,
                screen_dims_f32,
                pbr_material,
                triangle_indices,
                barycentric,
                attribute_data_offset,
                vertex_attribute_stride,
            );

            let material_color = pbr_get_material_color(
                triangle_indices,
                attribute_data_offset,
                triangle_index,
                pbr_material,
                barycentric,
                vertex_attribute_stride,
                texture_lods,
                world_normal,
                transforms.world_normal,
                os_vertices
            );
    {% endmatch %}

    var color = vec3<f32>(0.0);

    {% if debug.ibl_only %}
        // IBL only - skip direct lighting to isolate the issue
        color = brdf_ibl(
            material_color,
            material_color.normal,
            standard_coordinates.surface_to_camera,
            ibl_filtered_env_tex,
            ibl_filtered_env_sampler,
            ibl_irradiance_tex,
            ibl_irradiance_sampler,
            brdf_lut_tex,
            brdf_lut_sampler,
            ibl_info
        );
    {% else %}
        // Direct lighting: accumulate contributions from all lights
        // Note: Hardcoded to 4 lights (see lights.wgsl for definitions)
        let n_lights = 4u;
        for(var i = 0u; i < n_lights; i = i + 1u) {
            let light_brdf = light_to_brdf(get_light(i), material_color.normal, standard_coordinates.world_position);
            color += brdf_direct(material_color, light_brdf, standard_coordinates.surface_to_camera);
        }

        // Indirect lighting: IBL contribution (includes emissive)
        color += brdf_ibl(
            material_color,
            material_color.normal,
            standard_coordinates.surface_to_camera,
            ibl_filtered_env_tex,
            ibl_filtered_env_sampler,
            ibl_irradiance_tex,
            ibl_irradiance_sampler,
            brdf_lut_tex,
            brdf_lut_sampler,
            ibl_info
        );
    {% endif %}

    {% if debug.mips %}
        let i = i32(floor(texture_lods.base_color + 0.5)); // nearest mip
        let atlas_info = get_atlas_info(pbr_material.base_color_tex_info.atlas_index);
        let max_mip_level = select(15.0, atlas_info.levels_f - 1.0, atlas_info.valid && atlas_info.levels_f > 1.0);
        let level = f32(i) / max_mip_level;
        color = vec3<f32>(level, level, level);
    {% endif %}

    {% if debug.n_dot_v %}
        let n = safe_normalize(material_color.normal);
        let v = safe_normalize(standard_coordinates.surface_to_camera);
        let n_dot_v_val = saturate(dot(n, v));
        // Show n_dot_v as grayscale, but also show it in green channel for visibility
        // R = n_dot_v, G = n_dot_v * 2 for emphasis, B = 0
        color = vec3<f32>(n_dot_v_val, n_dot_v_val * 2.0, 0.0);
    {% endif %}

    {% if debug.normals %}
        // Visualize normals as RGB (map from [-1,1] to [0,1])
        let n = safe_normalize(material_color.normal);
        color = n * 0.5 + 0.5;
    {% endif %}

    {% if debug.solid_color %}
        // Just output bright magenta to verify debug system works
        color = vec3<f32>(1.0, 0.0, 1.0);
    {% endif %}

    {% if debug.view_direction %}
        // Visualize view direction (surface_to_camera) as RGB
        let v = safe_normalize(standard_coordinates.surface_to_camera);
        color = v * 0.5 + 0.5;
    {% endif %}

    {% if debug.irradiance_sample %}
        // Sample the irradiance map directly using the normal
        let n = safe_normalize(material_color.normal);
        let irradiance = textureSampleLevel(ibl_irradiance_tex, ibl_irradiance_sampler, n, 0.0).rgb;
        color = irradiance;
    {% endif %}

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
