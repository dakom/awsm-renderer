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

fn get_uv(attribute_data_offset: u32, set_index: u32, vertex_index: u32, vertex_attribute_stride: u32) -> vec2<f32> {
    // First get to the right vertex, THEN to the right UV set within that vertex
    let vertex_start = attribute_data_offset + (vertex_index * vertex_attribute_stride);
    let uv_offset = {{ uv_sets_index }}u + (set_index * 2u);
    let index = vertex_start + uv_offset;
    let uv = vec2<f32>(attribute_data[index], attribute_data[index + 1]);

    return uv;
}

fn texture_load_base_color(material: Material, attribute_uv: vec2<f32>) -> vec4<f32> {
    var color = material.base_color_factor;
    if material.has_base_color_texture {
        color = texture_load_atlas(material.base_color_tex_info, attribute_uv);
        //color *= texture_load_atlas(material.base_color_tex_info, attribute_uv);
    }

    // alpha_mode: 0=opaque, 1=mask, 2=blend
    if material.alpha_mode == 0u {
        color.a = 1.0;
    }


    return color;
}

fn texture_load_atlas(info: TextureInfo, attribute_uv: vec2<f32>) -> vec4<f32> {
    switch info.atlas_index {
        {% for texture_load_case_string in texture_load_case_strings %}
            {{texture_load_case_string}}
        {% endfor %}
        default: {
            return vec4<f32>(0.0, 0.0, 0.0, 0.0);
        }
    }
}

fn texture_load_atlas_binding(
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

// from here on - not sure if used

fn texture_load_uv(
    texture: texture_2d<f32>,
    coords: vec2<f32>,
    mip_level: u32,
) -> vec4<f32> {
    let pixel_coords = uv_to_pixel(coords, texture);
    return textureLoad(texture, pixel_coords, mip_level);
}

fn texture_load_2d_array_uv(
    texture: texture_2d_array<f32>,
    coords: vec2<f32>,
    array_index: i32,
    mip_level: u32,
) -> vec4<f32> {
    let pixel_coords = uv_to_pixel_2d_array(coords, texture);
    return textureLoad(texture, pixel_coords, array_index, mip_level);
}

fn uv_to_pixel(uv: vec2<f32>, texture: texture_2d<f32>) -> vec2<i32> {
    let size = vec2<f32>(textureDimensions(texture, 0u));
    return vec2<i32>(uv * size);
}

fn uv_to_pixel_2d_array(uv: vec2<f32>, texture: texture_2d_array<f32>) -> vec2<i32> {
    let size = vec2<f32>(textureDimensions(texture, 0u));
    return vec2<i32>(uv * size);
}
