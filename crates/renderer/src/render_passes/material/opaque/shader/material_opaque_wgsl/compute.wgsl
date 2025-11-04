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
/*************** START projection.wgsl ******************/
{% include "all_material_shared_wgsl/projection.wgsl" %}
/*************** END projection.wgsl ******************/
/*************** START textures.wgsl ******************/
{% include "all_material_shared_wgsl/textures.wgsl" %}
/*************** END textures.wgsl ******************/
/*************** START vertex_color.wgsl ******************/
{% include "all_material_shared_wgsl/vertex_color.wgsl" %}
/*************** END vertex_color.wgsl ******************/
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
{% if multisampled_geometry %}
/*************** START msaa.wgsl ******************/
{% include "material_opaque_wgsl/helpers/msaa.wgsl" %}
/*************** END msaa.wgsl ******************/
{% endif %}

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
{% if multisampled_geometry %}
    @group(0) @binding(0) var visibility_data_tex: texture_multisampled_2d<u32>;
    @group(0) @binding(1) var barycentric_tex: texture_multisampled_2d<f32>;
    @group(0) @binding(2) var depth_tex: texture_depth_multisampled_2d;
    @group(0) @binding(3) var geometry_normal_tex: texture_multisampled_2d<f32>;
    @group(0) @binding(4) var geometry_tangent_tex: texture_multisampled_2d<f32>;
{% else %}
    @group(0) @binding(0) var visibility_data_tex: texture_2d<u32>;
    @group(0) @binding(1) var barycentric_tex: texture_2d<f32>;
    @group(0) @binding(2) var depth_tex: texture_depth_2d;
    @group(0) @binding(3) var geometry_normal_tex: texture_2d<f32>;
    @group(0) @binding(4) var geometry_tangent_tex: texture_2d<f32>;
{% endif %}
@group(0) @binding(5) var<storage, read> visibility_data: array<f32>;
@group(0) @binding(6) var<storage, read> mesh_metas: array<MeshMeta>;
@group(0) @binding(7) var<storage, read> materials: array<PbrMaterialRaw>;
@group(0) @binding(8) var<storage, read> attribute_indices: array<u32>;
@group(0) @binding(9) var<storage, read> attribute_data: array<f32>;
@group(0) @binding(10) var<storage, read> model_transforms: array<mat4x4<f32>>;
@group(0) @binding(11) var<storage, read> normal_matrices: array<f32>;
@group(0) @binding(12) var<uniform> camera: CameraUniform;
@group(0) @binding(13) var skybox_tex: texture_cube<f32>;
@group(0) @binding(14) var skybox_sampler: sampler;
@group(0) @binding(15) var ibl_filtered_env_tex: texture_cube<f32>;
@group(0) @binding(16) var ibl_filtered_env_sampler: sampler;
@group(0) @binding(17) var ibl_irradiance_tex: texture_cube<f32>;
@group(0) @binding(18) var ibl_irradiance_sampler: sampler;
@group(0) @binding(19) var brdf_lut_tex: texture_2d<f32>;
@group(0) @binding(20) var brdf_lut_sampler: sampler;
@group(0) @binding(21) var opaque_tex: texture_storage_2d<rgba16float, write>;

@group(1) @binding(0) var<uniform> lights_info: LightsInfoPacked;
@group(1) @binding(1) var<storage, read> lights: array<LightPacked>;

