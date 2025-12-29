/*************** START bind_groups.wgsl ******************/
{% include "material_opaque_wgsl/bind_groups.wgsl" %}
/*************** END bind_groups.wgsl ******************/

/*************** START math.wgsl ******************/
{% include "utils_wgsl/math.wgsl" %}
/*************** END math.wgsl ******************/

/*************** START camera.wgsl ******************/
{% include "geometry_and_all_material_wgsl/camera.wgsl" %}
/*************** END camera.wgsl ******************/

/*************** START skybox.wgsl ******************/
{% include "material_opaque_wgsl/helpers/skybox.wgsl" %}
/*************** END skybox.wgsl ******************/

/*************** START mesh_meta.wgsl ******************/
{% include "opaque_and_transparency_wgsl/material_mesh_meta.wgsl" %}
/*************** END mesh_meta.wgsl ******************/

/*************** START textures.wgsl ******************/
{% include "opaque_and_transparency_wgsl/textures.wgsl" %}
/*************** END textures.wgsl ******************/

/*************** START material.wgsl ******************/
{% include "opaque_and_transparency_wgsl/pbr/material.wgsl" %}
/*************** END material.wgsl ******************/

/*************** START vertex_color.wgsl ******************/
{% include "opaque_and_transparency_wgsl/vertex_color.wgsl" %}
/*************** END vertex_color.wgsl ******************/

/*************** START lights.wgsl ******************/
{% include "opaque_and_transparency_wgsl/pbr/lighting/lights.wgsl" %}
/*************** END lights.wgsl ******************/



@compute @workgroup_size(8, 8)
fn main(
    @builtin(global_invocation_id) gid: vec3<u32>
) {
    let coords = vec2<i32>(gid.xy);
    let screen_dims = textureDimensions(opaque_tex);
    let screen_dims_i32 = vec2<i32>(i32(screen_dims.x), i32(screen_dims.y));
    let screen_dims_f32 = vec2<f32>(f32(screen_dims.x), f32(screen_dims.y));
    let pixel_center = vec2<f32>(f32(coords.x) + 0.5, f32(coords.y) + 0.5);

    // Bounds check
    if (coords.x >= screen_dims_i32.x || coords.y >= screen_dims_i32.y) {
        return;
    }

    let skybox_col = sample_skybox(coords, screen_dims_f32, camera, skybox_tex, skybox_sampler);

    textureStore(opaque_tex, coords, skybox_col);

}
