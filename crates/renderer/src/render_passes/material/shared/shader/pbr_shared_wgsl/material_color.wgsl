// Contains the final material properties after sampling all PBR textures.
// IMPORTANT: The 'normal' field contains the perturbed normal (with normal map applied),
// NOT the geometry normal. Always use material_color.normal for lighting calculations!
struct PbrMaterialColor {
    base: vec4<f32>,
    metallic_roughness: vec2<f32>,
    normal: vec3<f32>,  // Perturbed normal from normal mapping (use this for lighting!)
    occlusion: f32,
    emissive: vec3<f32>,
};

// Samples all PBR material textures and computes the final material properties including
// normal mapping. The returned normal is the perturbed normal (with normal map applied) and
// should be used for all lighting calculations.
fn pbr_get_material_color(
    triangle_indices: vec3<u32>,
    attribute_data_offset: u32,
    triangle_index: u32,
    material: PbrMaterial,
    barycentric: vec3<f32>,
    vertex_attribute_stride: u32,
    mip_levels: PbrMaterialMipLevels,
    world_normal: vec3<f32>,
    normal_matrix: mat3x3<f32>,
    os_vertices: ObjectSpaceVertices,
) -> PbrMaterialColor {

    var base = _pbr_material_base_color(
        material,
        texture_uv(
            attribute_data_offset,
            triangle_indices,
            barycentric,
            material.base_color_tex_info,
            vertex_attribute_stride,
        ),
        mip_levels.base_color,
    );

    {%- match color_sets %}
        {% when Some with (color_sets) %}
            base *= vertex_color(
                attribute_data_offset,
                triangle_indices,
                barycentric,
                material.color_info,
                vertex_attribute_stride,
            );
        {% when _ %}
    {% endmatch %}


    let metallic_roughness = _pbr_material_metallic_roughness_color (
        material,
        texture_uv(
            attribute_data_offset,
            triangle_indices,
            barycentric,
            material.metallic_roughness_tex_info,
            vertex_attribute_stride,
        ),
        mip_levels.metallic_roughness,
    );

    // Compute the normal-mapped normal by applying the normal texture to the geometry normal
    // using either stored tangents or computed tangent space from UVs
    let normal = _pbr_normal_color(
        material,
        texture_uv(
            attribute_data_offset,
            triangle_indices,
            barycentric,
            material.normal_tex_info,
            vertex_attribute_stride,
        ),
        mip_levels.normal,
        world_normal,
        barycentric,
        triangle_indices,
        attribute_data_offset,
        vertex_attribute_stride,
        normal_matrix,
        os_vertices,
    );

    let occlusion = _pbr_occlusion_color(
        material,
        texture_uv(
            attribute_data_offset,
            triangle_indices,
            barycentric,
            material.occlusion_tex_info,
            vertex_attribute_stride,
        ),
        mip_levels.occlusion,
    );

    let emissive = _pbr_material_emissive_color(
        material,
        texture_uv(
            attribute_data_offset,
            triangle_indices,
            barycentric,
            material.emissive_tex_info,
            vertex_attribute_stride,
        ),
        mip_levels.emissive,
    );

    return PbrMaterialColor(
        base,
        metallic_roughness,
        normal,
        occlusion,
        emissive,
    );
}

// Base Color
fn _pbr_material_base_color(material: PbrMaterial, attribute_uv: vec2<f32>, mip_level: f32) -> vec4<f32> {
    var color = material.base_color_factor;


    if material.has_base_color_texture {
        color *=
            texture_sample_atlas(material.base_color_tex_info, attribute_uv, mip_level);
    }


    // compute pass only deals with fully opaque
    // mask and blend are handled in the fragment shader
    color.a = 1.0;

    return color;
}

fn _pbr_material_metallic_roughness_color(
    material: PbrMaterial,
    attribute_uv: vec2<f32>,
    mip_level: f32,
) -> vec2<f32> {
    var color = vec2<f32>(material.metallic_factor, material.roughness_factor);
    if material.has_metallic_roughness_texture {
        let tex = texture_sample_atlas(material.metallic_roughness_tex_info, attribute_uv, mip_level);
        // glTF uses B channel for metallic, G channel for roughness
        color *= vec2<f32>(tex.b, tex.g);
    }
    return color;
}

