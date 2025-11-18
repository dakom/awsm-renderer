// 16-byte packed texture descriptor suitable for uniform/storage buffers.
//
// Layout:
// - size:               width/height in texels (16 bits each)
// - array_and_layer:    array texture index (12 bits), layer index (20 bits)
// - uv_and_sampler:     uv set index (8 bits), sampler index (24 bits)
// - extra:              flags (8 bits), address_mode_u (8 bits),
//                       address_mode_v (8 bits), padding (8 bits)
//
// Notes:
// - 16 bits for width/height covers all practical WebGPU limits.
// - 12 bits for array_index => up to 4096 texture arrays.
// - 20 bits for layer_index => up to 1,048,576 layers (way above spec limits).
// - 8 bits for uv_set_index => up to 256 UV sets (you'll use < 8).
// - 24 bits for sampler_index => up to ~16M samplers (effectively unlimited).
// - flags byte: bit 0 = has mipmaps; rest reserved.
struct TextureInfoRaw {
    // packed: width (low 16 bits), height (high 16 bits)
    size: u32,

    // packed:
    //   bits  0..11 : array_index
    //   bits 12..31 : layer_index
    array_and_layer: u32,

    // packed:
    //   bits  0..7  : uv_set_index
    //   bits  8..31 : sampler_index
    uv_and_sampler: u32,

    // packed:
    //   bits  0..7  : flags
    //                  bit 0 -> has mipmaps
    //                  bits 1..7 reserved
    //   bits  8..15 : address_mode_u
    //   bits 16..23 : address_mode_v
    //   bits 24..31 : padding / reserved
    extra: u32,
};

struct TextureInfo {
    size: vec2<u32>,   // (width, height)
    array_index: u32,
    layer_index: u32,
    uv_set_index: u32,
    sampler_index: u32,
    mipmapped: bool,
    address_mode_u: u32,
    address_mode_v: u32,
};

fn convert_texture_info(raw: TextureInfoRaw) -> TextureInfo {
    // size
    let width:  u32 = raw.size & 0xFFFFu;
    let height: u32 = raw.size >> 16u;

    // array index (12 bits) and layer index (20 bits)
    let array_index: u32 =  raw.array_and_layer & 0xFFFu;      // bits 0..11
    let layer_index: u32 =  raw.array_and_layer >> 12u;        // bits 12..31

    // uv set (8 bits) and sampler index (24 bits)
    let uv_set_index:  u32 =  raw.uv_and_sampler & 0xFFu;      // bits 0..7
    let sampler_index: u32 =  raw.uv_and_sampler >> 8u;        // bits 8..31

    // flags + address modes
    let flags: u32          = raw.extra & 0xFFu;                // bits 0..7
    let mipmapped: bool     = (flags & 0x1u) != 0u;

    let address_mode_u: u32 = (raw.extra >> 8u)  & 0xFFu;       // bits 8..15
    let address_mode_v: u32 = (raw.extra >> 16u) & 0xFFu;       // bits 16..23

    return TextureInfo(
        vec2<u32>(width, height),
        array_index,
        layer_index,
        uv_set_index,
        sampler_index,
        mipmapped,
        address_mode_u,
        address_mode_v,
    );
}


fn texture_uv(attribute_data_offset: u32, triangle_indices: vec3<u32>, barycentric: vec3<f32>, tex_info: TextureInfo, vertex_attribute_stride: u32) -> vec2<f32> {
    let uv0 = _texture_uv_per_vertex(attribute_data_offset, tex_info.uv_set_index, triangle_indices.x, vertex_attribute_stride);
    let uv1 = _texture_uv_per_vertex(attribute_data_offset, tex_info.uv_set_index, triangle_indices.y, vertex_attribute_stride);
    let uv2 = _texture_uv_per_vertex(attribute_data_offset, tex_info.uv_set_index, triangle_indices.z, vertex_attribute_stride);

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


{% match mipmap %}
    {% when MipmapMode::Gradient %}
        // NEW: Sampling with explicit gradients for anisotropic filtering support in compute shaders
        fn texture_pool_sample_grad(info: TextureInfo, attribute_uv: vec2<f32>, uv_derivs: UvDerivs) -> vec4<f32> {
            switch info.array_index {
                {% for i in 0..texture_pool_arrays_len %}
                    case {{ i }}u: {
                        return _texture_pool_sample_grad(info, pool_tex_{{ i }}, attribute_uv, uv_derivs);
                    }
                {% endfor %}
                default: {
                    return vec4<f32>(0.0, 0.0, 0.0, 0.0);
                }
            }
        }


        fn _texture_pool_sample_grad(
            info: TextureInfo,
            tex: texture_2d_array<f32>,
            attribute_uv: vec2<f32>,
            uv_derivs: UvDerivs
        ) -> vec4<f32> {
            var color: vec4<f32>;
            switch info.sampler_index {
                {% for i in 0..texture_pool_samplers_len %}
                    case {{ i }}u: {
                        color = textureSampleGrad(
                            tex,
                            pool_sampler_{{ i }},
                            attribute_uv,
                            i32(info.layer_index),
                            uv_derivs.ddx,
                            uv_derivs.ddy,
                        );
                    }
                {% endfor %}
                default: {
                    color = vec4<f32>(0.0, 0.0, 0.0, 0.0);
                }
            }

            return color;
        }


    {% when MipmapMode::None %}
        // Sampling helpers for the mega-texture atlas. Every fetch receives an explicit LOD so the compute
        // pass can emulate hardware derivative selection.
        fn texture_pool_sample_no_mips(info: TextureInfo, attribute_uv: vec2<f32>) -> vec4<f32> {
            switch info.array_index {
                {% for i in 0..texture_pool_arrays_len %}
                    case {{ i }}u: {
                        return _texture_pool_sample_no_mips(info, pool_tex_{{ i }}, attribute_uv);
                    }
                {% endfor %}
                default: {
                    // If we somehow reference an out-of-range sampler (should not happen), return black to
                    // avoid propagating NaNs that could poison later colour math.
                    return vec4<f32>(0.0, 0.0, 0.0, 0.0);
                }
            }
        }

        fn _texture_pool_sample_no_mips(
            info: TextureInfo,
            tex: texture_2d_array<f32>,
            uv: vec2<f32>,
        ) -> vec4<f32> {
            var color: vec4<f32>;
            switch info.sampler_index {
                {% for i in 0..texture_pool_samplers_len %}
                    case {{ i }}u: {
                        color = textureSampleLevel(
                            tex,
                            pool_sampler_{{ i }},
                            uv,
                            i32(info.layer_index),
                            0
                        );
                    }
                {% endfor %}
                default: {
                    color = vec4<f32>(0.0, 0.0, 0.0, 0.0);
                }
            }

            return color;
        }

{% endmatch %}
