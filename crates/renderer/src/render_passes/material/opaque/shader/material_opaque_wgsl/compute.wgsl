@group(0) @binding(0) var material_offset_tex: texture_2d<u32>;
@group(0) @binding(1) var world_normal_tex: texture_2d<f32>;
@group(0) @binding(2) var screen_pos_tex: texture_2d<f32>;
@group(0) @binding(3) var opaque_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(4) var<storage, read> materials: array<Material>;

struct Material {
    offset: u32,
    alpha_mode: u32,
    alpha_cutoff: f32,
    double_sided: u32,
    base_color_factor: vec4<f32>,
    metallic_factor: f32,
    roughness_factor: f32,
    normal_scale: f32,
    occlusion_strength: f32,
    emissive_factor: vec3<f32>,
    //196 bytes of padding to align to 256 bytes
    padding: array<u32, 49>,
};

// TODO - bind material uniform buffer, load material properties

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let coords = vec2<i32>(global_id.xy);
    let dimensions = textureDimensions(opaque_tex);
    
    // Bounds check
    if (coords.x >= i32(dimensions.x) || coords.y >= i32(dimensions.y)) {
        return;
    }

    let material_offset = textureLoad(material_offset_tex, coords, 0).r;
    if (material_offset == 0xffffffffu) {
        textureStore(opaque_tex, coords, vec4<f32>(0.0, 0.0, 0.0, 0.0));
        return; // Skip if material offset is not set
    }
    let world_normal = textureLoad(world_normal_tex, coords, 0);
    let screen_pos = textureLoad(screen_pos_tex, coords, 0);


    let material = materials[material_offset / 256u];

    var color = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    if material.alpha_mode == 1u {
        color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
    } else if material.alpha_mode == 2u {
        color = vec4<f32>(0.0, 1.0, 0.0, 1.0);
    } else if material.alpha_mode == 3u {
        color = vec4<f32>(0.0, 0.0, 1.0, 1.0);
    }


    // if (material_offset != 0u) {
    //     color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
    // }
    
    
    // Write to output texture
    textureStore(opaque_tex, coords, color);
}