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

{% match mipmap %}
    {% when MipmapMode::Gradient %}
/*************** START mipmap.wgsl ******************/
{% include "material_opaque_wgsl/helpers/mipmap.wgsl" %}
/*************** END mipmap.wgsl ******************/
    {% when _ %}
{% endmatch %}

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

{% if debug.any() %}
/*************** START debug.wgsl ******************/
{% include "material_opaque_wgsl/helpers/debug.wgsl" %}
/*************** END debug.wgsl ******************/
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
    @group(0) @binding(3) var normal_tangent_tex: texture_multisampled_2d<f32>;
    @group(0) @binding(4) var barycentric_derivatives_tex: texture_multisampled_2d<f32>;
{% else %}
    @group(0) @binding(0) var visibility_data_tex: texture_2d<u32>;
    @group(0) @binding(1) var barycentric_tex: texture_2d<f32>;
    @group(0) @binding(2) var depth_tex: texture_depth_2d;
    @group(0) @binding(3) var normal_tangent_tex: texture_2d<f32>;
    @group(0) @binding(4) var barycentric_derivatives_tex: texture_2d<f32>;
{% endif %}
@group(0) @binding(5) var<storage, read> visibility_data: array<f32>;
@group(0) @binding(6) var<storage, read> mesh_metas: array<MeshMeta>;
@group(0) @binding(7) var<storage, read> materials: array<PbrMaterialRaw>;
@group(0) @binding(8) var<storage, read> attribute_indices: array<u32>;
@group(0) @binding(9) var<storage, read> attribute_data: array<f32>;
@group(0) @binding(10) var<storage, read> model_transforms: array<mat4x4<f32>>;
@group(0) @binding(11) var<storage, read> normal_matrices: array<f32>;
@group(0) @binding(12) var<storage, read> texture_transforms: array<TextureTransform>;
@group(0) @binding(13) var<uniform> camera: CameraUniform;
@group(0) @binding(14) var skybox_tex: texture_cube<f32>;
@group(0) @binding(15) var skybox_sampler: sampler;
@group(0) @binding(16) var ibl_filtered_env_tex: texture_cube<f32>;
@group(0) @binding(17) var ibl_filtered_env_sampler: sampler;
@group(0) @binding(18) var ibl_irradiance_tex: texture_cube<f32>;
@group(0) @binding(19) var ibl_irradiance_sampler: sampler;
@group(0) @binding(20) var brdf_lut_tex: texture_2d<f32>;
@group(0) @binding(21) var brdf_lut_sampler: sampler;
@group(0) @binding(22) var opaque_tex: texture_storage_2d<rgba16float, write>;

@group(1) @binding(0) var<uniform> lights_info: LightsInfoPacked;
@group(1) @binding(1) var<storage, read> lights: array<LightPacked>;