// Applies normal mapping by constructing a TBN (tangent-bitangent-normal) matrix and
// transforming the normal texture sample from tangent space to world space.
// Falls back through three methods: stored tangents -> computed from UVs -> generated basis
fn _pbr_normal_color(
    material: PbrMaterial,
    attribute_uv: vec2<f32>,
    mip_level: f32,
    world_normal: vec3<f32>,
    barycentric: vec3<f32>,
    triangle_indices: vec3<u32>,
    attribute_data_offset: u32,
    vertex_attribute_stride: u32,
    normal_matrix: mat3x3<f32>,
    os_vertices: ObjectSpaceVertices,
) -> vec3<f32> {
    if !material.has_normal_texture {
        return world_normal;
    }

    // Sample normal map and unpack from [0,1] to [-1,1] range
    // Use mip_level parameter (not forced to 0) to get proper filtering
    let tex = texture_sample_atlas(material.normal_tex_info, attribute_uv, mip_level);
    var tangent_normal = vec3<f32>(
        (tex.r * 2.0 - 1.0) * material.normal_scale,
        (tex.g * 2.0 - 1.0) * material.normal_scale,
        tex.b * 2.0 - 1.0,
    );

    var T = vec3<f32>(0.0);
    var B = vec3<f32>(0.0);
    var basis_valid = false;

    // Method 1: Use stored tangents from glTF TANGENT attribute if available
    // Stride >= 7 means we have: normal (3 floats) + tangent (4 floats) = 7+ floats per vertex
    if (vertex_attribute_stride >= 7u) {
        let tangent = get_vertex_tangent(attribute_data_offset, triangle_indices, barycentric, vertex_attribute_stride);
        let tangent_len_sq = dot(tangent.xyz, tangent.xyz);
        if (tangent_len_sq > 0.0) {
            var world_tangent = normalize(normal_matrix * tangent.xyz);
            // Gram-Schmidt orthogonalization to ensure tangent is perpendicular to normal
            world_tangent = normalize(world_tangent - world_normal * dot(world_normal, world_tangent));
            // Compute bitangent using handedness sign (tangent.w = ±1)
            let world_bitangent = normalize(cross(world_normal, world_tangent) * tangent.w);
            T = world_tangent;
            B = world_bitangent;
            basis_valid = true;
        }
    }

    // Method 2: Compute tangent space from triangle UV derivatives (fallback for missing tangents)
    // This is used for glTF models that don't include TANGENT attributes (e.g., NormalTangentTest)
    let set_index = material.normal_tex_info.attribute_uv_set_index;
    let uv0 = _texture_uv_per_vertex(attribute_data_offset, set_index, triangle_indices.x, vertex_attribute_stride);
    let uv1 = _texture_uv_per_vertex(attribute_data_offset, set_index, triangle_indices.y, vertex_attribute_stride);
    let uv2 = _texture_uv_per_vertex(attribute_data_offset, set_index, triangle_indices.z, vertex_attribute_stride);

    let delta_pos1 = os_vertices.p1 - os_vertices.p0;
    let delta_pos2 = os_vertices.p2 - os_vertices.p0;
    let delta_uv1 = uv1 - uv0;
    let delta_uv2 = uv2 - uv0;
    let det = delta_uv1.x * delta_uv2.y - delta_uv1.y * delta_uv2.x;

    if (!basis_valid && abs(det) > 1e-6) {
        let r = 1.0 / det;
        let tangent_os = (delta_pos1 * delta_uv2.y - delta_pos2 * delta_uv1.y) * r;
        var world_tangent = normalize(normal_matrix * tangent_os);
        world_tangent = normalize(world_tangent - world_normal * dot(world_normal, world_tangent));
        let world_bitangent = normalize(cross(world_normal, world_tangent));
        T = world_tangent;
        B = world_bitangent;
        basis_valid = true;
    }

    // Method 3: Generate a fallback orthonormal basis (last resort)
    if (!basis_valid) {
        let up = vec3<f32>(0.0, 1.0, 0.0);
        var fallback = normalize(cross(up, world_normal));
        if (dot(fallback, fallback) < 1e-6) {
            fallback = normalize(cross(vec3<f32>(1.0, 0.0, 0.0), world_normal));
        }
        T = fallback;
        B = normalize(cross(world_normal, T));
    }

    // Transform the tangent-space normal to world space using the TBN matrix
    let tbn = mat3x3<f32>(T, B, world_normal);
    return normalize(tbn * tangent_normal);
}

fn _pbr_occlusion_color(
    material: PbrMaterial,
    attribute_uv: vec2<f32>,
    mip_level: f32,
) -> f32 {
    var occlusion = 1.0;
    if material.has_occlusion_texture {
        let tex = texture_sample_atlas(material.occlusion_tex_info, attribute_uv, mip_level);
        occlusion = mix(1.0, tex.r, material.occlusion_strength);
    }
    return occlusion;
}

fn _pbr_material_emissive_color(
    material: PbrMaterial,
    attribute_uv: vec2<f32>,
    mip_level: f32,
) -> vec3<f32> {
    var color = material.emissive_factor;
    if material.has_emissive_texture {
        color *=
            texture_sample_atlas(material.emissive_tex_info, attribute_uv, mip_level).rgb;
    }

    color *= material.emissive_strength;

    return color;
}

// Interpolate tangent vectors across a triangle using barycentric coordinates
fn get_vertex_tangent(
    attribute_data_offset: u32,
    triangle_indices: vec3<u32>,
    barycentric: vec3<f32>,
    vertex_attribute_stride: u32,
) -> vec4<f32> {
    let t0 = _get_vertex_tangent(attribute_data_offset, triangle_indices.x, vertex_attribute_stride);
    let t1 = _get_vertex_tangent(attribute_data_offset, triangle_indices.y, vertex_attribute_stride);
    let t2 = _get_vertex_tangent(attribute_data_offset, triangle_indices.z, vertex_attribute_stride);
    return barycentric.x * t0 + barycentric.y * t1 + barycentric.z * t2;
}

// Read tangent from packed attribute buffer
// Attribute layout per vertex: [normal.xyz (3 floats), tangent.xyzw (4 floats), ...]
fn _get_vertex_tangent(
    attribute_data_offset: u32,
    vertex_index: u32,
    vertex_attribute_stride: u32,
) -> vec4<f32> {
    if (vertex_attribute_stride < 7u) {
        // No tangent data available (stride < normal(3) + tangent(4))
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    let vertex_start = attribute_data_offset + (vertex_index * vertex_attribute_stride);
    let base = vertex_start + 3u; // tangents follow normals (3 float offset)

    return vec4<f32>(
        attribute_data[base],
        attribute_data[base + 1u],
        attribute_data[base + 2u],
        attribute_data[base + 3u],  // w component = handedness sign (±1)
    );
}
