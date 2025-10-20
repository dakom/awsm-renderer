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
@group(0) @binding(3) var<storage, read> materials: array<PbrMaterialRaw>; // TODO - just raw data, derive PbrMaterialRaw if that's what we have?
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
@group(0) @binding(15) var depth_tex: texture_depth_2d;
@group(0) @binding(16) var<storage, read> visibility_data: array<f32>;
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

    // get the vertex indices for this triangle
    let base_triangle_index = attribute_indices_offset + (triangle_index * 3u);
    let triangle_indices = vec3<u32>(attribute_indices[base_triangle_index], attribute_indices[base_triangle_index + 1], attribute_indices[base_triangle_index + 2]);

   // ============================================================================
    // DEBUG SECTION: Uncomment ONE of these blocks to visualize different data
    // ============================================================================

    // DEBUG 1: World position as color (normalized to [0,1])
    // Shows the reconstructed world_position from get_standard_coordinates()
    // let wp = standard_coordinates.world_position;
    // let debug_color = normalize(wp) * 0.5 + 0.5;
    // textureStore(opaque_tex, coords, vec4<f32>(debug_color, 1.0));
    // return;

    // DEBUG 2: World position X, Y, Z components separately
    // R = X, G = Y, B = Z (scaled and biased to visible range)
    // let wp = standard_coordinates.world_position;
    // let scale = 0.1; // adjust this to see smaller/larger positions
    // let debug_xyz = wp * scale * 0.5 + 0.5;
    // textureStore(opaque_tex, coords, vec4<f32>(debug_xyz, 1.0));
    // return;

    // DEBUG 3: View space depth visualization
    // let view_h        = camera.inv_proj * vec4(standard_coordinates.ndc, 1.0);
    // let view_position = view_h.xyz / max(view_h.w, 1e-8);
    // let z = clamp((-view_position.z) / 10.0, 0.0, 1.0); // 10m clip for viz
    // textureStore(opaque_tex, coords, vec4(z, z, z, 1.0));
    // return;

    // DEBUG 4: Raw depth buffer visualization
    // let d  = standard_coordinates.depth_sample;
    // textureStore(opaque_tex, coords, vec4(d, d, d, 1.0));
    // return;

    // DEBUG 5: Distance from camera to world_position
    // Visualizes how far each pixel is from camera.position
    // let dist_to_camera = length(camera.position - standard_coordinates.world_position);
    // let normalized_dist = clamp(dist_to_camera / 10.0, 0.0, 1.0); // 10m range
    // textureStore(opaque_tex, coords, vec4(normalized_dist, normalized_dist, normalized_dist, 1.0));
    // return;

    // DEBUG 6: Compare camera.position with world_position relationship
    // Shows direction vector from camera to surface
    // let to_surface = normalize(standard_coordinates.world_position - camera.position);
    // let debug_dir = to_surface * 0.5 + 0.5;
    // textureStore(opaque_tex, coords, vec4<f32>(debug_dir, 1.0));
    // return;

    // DEBUG 7: Point light distance visualization
    // Shows distance from the point light (at 2.5, 3.0, 2.0) to world_position
    // Red = close to light, Black = far from light
    // let point_light_pos = vec3<f32>(2.5, 3.0, 2.0);
    // let dist_to_light = length(point_light_pos - standard_coordinates.world_position);
    // let normalized_light_dist = clamp(1.0 - (dist_to_light / 10.0), 0.0, 1.0);
    // textureStore(opaque_tex, coords, vec4(normalized_light_dist, 0.0, 0.0, 1.0));
    // return;

    // DEBUG 8: Show surface_to_camera direction
    // Visualizes the view direction (should match camera orientation)
    // let debug_view = standard_coordinates.surface_to_camera * 0.5 + 0.5;
    // textureStore(opaque_tex, coords, vec4<f32>(debug_view, 1.0));
    // return;

    // DEBUG 9: Compare reconstructed world_position vs actual vertex world positions
    // This is THE KEY DEBUG - shows if world_position matches the actual geometry
    // Get the actual object-space vertices and transform them to world space
    // let debug_os_vertices = get_object_space_vertices(visibility_data_offset, triangle_index);
    // let v0_world = (transforms.world_model * vec4<f32>(debug_os_vertices.p0, 1.0)).xyz;
    // let v1_world = (transforms.world_model * vec4<f32>(debug_os_vertices.p1, 1.0)).xyz;
    // let v2_world = (transforms.world_model * vec4<f32>(debug_os_vertices.p2, 1.0)).xyz;
    // let actual_world_pos = barycentric.x * v0_world + barycentric.y * v1_world + barycentric.z * v2_world;
    //
    // // Compare: reconstructed vs actual
    // let diff = standard_coordinates.world_position - actual_world_pos;
    // let error_magnitude = length(diff);
    //
    // // Visualize error: white = no error, red = large error
    // let error_vis = clamp(error_magnitude * 10.0, 0.0, 1.0); // scale for visibility
    // textureStore(opaque_tex, coords, vec4(error_vis, 1.0 - error_vis, 1.0 - error_vis, 1.0));
    // return;

    // DEBUG 10: Show actual vertex world positions (for comparison with DEBUG 1/2)
    // let os_vertices = get_object_space_vertices(visibility_data_offset, triangle_index);
    // let v0_world = (transforms.world_model * vec4<f32>(os_vertices.p0, 1.0)).xyz;
    // let v1_world = (transforms.world_model * vec4<f32>(os_vertices.p1, 1.0)).xyz;
    // let v2_world = (transforms.world_model * vec4<f32>(os_vertices.p2, 1.0)).xyz;
    // let actual_world_pos = barycentric.x * v0_world + barycentric.y * v1_world + barycentric.z * v2_world;
    // let debug_actual = normalize(actual_world_pos) * 0.5 + 0.5;
    // textureStore(opaque_tex, coords, vec4<f32>(debug_actual, 1.0));
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

    // Add lighting from each light
    // Note: BRDF includes IBL which gets added 4 times (once per light)
    // We've reduced the IBL intensity in brdf.wgsl to compensate
    let n_lights = 4u;
    for(var i = 0u; i < n_lights; i = i + 1u) {
        let light_brdf = light_to_brdf(get_light(i), world_normal, standard_coordinates.world_position);

        // Always add BRDF - it naturally returns zero when n_dot_l is zero
        // This avoids any discontinuity from if statements
        color += brdf(material_color, light_brdf, standard_coordinates.surface_to_camera);
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