{% for i in 0..texture_pool_arrays_len %}
    @group(2) @binding({{ i }}u) var pool_tex_{{ i }}: texture_2d_array<f32>;
{% endfor %}
{% for i in 0..texture_pool_samplers_len %}
    @group(3) @binding({{ i }}u) var pool_sampler_{{ i }}: sampler;
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
        {% for s in 0..msaa_sample_count %}
            let vis_check_{{s}} = textureLoad(visibility_data_tex, coords, {{s}});
        {% endfor %}

        let any_sample_hit =
        {% for s in 0..msaa_sample_count %}
            join32(vis_check_{{s}}.x, vis_check_{{s}}.y) != U32_MAX
            {% if loop.last %}
            {% else %}
                ||
            {% endif %}
        {% endfor %};

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

                    // Check if this shader variant matches this sample's mesh attributes
                    var sample_matches_variant = true;
                    {%- match uv_sets %}
                        {% when Some with (variant_uv_sets) %}
                            if (mesh_meta_{{s}}.uv_set_count != {{ variant_uv_sets }}u) {
                                sample_matches_variant = false;
                            }
                        {% when None %}
                            if (mesh_meta_{{s}}.uv_set_count != 0u) {
                                sample_matches_variant = false;
                            }
                    {% endmatch %}
                    {%- match color_sets %}
                        {% when Some with (variant_color_sets) %}
                            if (mesh_meta_{{s}}.color_set_count != {{ variant_color_sets }}u) {
                                sample_matches_variant = false;
                            }
                        {% when None %}
                            if (mesh_meta_{{s}}.color_set_count != 0u) {
                                sample_matches_variant = false;
                            }
                    {% endmatch %}

                    if (sample_matches_variant) {
                        valid_samples++;
                        let material_offset_{{s}} = mesh_meta_{{s}}.material_offset;
                        let pbr_material_{{s}} = pbr_get_material(material_offset_{{s}});

                    let vertex_attribute_stride_{{s}} = mesh_meta_{{s}}.vertex_attribute_stride / 4;
                    let attribute_indices_offset_{{s}} = mesh_meta_{{s}}.vertex_attribute_indices_offset / 4;
                    let attribute_data_offset_{{s}} = mesh_meta_{{s}}.vertex_attribute_data_offset / 4;
                    let visibility_data_offset_{{s}} = mesh_meta_{{s}}.visibility_data_offset / 4;
                    let uv_sets_index_{{s}} = mesh_meta_{{s}}.uv_sets_index;

                    let base_tri_idx_{{s}} = attribute_indices_offset_{{s}} + (tri_{{s}} * 3u);
                    let tri_indices_{{s}} = vec3<u32>(
                        attribute_indices[base_tri_idx_{{s}}],
                        attribute_indices[base_tri_idx_{{s}} + 1],
                        attribute_indices[base_tri_idx_{{s}} + 2]
                    );

                    let bary_{{s}} = textureLoad(barycentric_tex, coords, {{s}});
                    let bary_derivs_{{s}} = textureLoad(barycentric_derivatives_tex, coords, {{s}});
                    let barycentric_{{s}} = vec3<f32>(bary_{{s}}.x, bary_{{s}}.y, 1.0 - bary_{{s}}.x - bary_{{s}}.y);
                    let packed_nt_{{s}} = textureLoad(normal_tangent_tex, coords, {{s}});
                    let tbn_{{s}} = unpack_normal_tangent(packed_nt_{{s}});
                    let normal_{{s}} = tbn_{{s}}.N;
                    let os_verts_{{s}} = get_object_space_vertices(visibility_data_offset_{{s}}, tri_{{s}});
                    let transforms_{{s}} = get_transforms(mesh_meta_{{s}});

                    {% match mipmap %}
                        {% when MipmapMode::Gradient %}
                            // Calculate proper gradients for this MSAA sample to enable mipmapping
                            let gradients_{{s}} = pbr_get_gradients(
                                barycentric_{{s}},
                                bary_derivs_{{s}},
                                pbr_material_{{s}},
                                tri_indices_{{s}},
                                attribute_data_offset_{{s}},
                                vertex_attribute_stride_{{s}},
                                uv_sets_index_{{s}},
                                normal_{{s}},
                                camera.view
                            );

                            // Compute material color with proper mipmapping
                            let mat_color_{{s}} = pbr_get_material_color_grad(
                                tri_indices_{{s}},
                                attribute_data_offset_{{s}},
                                tri_{{s}},
                                pbr_material_{{s}},
                                barycentric_{{s}},
                                vertex_attribute_stride_{{s}},
                                uv_sets_index_{{s}},
                                gradients_{{s}},
                                normal_{{s}},
                                transforms_{{s}}.world_normal,
                                os_verts_{{s}}
                            );
                        {% when MipmapMode::None %}
                            let mat_color_{{s}} = pbr_get_material_color_no_mips(
                                tri_indices_{{s}},
                                attribute_data_offset_{{s}},
                                tri_{{s}},
                                pbr_material_{{s}},
                                barycentric_{{s}},
                                vertex_attribute_stride_{{s}},
                                uv_sets_index_{{s}},
                                normal_{{s}},
                                transforms_{{s}}.world_normal,
                                os_verts_{{s}}
                            );
                    {% endmatch %}

                    var sample_color = vec3<f32>(0.0);

                    {% if has_lighting_ibl() %}
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
                    {% endif %}

                    {% if has_lighting_punctual() %}
                        for(var i = 0u; i < lights_info.n_lights; i = i + 1u) {
                            let light_brdf = light_to_brdf(get_light(i), mat_color_{{s}}.normal, standard_coordinates.world_position);
                            sample_color += brdf_direct(mat_color_{{s}}, light_brdf, standard_coordinates.surface_to_camera);
                        }
                    {% endif %}

                    color_sum += sample_color;
                    alpha_sum += mat_color_{{s}}.base.a;
                    }
                }
            {% endfor %}

            // Average and write the result
            if (valid_samples > 0u) {
                textureStore(opaque_tex, coords, vec4<f32>(color_sum / f32(valid_samples), alpha_sum / f32(valid_samples)));
            } else {
                // All samples failed validation - this shouldn't happen in correct scenes
                // Write magenta to make it obvious
                textureStore(opaque_tex, coords, vec4<f32>(1.0, 0.0, 1.0, 1.0));
            }
            return;
        }
    {% endif %}

    let barycentric_data = textureLoad(barycentric_tex, coords, 0);
    let barycentric = vec3<f32>(barycentric_data.x, barycentric_data.y, 1.0 - barycentric_data.x - barycentric_data.y);


    let mesh_meta = mesh_metas[material_meta_offset / META_SIZE_IN_BYTES];

    // Early exit if this shader variant doesn't match this mesh's attributes
    // Each shader variant is compiled for specific uv_sets/color_sets counts
    // Only process pixels where the mesh matches what this variant expects
    {%- match uv_sets %}
        {% when Some with (variant_uv_sets) %}
            if (mesh_meta.uv_set_count != {{ variant_uv_sets }}u) {
                return;
            }
        {% when None %}
            if (mesh_meta.uv_set_count != 0u) {
                return;
            }
    {% endmatch %}
    {%- match color_sets %}
        {% when Some with (variant_color_sets) %}
            if (mesh_meta.color_set_count != {{ variant_color_sets }}u) {
                return;
            }
        {% when None %}
            if (mesh_meta.color_set_count != 0u) {
                return;
            }
    {% endmatch %}

    let material_offset = mesh_meta.material_offset;
    let pbr_material = pbr_get_material(material_offset);

    let vertex_attribute_stride = mesh_meta.vertex_attribute_stride / 4; // 4 bytes per float
    let attribute_indices_offset = mesh_meta.vertex_attribute_indices_offset / 4;
    let attribute_data_offset = mesh_meta.vertex_attribute_data_offset / 4;
    let visibility_data_offset = mesh_meta.visibility_data_offset / 4;
    let uv_sets_index = mesh_meta.uv_sets_index;

    let base_triangle_index = attribute_indices_offset + (triangle_index * 3u);
    let triangle_indices = vec3<u32>(
        attribute_indices[base_triangle_index],
        attribute_indices[base_triangle_index + 1],
        attribute_indices[base_triangle_index + 2]
    );

    let transforms = get_transforms(mesh_meta);

    let standard_coordinates = get_standard_coordinates(coords, screen_dims);

    // Load world-space normal directly from geometry pass output (already transformed with morphs/skins)
    let packed_nt = textureLoad(normal_tangent_tex, coords, 0);
    let tbn = unpack_normal_tangent(packed_nt);
    let world_normal = tbn.N;

    let os_vertices = get_object_space_vertices(visibility_data_offset, triangle_index);

    let lights_info = get_lights_info();

    {% match mipmap %}
        {% when MipmapMode::Gradient %}

            let bary_derivs = textureLoad(barycentric_derivatives_tex, coords, 0);
            // Gradient-based sampling for anisotropic filtering
            let gradients = pbr_get_gradients(
                barycentric,
                bary_derivs,
                pbr_material,
                triangle_indices,
                attribute_data_offset,
                vertex_attribute_stride,
                uv_sets_index,
                world_normal,
                camera.view
            );

            let material_color = pbr_get_material_color_grad(
                triangle_indices,
                attribute_data_offset,
                triangle_index,
                pbr_material,
                barycentric,
                vertex_attribute_stride,
                uv_sets_index,
                gradients,
                world_normal,
                transforms.world_normal,
                os_vertices
            );
        {% when MipmapMode::None %}
            let material_color = pbr_get_material_color_no_mips(
                triangle_indices,
                attribute_data_offset,
                triangle_index,
                pbr_material,
                barycentric,
                vertex_attribute_stride,
                uv_sets_index,
                world_normal,
                transforms.world_normal,
                os_vertices
            );
    {% endmatch %}

    var color = vec3<f32>(0.0);

    {% if has_lighting_ibl() %}
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
    {% endif %}

    {% if has_lighting_punctual() %}
        // Punctual lighting: accumulate contributions from all lights
        for(var i = 0u; i < lights_info.n_lights; i = i + 1u) {
            let light_brdf = light_to_brdf(get_light(i), material_color.normal, standard_coordinates.world_position);
            color += brdf_direct(material_color, light_brdf, standard_coordinates.surface_to_camera);
        }
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

                    // Check if this shader variant matches this sample's mesh attributes
                    var sample_matches_variant_{{s}} = true;
                    {%- match uv_sets %}
                        {% when Some with (variant_uv_sets) %}
                            if (mesh_meta_{{s}}.uv_set_count != {{ variant_uv_sets }}u) {
                                sample_matches_variant_{{s}} = false;
                            }
                        {% when None %}
                            if (mesh_meta_{{s}}.uv_set_count != 0u) {
                                sample_matches_variant_{{s}} = false;
                            }
                    {% endmatch %}
                    {%- match color_sets %}
                        {% when Some with (variant_color_sets) %}
                            if (mesh_meta_{{s}}.color_set_count != {{ variant_color_sets }}u) {
                                sample_matches_variant_{{s}} = false;
                            }
                        {% when None %}
                            if (mesh_meta_{{s}}.color_set_count != 0u) {
                                sample_matches_variant_{{s}} = false;
                            }
                    {% endmatch %}

                    if (sample_matches_variant_{{s}}) {
                        valid_samples++;
                        let material_offset_{{s}} = mesh_meta_{{s}}.material_offset;
                        let pbr_material_{{s}} = pbr_get_material(material_offset_{{s}});

                        // Per-sample mesh data
                        let vertex_attribute_stride_{{s}} = mesh_meta_{{s}}.vertex_attribute_stride / 4;
                    let attribute_indices_offset_{{s}} = mesh_meta_{{s}}.vertex_attribute_indices_offset / 4;
                    let attribute_data_offset_{{s}} = mesh_meta_{{s}}.vertex_attribute_data_offset / 4;
                    let visibility_data_offset_{{s}} = mesh_meta_{{s}}.visibility_data_offset / 4;
                    let uv_sets_index_{{s}} = mesh_meta_{{s}}.uv_sets_index;

                    // Per-sample triangle indices
                    let base_triangle_index_{{s}} = attribute_indices_offset_{{s}} + (tri_id_{{s}} * 3u);
                    let triangle_indices_{{s}} = vec3<u32>(
                        attribute_indices[base_triangle_index_{{s}}],
                        attribute_indices[base_triangle_index_{{s}} + 1],
                        attribute_indices[base_triangle_index_{{s}} + 2]
                    );

                    // Per-sample geometry
                    let bary_{{s}} = textureLoad(barycentric_tex, coords, {{s}});
                    let bary_derivs_{{s}} = textureLoad(barycentric_derivatives_tex, coords, {{s}});
                    let barycentric_{{s}} = vec3<f32>(bary_{{s}}.x, bary_{{s}}.y, 1.0 - bary_{{s}}.x - bary_{{s}}.y);
                    let packed_nt_{{s}} = textureLoad(normal_tangent_tex, coords, {{s}});
                    let tbn_{{s}} = unpack_normal_tangent(packed_nt_{{s}});
                    let normal_{{s}} = tbn_{{s}}.N;
                    let os_vertices_{{s}} = get_object_space_vertices(visibility_data_offset_{{s}}, tri_id_{{s}});
                    let transforms_{{s}} = get_transforms(mesh_meta_{{s}});

                    // Calculate proper gradients for this MSAA sample to enable mipmapping
                    {% match mipmap %}
                        {% when MipmapMode::Gradient %}
                            let gradients_{{s}} = pbr_get_gradients(
                                barycentric_{{s}},
                                bary_derivs_{{s}},
                                pbr_material_{{s}},
                                triangle_indices_{{s}},
                                attribute_data_offset_{{s}},
                                vertex_attribute_stride_{{s}},
                                uv_sets_index_{{s}},
                                normal_{{s}},
                                camera.view
                            );

                            // Compute material color with proper mipmapping
                            let material_color_{{s}} = pbr_get_material_color_grad(
                                triangle_indices_{{s}},
                                attribute_data_offset_{{s}},
                                tri_id_{{s}},
                                pbr_material_{{s}},
                                barycentric_{{s}},
                                vertex_attribute_stride_{{s}},
                                uv_sets_index_{{s}},
                                gradients_{{s}},
                                normal_{{s}},
                                transforms_{{s}}.world_normal,
                                os_vertices_{{s}}
                            );
                        {% when MipmapMode::None %}
                            let material_color_{{s}} = pbr_get_material_color_no_mips(
                                triangle_indices_{{s}},
                                attribute_data_offset_{{s}},
                                tri_id_{{s}},
                                pbr_material_{{s}},
                                barycentric_{{s}},
                                vertex_attribute_stride_{{s}},
                                uv_sets_index_{{s}},
                                normal_{{s}},
                                transforms_{{s}}.world_normal,
                                os_vertices_{{s}}
                            );
                    {% endmatch %}

                    // Compute lighting for this sample
                    var sample_color = vec3<f32>(0.0);

                    {% if has_lighting_ibl() %}
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
                    {% endif %}

                    {% if has_lighting_punctual() %}
                        for(var i = 0u; i < lights_info.n_lights; i = i + 1u) {
                            let light_brdf = light_to_brdf(get_light(i), material_color_{{s}}.normal, standard_coordinates.world_position);
                            sample_color += brdf_direct(material_color_{{s}}, light_brdf, standard_coordinates.surface_to_camera);
                        }
                    {% endif %}

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

    // Write to output texture (non-edge path or non-MSAA)
    textureStore(opaque_tex, coords, vec4<f32>(color, material_color.base.a));
}

fn get_triangle_indices(attribute_indices_offset: u32, triangle_index: u32) -> vec3<u32> {
    let base = attribute_indices_offset + (triangle_index * 3u);
    return vec3<u32>(
        attribute_indices[base],
        attribute_indices[base + 1u],
        attribute_indices[base + 2u],
    );
}
