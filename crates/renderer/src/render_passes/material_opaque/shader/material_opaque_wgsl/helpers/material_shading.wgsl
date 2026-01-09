// Helper functions for material shading to reduce repetition in compute.wgsl

{% match mipmap %}
    {% when MipmapMode::Gradient %}
        // Compute material color with gradient-based mipmapping
        fn compute_material_color(
            camera: Camera,
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
            camera: Camera,
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
