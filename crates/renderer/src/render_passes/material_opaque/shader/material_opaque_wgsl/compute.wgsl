/*************** START color_space.wgsl ******************/
{% include "shared_wgsl/color_space.wgsl" %}
/*************** END color_space.wgsl ******************/

/*************** START debug.wgsl ******************/
{% include "shared_wgsl/debug.wgsl" %}
/*************** END debug.wgsl ******************/

/*************** START camera.wgsl ******************/
{% include "shared_wgsl/camera.wgsl" %}
/*************** END camera.wgsl ******************/

/*************** START math.wgsl ******************/
{% include "shared_wgsl/math.wgsl" %}
/*************** END math.wgsl ******************/

/*************** START mesh_meta.wgsl ******************/
{% include "shared_wgsl/material_mesh_meta.wgsl" %}
/*************** END mesh_meta.wgsl ******************/

/*************** START textures.wgsl ******************/
{% include "shared_wgsl/textures.wgsl" %}
/*************** END textures.wgsl ******************/

/*************** START vertex_color.wgsl ******************/
{% include "shared_wgsl/vertex_color.wgsl" %}
/*************** END vertex_color.wgsl ******************/

/*************** START vertex_color_attrib.wgsl ******************/
{% include "material_opaque_wgsl/helpers/vertex_color_attrib.wgsl" %}
/*************** END vertex_color_attrib.wgsl ******************/

/*************** START transforms.wgsl ******************/
{% include "shared_wgsl/transforms.wgsl" %}
/*************** END transforms.wgsl ******************/

/*************** START lights.wgsl ******************/
{% include "shared_wgsl/lighting/lights.wgsl" %}
/*************** END lights.wgsl ******************/

/*************** START brdf.wgsl ******************/
{% include "shared_wgsl/lighting/brdf.wgsl" %}
/*************** END brdf.wgsl ******************/

/*************** START unlit.wgsl ******************/
{% include "shared_wgsl/lighting/unlit.wgsl" %}
/*************** END unlit.wgsl ******************/


/*************** START material.wgsl ******************/
{% include "shared_wgsl/material.wgsl" %}
/*************** END material.wgsl ******************/


{% match mipmap %}
    {% when MipmapMode::Gradient %}
/*************** START mipmap.wgsl ******************/
{% include "material_opaque_wgsl/helpers/mipmap.wgsl" %}
/*************** END mipmap.wgsl ******************/
    {% when _ %}
{% endmatch %}

/*************** START texture_uvs.wgsl ******************/
{% include "material_opaque_wgsl/helpers/texture_uvs.wgsl" %}
/*************** END texture_uvs.wgsl ******************/

/*************** START standard.wgsl ******************/
{% include "material_opaque_wgsl/helpers/standard.wgsl" %}
/*************** END standard.wgsl ******************/

/*************** START material_color.wgsl ******************/
{% include "material_opaque_wgsl/helpers/material_color_calc.wgsl" %}
/*************** END material_color.wgsl ******************/

/*************** START positions.wgsl ******************/
{% include "material_opaque_wgsl/helpers/positions.wgsl" %}
/*************** END positions.wgsl ******************/

/*************** START skybox.wgsl ******************/
{% include "material_opaque_wgsl/helpers/skybox.wgsl" %}
/*************** END skybox.wgsl ******************/

{% if multisampled_geometry %}
/*************** START msaa.wgsl ******************/
{% include "material_opaque_wgsl/helpers/msaa.wgsl" %}
/*************** END msaa.wgsl ******************/
{% endif %}

/*************** START material_shading.wgsl ******************/
{% include "material_opaque_wgsl/helpers/material_shading.wgsl" %}
/*************** END material_shading.wgsl ******************/

