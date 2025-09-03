fn texture_load_base_color(material: Material, attribute_uv: vec2<f32>) -> vec4<f32> {
    var color = material.base_color_factor;
    if material.has_base_color_texture {
        color *= texture_load_atlas(material.base_color_tex_info, attribute_uv); 
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
    let cell_offset = vec2<u32>(attribute_uv * cell_size);

    var coords = info.pixel_offset + cell_offset;

    // coords.x += 250;
    // coords.y += 250;

    var color = textureLoad(atlas_tex, coords, info.layer_index, mip_level);

    color.a = 1.0;

    return color;

}

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