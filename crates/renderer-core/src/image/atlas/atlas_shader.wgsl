// Write the image atlas entry to the atlas texture
// dispatch with (src_width + 2*padding, src_height + 2*padding, 1)
@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var dst_atlas_tex: texture_storage_2d_array<rgba16float, write>;
@group(0) @binding(2) var<uniform> entry: ImageAtlasEntry;

struct ImageAtlasEntry {
    pixel_offset: vec2<u32>, // This is the offset where the image will be placed in the atlas, past padding
    padding: u32,
    layer_index: u32
};

@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) gid : vec3<u32>) {
    let src_size = textureDimensions(src_tex);
    let padded_size = src_size + vec2<u32>(entry.padding * 2u);
    
    // Bounds check for the padded area we're writing
    if (gid.x >= padded_size.x || gid.y >= padded_size.y) {
        return;
    }
    
    // Calculate source coordinate with edge clamping for padding
    let src_coord = clamp(
        vec2<i32>(gid.xy) - vec2<i32>(entry.padding),
        vec2<i32>(0),
        vec2<i32>(src_size - vec2<u32>(1u))
    );
    
    // Load the source color (with edge clamping for padding)
    let src_color = textureLoad(src_tex, src_coord, 0);
    
    // Calculate destination position
    let dst_xy = entry.pixel_offset - vec2<u32>(entry.padding) + gid.xy;
    
    // Store to atlas texture
    textureStore(dst_atlas_tex, dst_xy, entry.layer_index, src_color);
}