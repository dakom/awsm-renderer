{% include "pbr_shared_wgsl/color_space.wgsl" %}
{% include "pbr_shared_wgsl/material.wgsl" %}
{% include "pbr_shared_wgsl/textures.wgsl" %}
{% include "pbr_shared_wgsl/debug.wgsl" %}

@group(0) @binding(0) var material_offset_tex: texture_2d<u32>;
@group(0) @binding(1) var world_normal_tex: texture_2d<f32>;
@group(0) @binding(2) var screen_pos_tex_0: texture_2d<f32>;
@group(0) @binding(3) var screen_pos_tex_1: texture_2d<f32>;
@group(0) @binding(4) var opaque_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(5) var<storage, read> materials: array<MaterialRaw>;
{% for texture_binding_string in texture_binding_strings %}
    {{texture_binding_string}}
{% endfor %}


// TODO - bind material uniform buffer, load material properties

@compute @workgroup_size(8, 8)
fn main(
    @builtin(global_invocation_id) gid: vec3<u32>
) {
    let coords = vec2<i32>(gid.xy);
    let dimensions = textureDimensions(opaque_tex);
    
    // Bounds check
    if (coords.x >= i32(dimensions.x) || coords.y >= i32(dimensions.y)) {
        return;
    }

    var color = vec4<f32>(0.0, 0.0, 0.0, 0.0);
    {% if has_atlas %}
        color = debug_test(atlas_tex_0, 0, coords, textureDimensions(opaque_tex));
    {% endif %}

            // let material_offset = textureLoad(material_offset_tex, coords, 0).r;
            // if (material_offset == 0xffffffffu) {
            //     textureStore(opaque_tex, coords, vec4<f32>(0.0, 0.0, 0.0, 0.0));
            //     return; // Skip if material offset is not set
            // }
            // let world_normal = textureLoad(world_normal_tex, coords, 0);

            // // ping/pong this one
            // let screen_pos = textureLoad(screen_pos_tex_0, coords, 0);


            // let material = convert_material(materials[material_offset / 256u]);

            // var color = vec4<f32>(0.0, 0.0, 0.0, 0.0);

            // color = texture_load_base_color(material);

    // if material.has_base_color_texture {
    //     color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
    // }

    // if material.alpha_mode == 1u {
    //     color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
    // } else if material.alpha_mode == 2u {
    //     color = vec4<f32>(0.0, 1.0, 0.0, 1.0);
    // } else if material.alpha_mode == 3u {
    //     color = vec4<f32>(0.0, 0.0, 1.0, 1.0);
    // }


    // if (material_offset != 0u) {
    //     color = vec4<f32>(1.0, 0.0, 0.0, 1.0);
    // }
    
    
    // Write to output texture
    textureStore(opaque_tex, coords, color);
}