{% for i in 0..texture_atlas_len %}
    @group(2) @binding({{ i }}u) var atlas_tex_{{ i }}: texture_2d_array<f32>;
{% endfor %}
{% for i in 0..sampler_atlas_len %}
    @group(3) @binding({{ i }}u) var atlas_sampler_{{ i }}: sampler;
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

    let triangle_index = join32(visibility_data.x, visibility_data.y);
    let material_meta_offset = join32(visibility_data.z, visibility_data.w);

    // early return if nothing was drawn at this pixel (only if no MSAA, otherwise check all samples)
    {% if multisampled_geometry %}
        // With MSAA, check if ANY sample hit geometry before early returning
        // Using short-circuit OR for efficiency (stops checking once a hit is found)
        {% for s in 0..msaa_sample_count %}let vis_check_{{s}} = textureLoad(visibility_data_tex, coords, {{s}});
        {% endfor %}let any_sample_hit = {% for s in 0..msaa_sample_count %}join32(vis_check_{{s}}.x, vis_check_{{s}}.y) != U32_MAX{% if loop.last %}{% else %} || {% endif %}{% endfor %};

        if (!any_sample_hit) {
            // All samples are skybox - just render skybox
            let color = sample_skybox(coords, screen_dims_f32, camera, skybox_tex, skybox_sampler);
            textureStore(opaque_tex, coords, color);
            return;
        }
    {% else %}
        if (triangle_index == U32_MAX) {
            let color = sample_skybox(coords, screen_dims_f32, camera, skybox_tex, skybox_sampler);
            textureStore(opaque_tex, coords, color);
            return;
        }
    {% endif %}
    // Special case: Sample 0 is skybox but other samples might have geometry
    // This handles silhouette edges where background is visible at sample 0
    // We must handle this separately because the main path assumes sample 0 has valid geometry data
    {% if multisampled_geometry %}
        if (triangle_index == U32_MAX) {
            // Process all samples with full per-sample shading and blend
            // (This path was triggered by any_sample_hit check above, so we know at least one sample has geometry)
            var color_sum = vec3<f32>(0.0);
            var alpha_sum = 0.0;
            var valid_samples = 0u;

            let lights_info = get_lights_info();
            let standard_coordinates = get_standard_coordinates(coords, screen_dims);
            // Use LOD 0 for all samples (highest detail) - conservative for silhouette edges
            let zero_lods = PbrMaterialMipLevels(0.0, 0.0, 0.0, 0.0, 0.0);

            {% for s in 0..msaa_sample_count %}
                let vis_{{s}} = textureLoad(visibility_data_tex, coords, {{s}});
                let tri_{{s}} = join32(vis_{{s}}.x, vis_{{s}}.y);
                let mat_meta_{{s}} = join32(vis_{{s}}.z, vis_{{s}}.w);

                if (tri_{{s}} == U32_MAX) {
                    valid_samples++;
                    let skybox_col = sample_skybox(coords, screen_dims_f32, camera, skybox_tex, skybox_sampler);
                    color_sum += skybox_col.rgb;
                    alpha_sum += skybox_col.a;
                } else {
                    // Full per-sample geometry shading
                    let mesh_meta_{{s}} = mesh_metas[mat_meta_{{s}} / META_SIZE_IN_BYTES];
                    let material_offset_{{s}} = mesh_meta_{{s}}.material_offset;
                    let pbr_material_{{s}} = pbr_get_material(material_offset_{{s}});

                    if (pbr_should_run(pbr_material_{{s}})) {
                        valid_samples++;

                        let vertex_attribute_stride_{{s}} = mesh_meta_{{s}}.vertex_attribute_stride / 4;
                        let attribute_indices_offset_{{s}} = mesh_meta_{{s}}.vertex_attribute_indices_offset / 4;
                        let attribute_data_offset_{{s}} = mesh_meta_{{s}}.vertex_attribute_data_offset / 4;
                        let visibility_data_offset_{{s}} = mesh_meta_{{s}}.visibility_data_offset / 4;

                        let base_tri_idx_{{s}} = attribute_indices_offset_{{s}} + (tri_{{s}} * 3u);
                        let tri_indices_{{s}} = vec3<u32>(
                            attribute_indices[base_tri_idx_{{s}}],
                            attribute_indices[base_tri_idx_{{s}} + 1],
                            attribute_indices[base_tri_idx_{{s}} + 2]
                        );

                        let bary_{{s}} = textureLoad(barycentric_tex, coords, {{s}});
                        let barycentric_{{s}} = vec3<f32>(bary_{{s}}.x, bary_{{s}}.y, 1.0 - bary_{{s}}.x - bary_{{s}}.y);
                        let normal_{{s}} = textureLoad(geometry_normal_tex, coords, {{s}}).xyz;
                        let os_verts_{{s}} = get_object_space_vertices(visibility_data_offset_{{s}}, tri_{{s}});
                        let transforms_{{s}} = get_transforms(mesh_meta_{{s}});

                        let mat_color_{{s}} = pbr_get_material_color(
                            tri_indices_{{s}},
                            attribute_data_offset_{{s}},
                            tri_{{s}},
                            pbr_material_{{s}},
                            barycentric_{{s}},
                            vertex_attribute_stride_{{s}},
                            zero_lods,
                            normal_{{s}},
                            transforms_{{s}}.world_normal,
                            os_verts_{{s}}
                        );

                        var sample_color = vec3<f32>(0.0);

                        {% match debug.lighting %}
                            {% when ShaderTemplateMaterialOpaqueDebugLighting::None | ShaderTemplateMaterialOpaqueDebugLighting::IblOnly %}
                                sample_color = brdf_ibl(
                                    mat_color_{{s}},
                                    mat_color_{{s}}.normal,
                                    standard_coordinates.surface_to_camera,
                                    ibl_filtered_env_tex,
                                    ibl_filtered_env_sampler,
                                    ibl_irradiance_tex,
                                    ibl_irradiance_sampler,
                                    brdf_lut_tex,
                                    brdf_lut_sampler,
                                    lights_info.ibl
                                );
                            {% when _ %}
                        {% endmatch %}

                        {% match debug.lighting %}
                            {% when ShaderTemplateMaterialOpaqueDebugLighting::None | ShaderTemplateMaterialOpaqueDebugLighting::PunctualOnly %}
                                for(var i = 0u; i < lights_info.n_lights; i = i + 1u) {
                                    let light_brdf = light_to_brdf(get_light(i), mat_color_{{s}}.normal, standard_coordinates.world_position);
                                    sample_color += brdf_direct(mat_color_{{s}}, light_brdf, standard_coordinates.surface_to_camera);
                                }
                            {% when _ %}
                        {% endmatch %}

                        color_sum += sample_color;
                        alpha_sum += mat_color_{{s}}.base.a;
                    }
                }
            {% endfor %}

            if (valid_samples > 0u) {
                textureStore(opaque_tex, coords, vec4<f32>(color_sum / f32(valid_samples), alpha_sum / f32(valid_samples)));
            } else {
                textureStore(opaque_tex, coords, sample_skybox(coords, screen_dims_f32, camera, skybox_tex, skybox_sampler));
            }
            return;
        }
    {% endif %}

    let barycentric_data = textureLoad(barycentric_tex, coords, 0);
    let barycentric = vec3<f32>(barycentric_data.x, barycentric_data.y, 1.0 - barycentric_data.x - barycentric_data.y);


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

    let lights_info = get_lights_info();

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

    {% match debug.lighting %}
        {% when ShaderTemplateMaterialOpaqueDebugLighting::None | ShaderTemplateMaterialOpaqueDebugLighting::IblOnly %}
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
                lights_info.ibl
            );
        {% when _ %}
    {% endmatch %}

    {% match debug.lighting %}
        {% when ShaderTemplateMaterialOpaqueDebugLighting::None | ShaderTemplateMaterialOpaqueDebugLighting::PunctualOnly %}
            // Punctual lighting: accumulate contributions from all lights
            for(var i = 0u; i < lights_info.n_lights; i = i + 1u) {
                let light_brdf = light_to_brdf(get_light(i), material_color.normal, standard_coordinates.world_position);
                color += brdf_direct(material_color, light_brdf, standard_coordinates.surface_to_camera);
            }
        {% when _ %}
    {% endmatch %}

    {% match debug.lighting %}
        {% when ShaderTemplateMaterialOpaqueDebugLighting::HardcodedPunctualOnly %}
            for(var i = 0u; i < lights_info.n_lights; i = i + 1u) {
                var light: Light;
                switch(i) {
                    case 0u: {
                        light = Light(
                            1u, // Directional
                            vec3<f32>(1.0, 1.0, 1.0), // color
                            1.0, // intensity
                            vec3<f32>(0.0, 0.0, 0.0), // position
                            0.0, // range
                            vec3<f32>(-1.0, -0.5, -0.1), // direction
                            0.0, // inner_cone
                            0.0  // outer_cone
                        );
                    }
                    default: {
                        // no light
                        light = Light(0u, vec3<f32>(0.0), 0.0, vec3<f32>(0.0), 0.0, vec3<f32>(0.0), 0.0, 0.0);
                    }
                }
                let light_brdf = light_to_brdf(light, material_color.normal, standard_coordinates.world_position);
                color += brdf_direct(material_color, light_brdf, standard_coordinates.surface_to_camera);
            }
        {% when _ %}
    {% endmatch %}

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

    // MSAA Resolve: if this is an edge pixel, sample all MSAA samples and blend
    {% if multisampled_geometry && !debug.msaa_detect_edges %}
        let samples_to_process = msaa_sample_count_for_pixel(coords, pixel_center, screen_dims_f32, world_normal, triangle_index);

        if (samples_to_process > 1u) {
            // Edge pixel - resolve MSAA by averaging all samples
            // NOTE: Each sample can be on a DIFFERENT triangle/mesh/material!
            var color_sum = vec3<f32>(0.0);
            var alpha_sum = 0.0;
            var valid_samples = 0u;

            // Process all MSAA samples (unrolled via template)
            {% for s in 0..msaa_sample_count %}
                let visibility_{{s}} = textureLoad(visibility_data_tex, coords, {{s}});
                let tri_id_{{s}} = join32(visibility_{{s}}.x, visibility_{{s}}.y);
                let material_meta_offset_{{s}} = join32(visibility_{{s}}.z, visibility_{{s}}.w);

                if (tri_id_{{s}} == U32_MAX) {
                    // Sample hit background - use skybox
                    // Note: skybox is at infinity, so sub-pixel sample position doesn't matter much
                    // All samples use pixel center, which is fine for distant skybox
                    valid_samples++;
                    let skybox_color = sample_skybox(coords, screen_dims_f32, camera, skybox_tex, skybox_sampler);
                    color_sum += skybox_color.rgb;
                    alpha_sum += skybox_color.a;
                } else {
                    // Each sample needs its own mesh/material data
                    let mesh_meta_{{s}} = mesh_metas[material_meta_offset_{{s}} / META_SIZE_IN_BYTES];
                    let material_offset_{{s}} = mesh_meta_{{s}}.material_offset;
                    let pbr_material_{{s}} = pbr_get_material(material_offset_{{s}});

                    // Only process if this sample's mesh provides required data
                    if (pbr_should_run(pbr_material_{{s}})) {
                        valid_samples++;

                        // Per-sample mesh data
                        let vertex_attribute_stride_{{s}} = mesh_meta_{{s}}.vertex_attribute_stride / 4;
                        let attribute_indices_offset_{{s}} = mesh_meta_{{s}}.vertex_attribute_indices_offset / 4;
                        let attribute_data_offset_{{s}} = mesh_meta_{{s}}.vertex_attribute_data_offset / 4;
                        let visibility_data_offset_{{s}} = mesh_meta_{{s}}.visibility_data_offset / 4;

                        // Per-sample triangle indices
                        let base_triangle_index_{{s}} = attribute_indices_offset_{{s}} + (tri_id_{{s}} * 3u);
                        let triangle_indices_{{s}} = vec3<u32>(
                            attribute_indices[base_triangle_index_{{s}}],
                            attribute_indices[base_triangle_index_{{s}} + 1],
                            attribute_indices[base_triangle_index_{{s}} + 2]
                        );

                        // Per-sample geometry
                        let bary_{{s}} = textureLoad(barycentric_tex, coords, {{s}});
                        let barycentric_{{s}} = vec3<f32>(bary_{{s}}.x, bary_{{s}}.y, 1.0 - bary_{{s}}.x - bary_{{s}}.y);
                        let normal_{{s}} = textureLoad(geometry_normal_tex, coords, {{s}}).xyz;
                        let os_vertices_{{s}} = get_object_space_vertices(visibility_data_offset_{{s}}, tri_id_{{s}});
                        let transforms_{{s}} = get_transforms(mesh_meta_{{s}});

                        // Compute material color for this sample with its own data
                        // OPTIMIZATION: Reuse texture_lods from sample 0 instead of computing per-sample
                        // This is acceptable because MSAA samples are sub-pixel, so LOD difference is negligible
                        // Computing per-sample LODs would require projecting vertices for each sample (expensive!)
                        let material_color_{{s}} = pbr_get_material_color(
                            triangle_indices_{{s}},
                            attribute_data_offset_{{s}},
                            tri_id_{{s}},
                            pbr_material_{{s}},
                            barycentric_{{s}},
                            vertex_attribute_stride_{{s}},
                            texture_lods,  // Reuse from sample 0
                            normal_{{s}},
                            transforms_{{s}}.world_normal,
                            os_vertices_{{s}}
                        );

                        // Compute lighting for this sample
                        var sample_color = vec3<f32>(0.0);

                        {% match debug.lighting %}
                            {% when ShaderTemplateMaterialOpaqueDebugLighting::None | ShaderTemplateMaterialOpaqueDebugLighting::IblOnly %}
                                sample_color = brdf_ibl(
                                    material_color_{{s}},
                                    material_color_{{s}}.normal,
                                    standard_coordinates.surface_to_camera,
                                    ibl_filtered_env_tex,
                                    ibl_filtered_env_sampler,
                                    ibl_irradiance_tex,
                                    ibl_irradiance_sampler,
                                    brdf_lut_tex,
                                    brdf_lut_sampler,
                                    lights_info.ibl
                                );
                            {% when _ %}
                        {% endmatch %}

                        {% match debug.lighting %}
                            {% when ShaderTemplateMaterialOpaqueDebugLighting::None | ShaderTemplateMaterialOpaqueDebugLighting::PunctualOnly %}
                                for(var i = 0u; i < lights_info.n_lights; i = i + 1u) {
                                    let light_brdf = light_to_brdf(get_light(i), material_color_{{s}}.normal, standard_coordinates.world_position);
                                    sample_color += brdf_direct(material_color_{{s}}, light_brdf, standard_coordinates.surface_to_camera);
                                }
                            {% when _ %}
                        {% endmatch %}

                        color_sum += sample_color;
                        alpha_sum += material_color_{{s}}.base.a;
                    }
                }
            {% endfor %}

            // Average the results
            if (valid_samples > 0u) {
                color = color_sum / f32(valid_samples);
                let avg_alpha = alpha_sum / f32(valid_samples);
                textureStore(opaque_tex, coords, vec4<f32>(color, avg_alpha));
                return;
            }
        }
    {% endif %}

    {% if multisampled_geometry && debug.msaa_detect_edges %}
        // Debug visualization: show detected edges in magenta
        if (depth_edge_mask(coords, pixel_center, screen_dims_f32, world_normal, triangle_index)) {
            textureStore(opaque_tex, coords, vec4<f32>(1.0, 0.0, 1.0, 1.0));
            return;
        }
    {% endif %}

    // Write to output texture (non-edge path or non-MSAA)
    textureStore(opaque_tex, coords, vec4<f32>(color, material_color.base.a));
}

fn pbr_should_run(material: PbrMaterial) -> bool {
    return pbr_should_run_uvs(material) && pbr_should_run_colors(material);
}

fn pbr_should_run_colors(material: PbrMaterial) -> bool {
    {%- match color_sets %}
        {% when Some with (color_sets) %}
            return material.has_color_info == true && material.color_info.set_index < {{ color_sets }};
        {% when None %}
            return material.has_color_info == false;
    {% endmatch %}
}

// Decide whether we have enough UV inputs to evaluate every texture referenced by the material.
// Each branch checks the number of TEXCOORD sets exposed by the mesh (see `attributes.rs`) against
// what the material expects, and returns false when sampling would read garbage data.
fn pbr_should_run_uvs(material: PbrMaterial) -> bool {
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
