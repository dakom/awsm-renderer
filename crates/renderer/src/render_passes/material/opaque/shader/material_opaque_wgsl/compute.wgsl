{% include "all_material_shared_wgsl/color_space.wgsl" %}
{% include "all_material_shared_wgsl/debug.wgsl" %}
{% include "all_material_shared_wgsl/math.wgsl" %}
{% include "all_material_shared_wgsl/mesh_meta.wgsl" %}
{% include "all_material_shared_wgsl/normal.wgsl" %}
{% include "all_material_shared_wgsl/projection.wgsl" %}
{% include "all_material_shared_wgsl/textures.wgsl" %}
{% include "all_material_shared_wgsl/transforms.wgsl" %}
{% include "all_material_shared_wgsl/positions.wgsl" %}
{% include "pbr_shared_wgsl/lighting/lights.wgsl" %}
{% include "pbr_shared_wgsl/lighting/brdf.wgsl" %}
{% include "pbr_shared_wgsl/lighting/unlit.wgsl" %}
{% include "pbr_shared_wgsl/material.wgsl" %}
{% include "pbr_shared_wgsl/material_color.wgsl" %}
{% include "material_opaque_wgsl/helpers/mipmap.wgsl" %}
{% include "material_opaque_wgsl/helpers/standard.wgsl" %}

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
@group(0) @binding(3) var<storage, read> materials: array<PbrMaterialRaw>; // TODO - just raw data, derive PbrMaterialRaw if that's what we have?
@group(0) @binding(4) var<storage, read> attribute_indices: array<u32>;
@group(0) @binding(5) var<storage, read> attribute_data: array<f32>;
@group(0) @binding(6) var<storage, read> model_transforms: array<mat4x4<f32>>;
@group(0) @binding(7) var<storage, read> normal_matrices: array<f32>;
@group(0) @binding(8) var<uniform> camera: CameraUniform;
@group(0) @binding(9) var depth_tex: texture_depth_2d;
@group(0) @binding(10) var<storage, read> visibility_data: array<f32>;
{% for b in texture_bindings %}
    @group({{ b.group }}u) @binding({{ b.binding }}u) var atlas_tex_{{ b.atlas_index }}: texture_2d_array<f32>;
{% endfor %}
{% for s in sampler_bindings %}
    @group({{ s.group }}u) @binding({{ s.binding }}u) var atlas_sampler_{{ s.sampler_index }}: sampler;
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

    // get the vertex indices for this triangle
    let base_triangle_index = attribute_indices_offset + (triangle_index * 3u);
    let triangle_indices = vec3<u32>(attribute_indices[base_triangle_index], attribute_indices[base_triangle_index + 1], attribute_indices[base_triangle_index + 2]);

   // DEBUGGING
    // let wp = standard_coordinates.world_position;
    // let debug_color = normalize(wp) * 0.5 + 0.5;

    // // let view_h        = camera.inv_proj * vec4(standard_coordinates.ndc, 1.0);
    // // let view_position = view_h.xyz / max(view_h.w, 1e-8);
    // // let z = clamp((-view_position.z) / 10.0, 0.0, 1.0); // 10m clip for viz
    // // textureStore(opaque_tex, coords, vec4(z, z, z, 1.0));

    // let d  = standard_coordinates.depth_sample;     // sample from depth_tex
    // textureStore(opaque_tex, coords, vec4(d, d, d, 1.0));
    // // let d1 = 1.0 - d;

    // // textureStore(opaque_tex, coords,
    // //   // R = raw depth, G = 1 - depth, B = view-space-Z probe (optional)
    // //   vec4<f32>(d, d1, clamp((-standard_coordinates.view_position.z) / 10.0, 0.0, 1.0), 1.0));

    // //textureStore(opaque_tex, coords, vec4<f32>(debug_color, 1.0));
    // return;

    {% if normals %}
        let world_normal = get_world_normal(
            triangle_indices,
            barycentric,
            attribute_data_offset,
            vertex_attribute_stride,
            pbr_material,
            transforms.world_normal,
        );

        // DEBUGGING, JUST SHOW NORMAL AS COLOR
        // textureStore(opaque_tex, coords, vec4<f32>(world_normal * 0.5 + 0.5, 1.0));
        // return;
    {% else %}
        let world_normal = vec3<f32>(1.0, 1.0, 1.0);
    {% endif %}


    // --- A/B switch --------------------------------------------------------------

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
                world_normal
            );
        {% when MipmapMode::Gradient %}
            let os_vertices = get_object_space_vertices(visibility_data_offset, triangle_index);
            let projected_vertices = project_vertices(os_vertices, transforms.world_model, screen_dims_f32);

            // Build cache once per pixel (same inputs you pass into pbr_get_mipmap_levels)
            let mip_cache = build_mip_cache_with_barycentric(
                projected_vertices,
                pixel_center
            );

            let material_color = pbr_get_material_color_with_grads(
                triangle_indices,
                attribute_data_offset,
                pbr_material,
                barycentric,
                vertex_attribute_stride,
                mip_cache,
                world_normal
            );
        {% when MipmapMode::Lod %}
            let os_vertices = get_object_space_vertices(visibility_data_offset, triangle_index);
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

            // TEMP DEBUG: Visualize mip levels
            // let lod = texture_lods.base_color;
            // textureStore(opaque_tex, coords, vec4<f32>(lod/8.0, lod/8.0, lod/8.0, 1.0));
            // return;

            let material_color = pbr_get_material_color(
                triangle_indices,
                attribute_data_offset,
                triangle_index,
                pbr_material,
                barycentric,
                vertex_attribute_stride,
                texture_lods,
                world_normal
            );
    {% endmatch %}


    var color = vec3<f32>(0.0);

    // TODO - lighting
    let n_lights = 2u;
    for(var i = 0u; i < n_lights; i = i + 1u) {
        let light_brdf = light_to_brdf(get_light(i), world_normal, standard_coordinates.world_position);

        if (light_brdf.n_dot_l > 0.0001) {
            color += brdf(material_color, light_brdf, standard_coordinates.surface_to_camera);
        } else {
            color += unlit(material_color);
        }
    }

    //color = unlit(material_color);

    {% if debug.mips %}
        let i = i32(floor(texture_lods.base_color + 0.5)); // nearest mip
        let atlas_info = get_atlas_info(pbr_material.base_color_tex_info.atlas_index);
        let max_mip_level = select(15.0, atlas_info.levels_f - 1.0, atlas_info.valid && atlas_info.levels_f > 1.0);
        let level = f32(i) / max_mip_level;
        color = vec3<f32>(level, level, level);
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
