// Helper functions for material shading to reduce repetition in compute.wgsl

{% match mipmap %}
    {% when MipmapMode::Gradient %}
        // Compute material color with gradient-based mipmapping
        fn compute_material_color(
            triangle_indices: vec3<u32>,
            attribute_data_offset: u32,
            triangle_index: u32,
            pbr_material: PbrMaterial,
            barycentric: vec3<f32>,
            vertex_attribute_stride: u32,
            uv_sets_index: u32,
            world_normal: vec3<f32>,
            world_normal_transform: mat3x3<f32>,
            os_vertices: ObjectSpaceVertices,
            bary_derivs: vec4<f32>,
        ) -> PbrMaterialColor {
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

            return pbr_get_material_color_grad(
                triangle_indices,
                attribute_data_offset,
                triangle_index,
                pbr_material,
                barycentric,
                vertex_attribute_stride,
                uv_sets_index,
                gradients,
                world_normal,
                world_normal_transform,
                os_vertices
            );
        }
    {% when MipmapMode::None %}
        // Compute material color without mipmapping
        fn compute_material_color(
            triangle_indices: vec3<u32>,
            attribute_data_offset: u32,
            triangle_index: u32,
            pbr_material: PbrMaterial,
            barycentric: vec3<f32>,
            vertex_attribute_stride: u32,
            uv_sets_index: u32,
            world_normal: vec3<f32>,
            world_normal_transform: mat3x3<f32>,
            os_vertices: ObjectSpaceVertices,
        ) -> PbrMaterialColor {
            return pbr_get_material_color_no_mips(
                triangle_indices,
                attribute_data_offset,
                triangle_index,
                pbr_material,
                barycentric,
                vertex_attribute_stride,
                uv_sets_index,
                world_normal,
                world_normal_transform,
                os_vertices
            );
        }
{% endmatch %}

// Apply all enabled lighting to a material and return the final color
fn apply_lighting(
    material_color: PbrMaterialColor,
    surface_to_camera: vec3<f32>,
    world_position: vec3<f32>,
    lights_info: LightsInfo,
) -> vec3<f32> {
    var color = vec3<f32>(0.0);

    {% if has_lighting_ibl() %}
        color = brdf_ibl(
            material_color,
            material_color.normal,
            surface_to_camera,
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
            let light_brdf = light_to_brdf(get_light(i), material_color.normal, world_position);
            color += brdf_direct(material_color, light_brdf, surface_to_camera);
        }
    {% endif %}

    return color;
}