{% if debug.any() %}
/*************** START debug.wgsl ******************/
{% include "material_opaque_wgsl/helpers/debug.wgsl" %}
/*************** END debug.wgsl ******************/
{% endif %}


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

    let visibility_data_info = textureLoad(visibility_data_tex, coords, 0);

    let triangle_index = join32(visibility_data_info.x, visibility_data_info.y);
    let material_meta_offset = join32(visibility_data_info.z, visibility_data_info.w);

    let camera = camera_from_raw(camera_raw);


    // early return if we only hit skybox / no geometry (for all samples if MSAA)
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

    // Special case: we've hit the skybox in our main sample (triangle_index is U32_MAX)
    // and yet at least one other MSAA sample hit geometry (any_sample_hit is true from above)
    // so we need to blend all samples properly with the skybox and per-sample shading
    {% if multisampled_geometry %}
        if (triangle_index == U32_MAX) {
            // Process all samples with full per-sample shading and blend
            var color_sum = vec3<f32>(0.0);
            var alpha_sum = 0.0;

            // Count of valid samples (either skybox or geometry)
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
                    let material_mesh_meta_{{s}} = material_mesh_metas[mat_meta_{{s}} / META_SIZE_IN_BYTES];

                    // Check if this shader variant matches this sample's mesh attributes
                    if (mesh_matches_variant(material_mesh_meta_{{s}})) {
                        valid_samples++;
                        let material_offset_{{s}} = material_mesh_meta_{{s}}.material_offset;
                        let pbr_material_{{s}} = pbr_get_material(material_offset_{{s}});

                        let vertex_attribute_stride_{{s}} = material_mesh_meta_{{s}}.vertex_attribute_stride / 4;
                        let attribute_indices_offset_{{s}} = material_mesh_meta_{{s}}.vertex_attribute_indices_offset / 4;
                        let attribute_data_offset_{{s}} = material_mesh_meta_{{s}}.vertex_attribute_data_offset / 4;
                        let visibility_geometry_data_offset_{{s}} = material_mesh_meta_{{s}}.visibility_geometry_data_offset / 4;
                        let uv_sets_index_{{s}} = material_mesh_meta_{{s}}.uv_sets_index;

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
                        let os_verts_{{s}} = get_object_space_vertices(visibility_geometry_data_offset_{{s}}, tri_{{s}});
                        let transforms_{{s}} = get_transforms(material_mesh_meta_{{s}});

                        // Compute material color
                        {% match mipmap %}
                            {% when MipmapMode::Gradient %}
                                let mat_color_{{s}} = compute_material_color(
                                    camera,
                                    tri_indices_{{s}},
                                    attribute_data_offset_{{s}},
                                    tri_{{s}},
                                    pbr_material_{{s}},
                                    barycentric_{{s}},
                                    vertex_attribute_stride_{{s}},
                                    uv_sets_index_{{s}},
                                    normal_{{s}},
                                    transforms_{{s}}.world_normal,
                                    os_verts_{{s}},
                                    bary_derivs_{{s}},
                                );
                            {% when MipmapMode::None %}
                                let mat_color_{{s}} = compute_material_color(
                                    camera,
                                    tri_indices_{{s}},
                                    attribute_data_offset_{{s}},
                                    tri_{{s}},
                                    pbr_material_{{s}},
                                    barycentric_{{s}},
                                    vertex_attribute_stride_{{s}},
                                    uv_sets_index_{{s}},
                                    normal_{{s}},
                                    transforms_{{s}}.world_normal,
                                    os_verts_{{s}},
                                );
                        {% endmatch %}

                        // Apply lighting
                        // TODO - if material is unlit:
                        //let sample_color = unlit(mat_color_{{s}});
                        let sample_color = apply_lighting(
                            mat_color_{{s}},
                            standard_coordinates.surface_to_camera,
                            standard_coordinates.world_position,
                            lights_info
                        );

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

    // If we've reached this point, the main sample hit geometry.
    let material_mesh_meta = material_mesh_metas[material_meta_offset / META_SIZE_IN_BYTES];

    // return early if the geometry hit is hud element (will be redrawn in transparency pass)
    if (material_mesh_meta.is_hud == 1u) {
        // this may bleed a little due to MSAA, but that's okay since huds are redrawn later
        return;
    }


    // Early exit if the main sample doesn't match this mesh's attributes
    // (even if other MSAA samples might match - those will be handled by _some_ main sample matching)
    if (!mesh_matches_variant(material_mesh_meta)) {
        return;
    }

    let barycentric_data = textureLoad(barycentric_tex, coords, 0);
    let barycentric = vec3<f32>(barycentric_data.x, barycentric_data.y, 1.0 - barycentric_data.x - barycentric_data.y);

    let material_offset = material_mesh_meta.material_offset;
    let pbr_material = pbr_get_material(material_offset);

    let vertex_attribute_stride = material_mesh_meta.vertex_attribute_stride / 4; // 4 bytes per float
    let attribute_indices_offset = material_mesh_meta.vertex_attribute_indices_offset / 4;
    let attribute_data_offset = material_mesh_meta.vertex_attribute_data_offset / 4;
    let visibility_geometry_data_offset = material_mesh_meta.visibility_geometry_data_offset / 4;
    let uv_sets_index = material_mesh_meta.uv_sets_index;

    let base_triangle_index = attribute_indices_offset + (triangle_index * 3u);
    let triangle_indices = vec3<u32>(
        attribute_indices[base_triangle_index],
        attribute_indices[base_triangle_index + 1],
        attribute_indices[base_triangle_index + 2]
    );

    let transforms = get_transforms(material_mesh_meta);

    let standard_coordinates = get_standard_coordinates(coords, screen_dims);

    // Load world-space normal directly from geometry pass output (already transformed with morphs/skins)
    let packed_nt = textureLoad(normal_tangent_tex, coords, 0);
    let tbn = unpack_normal_tangent(packed_nt);
    let world_normal = tbn.N;

    let os_vertices = get_object_space_vertices(visibility_geometry_data_offset, triangle_index);

    let lights_info = get_lights_info();

    // Compute material color
    {% match mipmap %}
        {% when MipmapMode::Gradient %}
            let bary_derivs = textureLoad(barycentric_derivatives_tex, coords, 0);
            let material_color = compute_material_color(
                camera,
                triangle_indices,
                attribute_data_offset,
                triangle_index,
                pbr_material,
                barycentric,
                vertex_attribute_stride,
                uv_sets_index,
                world_normal,
                transforms.world_normal,
                os_vertices,
                bary_derivs,
            );
        {% when MipmapMode::None %}
            let material_color = compute_material_color(
                camera,
                triangle_indices,
                attribute_data_offset,
                triangle_index,
                pbr_material,
                barycentric,
                vertex_attribute_stride,
                uv_sets_index,
                world_normal,
                transforms.world_normal,
                os_vertices,
            );
    {% endmatch %}

    // Apply lighting
    // TODO: if unlit var color = unlit(material_color);
    var color = apply_lighting(
        material_color,
        standard_coordinates.surface_to_camera,
        standard_coordinates.world_position,
        lights_info
    );

    // If we're not doing MSAA, we're done here, but if we are, we need to check if this is an edge pixel
    {% if multisampled_geometry && !debug.msaa_detect_edges %}
        let samples_to_process = msaa_sample_count_for_pixel(camera, coords, pixel_center, screen_dims_f32, world_normal, triangle_index);

        // If more than 1 sample to process, it's an edge pixel
        if (samples_to_process > 1u) {
            // Resolve MSAA by averaging all samples
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
                    let material_mesh_meta_{{s}} = material_mesh_metas[material_meta_offset_{{s}} / META_SIZE_IN_BYTES];

                    // Check if this shader variant matches this sample's mesh attributes
                    if (mesh_matches_variant(material_mesh_meta_{{s}})) {
                        valid_samples++;
                        let material_offset_{{s}} = material_mesh_meta_{{s}}.material_offset;
                        let pbr_material_{{s}} = pbr_get_material(material_offset_{{s}});

                        // Per-sample mesh data
                        let vertex_attribute_stride_{{s}} = material_mesh_meta_{{s}}.vertex_attribute_stride / 4;
                        let attribute_indices_offset_{{s}} = material_mesh_meta_{{s}}.vertex_attribute_indices_offset / 4;
                        let attribute_data_offset_{{s}} = material_mesh_meta_{{s}}.vertex_attribute_data_offset / 4;
                        let visibility_geometry_data_offset_{{s}} = material_mesh_meta_{{s}}.visibility_geometry_data_offset / 4;
                        let uv_sets_index_{{s}} = material_mesh_meta_{{s}}.uv_sets_index;

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
                        let os_vertices_{{s}} = get_object_space_vertices(visibility_geometry_data_offset_{{s}}, tri_id_{{s}});
                        let transforms_{{s}} = get_transforms(material_mesh_meta_{{s}});

                        // Compute material color
                        {% match mipmap %}
                            {% when MipmapMode::Gradient %}
                                let material_color_{{s}} = compute_material_color(
                                    camera,
                                    triangle_indices_{{s}},
                                    attribute_data_offset_{{s}},
                                    tri_id_{{s}},
                                    pbr_material_{{s}},
                                    barycentric_{{s}},
                                    vertex_attribute_stride_{{s}},
                                    uv_sets_index_{{s}},
                                    normal_{{s}},
                                    transforms_{{s}}.world_normal,
                                    os_vertices_{{s}},
                                    bary_derivs_{{s}},
                                );
                            {% when MipmapMode::None %}
                                let material_color_{{s}} = compute_material_color(
                                    camera,
                                    triangle_indices_{{s}},
                                    attribute_data_offset_{{s}},
                                    tri_id_{{s}},
                                    pbr_material_{{s}},
                                    barycentric_{{s}},
                                    vertex_attribute_stride_{{s}},
                                    uv_sets_index_{{s}},
                                    normal_{{s}},
                                    transforms_{{s}}.world_normal,
                                    os_vertices_{{s}},
                                );
                        {% endmatch %}

                        // Apply lighting
                        // TODO: if unlit let sample_color = unlit(material_color_{{s}});
                        let sample_color = apply_lighting(
                            material_color_{{s}},
                            standard_coordinates.surface_to_camera,
                            standard_coordinates.world_position,
                            lights_info
                        );

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


    {% if debug.normals %}
        // Debug visualization: encode normal as color
        textureStore(opaque_tex, coords, vec4<f32>(debug_normals(world_normal), 1.0));
        return;
    {% else if debug.base_color %}
        textureStore(opaque_tex, coords, material_color.base);
        return;
    {% endif %}

    // Write to output texture in the case of no MSAA or non-edge pixel
    textureStore(opaque_tex, coords, vec4<f32>(color, material_color.base.a));
}

// Check if a mesh's attributes match what this shader variant was compiled for.
// Each variant is compiled for specific uv_sets/color_sets counts.
// In the future, this will also check material type (pbr, toon, etc).
fn mesh_matches_variant(material_mesh_meta: MaterialMeshMeta) -> bool {
    // Check UV set count
    {%- match uv_sets %}
        {% when Some with (variant_uv_sets) %}
            if (material_mesh_meta.uv_set_count != {{ variant_uv_sets }}u) {
                return false;
            }
        {% when None %}
            if (material_mesh_meta.uv_set_count != 0u) {
                return false;
            }
    {% endmatch %}

    // Check color set count
    {%- match color_sets %}
        {% when Some with (variant_color_sets) %}
            if (material_mesh_meta.color_set_count != {{ variant_color_sets }}u) {
                return false;
            }
        {% when None %}
            if (material_mesh_meta.color_set_count != 0u) {
                return false;
            }
    {% endmatch %}

    return true;
}

fn get_triangle_indices(attribute_indices_offset: u32, triangle_index: u32) -> vec3<u32> {
    let base = attribute_indices_offset + (triangle_index * 3u);
    return vec3<u32>(
        attribute_indices[base],
        attribute_indices[base + 1u],
        attribute_indices[base + 2u],
    );
}
