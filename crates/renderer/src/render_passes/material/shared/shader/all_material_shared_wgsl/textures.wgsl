// 24 bytes
struct TextureInfoRaw {
    pixel_offset_x: u32,
    pixel_offset_y: u32,
    width: u32,
    height: u32,
    atlas_layer_index: u32,
    entry_attribute_uv_set_index: u32,
}

struct TextureInfo {
    pixel_offset: vec2<u32>,
    size: vec2<u32>,
    atlas_index: u32,
    layer_index: u32,
    entry_index: u32,
    attribute_uv_set_index: u32,
}

fn convert_texture_info(raw: TextureInfoRaw) -> TextureInfo {
    return TextureInfo(
        vec2<u32>(raw.pixel_offset_x, raw.pixel_offset_y),
        vec2<u32>(raw.width, raw.height),
        raw.atlas_layer_index & 0xFFFFu,           // atlas_index (16 bits)
        (raw.atlas_layer_index >> 16u) & 0xFFFFu, // layer_index (16 bits)
        raw.entry_attribute_uv_set_index & 0xFFFFu,    // entry_index (16 bits)
        (raw.entry_attribute_uv_set_index >> 16u) & 0xFFFFu, // attribute_uv_index (16 bits)
    );
}


fn texture_uv(attribute_data_offset: u32, triangle_indices: vec3<u32>, barycentric: vec3<f32>, tex_info: TextureInfo, vertex_attribute_stride: u32) -> vec2<f32> {
    let uv0 = _texture_uv_per_vertex(attribute_data_offset, tex_info.attribute_uv_set_index, triangle_indices.x, vertex_attribute_stride);
    let uv1 = _texture_uv_per_vertex(attribute_data_offset, tex_info.attribute_uv_set_index, triangle_indices.y, vertex_attribute_stride);
    let uv2 = _texture_uv_per_vertex(attribute_data_offset, tex_info.attribute_uv_set_index, triangle_indices.z, vertex_attribute_stride);

    let interpolated_uv = barycentric.x * uv0 + barycentric.y * uv1 + barycentric.z * uv2;

    return interpolated_uv;
}

fn _texture_uv_per_vertex(attribute_data_offset: u32, set_index: u32, vertex_index: u32, vertex_attribute_stride: u32) -> vec2<f32> {
    // First get to the right vertex, THEN to the right UV set within that vertex
    let vertex_start = attribute_data_offset + (vertex_index * vertex_attribute_stride);
    // `uv_sets_index` points to the beginning of TEXCOORD_0 inside the packed stream.
    // Each additional UV set contributes two more floats per vertex.
    let uv_offset = {{ uv_sets_index }}u + (set_index * 2u);
    let index = vertex_start + uv_offset;
    let uv = vec2<f32>(attribute_data[index], attribute_data[index + 1]);

    return uv;
}


fn texture_load_atlas_srgb(info: TextureInfo, attribute_uv: vec2<f32>) -> vec4<f32> {
    let raw_color = _texture_load_atlas(info, attribute_uv);
    return vec4<f32>(srgb_to_linear(raw_color.rgb), raw_color.a);
}

fn _texture_load_atlas(info: TextureInfo, attribute_uv: vec2<f32>) -> vec4<f32> {
    switch info.atlas_index {
        {% for i in 0..total_atlas_index %}
            case {{ i }}u: {
                return _texture_load_atlas_binding(info, atlas_tex_{{ i }}, attribute_uv);
            }
        {% endfor %}
        default: {
            return vec4<f32>(0.0, 0.0, 0.0, 0.0);
        }
    }
}

fn _texture_load_atlas_binding(
    info: TextureInfo,
    atlas_tex: texture_2d_array<f32>,
    attribute_uv: vec2<f32>,
) -> vec4<f32> {
    let mip_level = 0u; // TODO: Handle mip levels

    let cell_size = vec2<f32>(info.size);
    let cell_offset = attribute_uv * cell_size;

    var coords = info.pixel_offset + vec2<u32>(cell_offset);

    var color = textureLoad(atlas_tex, coords, info.layer_index, mip_level);

    color.a = 1.0;

    return color;

}
