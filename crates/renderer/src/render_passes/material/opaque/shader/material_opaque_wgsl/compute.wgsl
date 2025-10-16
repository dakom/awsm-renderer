{% include "all_material_shared_wgsl/color_space.wgsl" %}
{% include "all_material_shared_wgsl/debug.wgsl" %}
{% include "all_material_shared_wgsl/math.wgsl" %}
{% include "all_material_shared_wgsl/meta.wgsl" %}
{% include "all_material_shared_wgsl/projection.wgsl" %}
{% include "all_material_shared_wgsl/textures.wgsl" %}
{% include "pbr_shared_wgsl/lighting/brdf.wgsl" %}
{% include "pbr_shared_wgsl/lighting/unlit.wgsl" %}
{% include "pbr_shared_wgsl/material.wgsl" %}
{% include "pbr_shared_wgsl/material_color.wgsl" %}

// Mirrors the CPU-side `CameraBuffer` layout. The extra inverse matrices and frustum rays give
// us everything needed to reconstruct world-space positions from a depth value inside this
// compute pass.
struct CameraUniform {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    inv_proj: mat4x4<f32>,
    inv_view: mat4x4<f32>,
    position: vec3<f32>,
    frame_count: u32,
    frustum_rays: array<vec4<f32>, 4>,
};

@group(0) @binding(0) var<storage, read> mesh_metas: array<MaterialMeshMeta>;
@group(0) @binding(1) var visibility_data_tex: texture_2d<f32>;
@group(0) @binding(2) var opaque_tex: texture_storage_2d<rgba16float, write>;
@group(0) @binding(3) var<storage, read> materials: array<PbrMaterialRaw>; // TODO - just raw data, derive PbrMaterialRaw if that's what we have?
@group(0) @binding(4) var<storage, read> attribute_indices: array<u32>;
@group(0) @binding(5) var<storage, read> attribute_data: array<f32>;
@group(0) @binding(6) var<uniform> camera: CameraUniform;
@group(0) @binding(7) var depth_tex: texture_depth_2d;
{% for b in texture_bindings %}
    @group({{ b.group }}u) @binding({{ b.binding }}u) var atlas_tex_{{ b.atlas_index }}: texture_2d_array<f32>;
{% endfor %}
{% for s in sampler_bindings %}
    @group({{ s.group }}u) @binding({{ s.binding }}u) var atlas_sampler_{{ s.sampler_index }}: sampler;
{% endfor %}


const f32_max = 2139095039u;

const ambient = vec3<f32>(1.0); // TODO - make this settable, or get from IBL

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

    let visibility_data = textureLoad(visibility_data_tex, coords, 0);

    let triangle_index = bitcast<u32>(visibility_data.x);
    // early return if nothing was drawn at this pixel
    if (triangle_index == f32_max) {
        return;
    }
    let material_meta_offset = bitcast<u32>(visibility_data.y);
    let barycentric = vec3<f32>(visibility_data.z, visibility_data.w, 1.0 - visibility_data.z - visibility_data.w);


    let mesh_meta = mesh_metas[material_meta_offset / meta_size_in_bytes];
    let material_offset = mesh_meta.material_offset;

    let pbr_material = pbr_get_material(material_offset);

    // Skip work when the mesh doesn't provide enough UV data for the material.
    if !pbr_should_run(pbr_material) {
        return;
    }

    let vertex_attribute_stride = mesh_meta.vertex_attribute_stride / 4; // 4 bytes per float
    let attribute_indices_offset = mesh_meta.vertex_attribute_indices_offset / 4;
    let attribute_data_offset = mesh_meta.vertex_attribute_data_offset / 4;

    // Sample the depth buffer written by the visibility pass. Because we request level 0 the GPU
    // picks the highest-resolution mip. The resulting value is still in clip-space depth, so we
    // convert back to NDC and then to world space with the inverse view-projection matrix.
    let depth_sample = textureLoad(depth_tex, coords, 0);
    let depth_dims = textureDimensions(depth_tex, 0);
    let screen_dims = vec2<f32>(f32(depth_dims.x), f32(depth_dims.y));
    // Convert the integer pixel coordinate into normalized device coordinates using the pixel
    // centre. This matches how rasterisation computes attribute interpolation.
    let pixel_center = (vec2<f32>(f32(coords.x), f32(coords.y)) + vec2<f32>(0.5, 0.5)) / screen_dims;
    let ndc = vec3<f32>(pixel_center * 2.0 - vec2<f32>(1.0, 1.0), depth_sample * 2.0 - 1.0);
    let clip_position = vec4<f32>(ndc, 1.0);
    let world_position_h = camera.inv_view_proj * clip_position;
    let world_position = world_position_h.xyz / world_position_h.w;
    let to_camera = camera.position - world_position;
    var surface_to_camera = vec3<f32>(0.0, 0.0, 1.0);
    if (length(to_camera) > 0.0) {
        surface_to_camera = normalize(to_camera);
    }

    let triangle_indices_current = get_triangle_indices(attribute_indices_offset, triangle_index);
    let screen_dims_i32 = vec2<i32>(i32(depth_dims.x), i32(depth_dims.y));

    // Derive texture LODs by comparing UVs between neighbouring pixels. This approximates the
    // derivatives hardware would have given us in a fragment shader and keeps mip selection
    // consistent even though we deferred shading to a compute pass.
    // Approximate mip selection via neighbour UVs. We wrap the intermediate UVs using the same
    // addressing logic as the sampler so clamp-to-edge regions don't artificially inflate the
    // gradient (which would otherwise generate thin seams near borders).
    let base_color_lod = compute_texture_lod(
        pbr_material.base_color_tex_info,
        coords,
        triangle_indices_current,
        triangle_index,
        barycentric,
        attribute_indices_offset,
        attribute_data_offset,
        vertex_attribute_stride,
        screen_dims_i32,
        pbr_material.has_base_color_texture,
    );

    let metallic_roughness_lod = compute_texture_lod(
        pbr_material.metallic_roughness_tex_info,
        coords,
        triangle_indices_current,
        triangle_index,
        barycentric,
        attribute_indices_offset,
        attribute_data_offset,
        vertex_attribute_stride,
        screen_dims_i32,
        pbr_material.has_metallic_roughness_texture,
    );

    let normal_lod = compute_texture_lod(
        pbr_material.normal_tex_info,
        coords,
        triangle_indices_current,
        triangle_index,
        barycentric,
        attribute_indices_offset,
        attribute_data_offset,
        vertex_attribute_stride,
        screen_dims_i32,
        pbr_material.has_normal_texture,
    );

    let occlusion_lod = compute_texture_lod(
        pbr_material.occlusion_tex_info,
        coords,
        triangle_indices_current,
        triangle_index,
        barycentric,
        attribute_indices_offset,
        attribute_data_offset,
        vertex_attribute_stride,
        screen_dims_i32,
        pbr_material.has_occlusion_texture,
    );

    let emissive_lod = compute_texture_lod(
        pbr_material.emissive_tex_info,
        coords,
        triangle_indices_current,
        triangle_index,
        barycentric,
        attribute_indices_offset,
        attribute_data_offset,
        vertex_attribute_stride,
        screen_dims_i32,
        pbr_material.has_emissive_texture,
    );

    let texture_lods = PbrMaterialMipLevels(
        base_color_lod,
        metallic_roughness_lod,
        normal_lod,
        occlusion_lod,
        emissive_lod,
    );

    let material_color = pbr_get_material_color(
        attribute_indices_offset,
        attribute_data_offset,
        triangle_index,
        pbr_material,
        barycentric,
        vertex_attribute_stride,
        texture_lods,
    );

    var color = vec3<f32>(0.0);

    // TODO - lighting
    // something like:
    // for(var i = 0u; i < n_lights; i = i + 1u) {
    //     let light_brdf = light_to_brdf(get_light(i), normal, input.world_position);

    //     if (light_brdf.n_dot_l > 0.0001) {
    //         color += brdf(input, material, light_brdf, ambient, surface_to_camera);
    //     } else {
    //         color += ambient * material.base_color.rgb;
    //     }
    // }
    //
    // For now, just color with full material color (emissive etc.) but unlit

    color = unlit(material_color, ambient, surface_to_camera);

    // Write to output texture
    textureStore(opaque_tex, coords, vec4<f32>(color, material_color.base.a));
}

// Decide whether we have enough UV inputs to evaluate every texture referenced by the material.
// Each branch checks the number of TEXCOORD sets exposed by the mesh (see `attributes.rs`) against
// what the material expects, and returns false when sampling would read garbage data.
fn pbr_should_run(material: PbrMaterial) -> bool {
    {%- match uv_sets %}
        {% when Some with (uv_sets) %}
            return pbr_material_uses_uv_count(material, {{ uv_sets }});
        {% when None %}
            return !pbr_material_has_any_uvs(material);
    {% endmatch %}
}

fn pbr_material_has_any_uvs(material: PbrMaterial) -> bool {
    // When the mesh supplies zero UV sets we can only shade materials that also skip every UV-backed texture.
    return material.has_base_color_texture ||
        material.has_metallic_roughness_texture ||
        material.has_normal_texture ||
        material.has_occlusion_texture ||
        material.has_emissive_texture;
}

fn pbr_material_uses_uv_count(material: PbrMaterial, uv_set_count: u32) -> bool {
    // Validate every texture's UV requirements individually so that a single mismatched binding aborts shading.
    if !texture_fits_uv_budget(material.has_base_color_texture, material.base_color_tex_info, uv_set_count) {
        return false;
    }

    if !texture_fits_uv_budget(material.has_metallic_roughness_texture, material.metallic_roughness_tex_info, uv_set_count) {
        return false;
    }

    if !texture_fits_uv_budget(material.has_normal_texture, material.normal_tex_info, uv_set_count) {
        return false;
    }

    if !texture_fits_uv_budget(material.has_occlusion_texture, material.occlusion_tex_info, uv_set_count) {
        return false;
    }

    if !texture_fits_uv_budget(material.has_emissive_texture, material.emissive_tex_info, uv_set_count) {
        return false;
    }

    return true;
}

fn texture_fits_uv_budget(has_texture: bool, info: TextureInfo, uv_set_count: u32) -> bool {
    if !has_texture {
        return true;
    }

    // Reject textures that reference UV sets the mesh never uploaded.
    return info.attribute_uv_set_index < uv_set_count;
}

fn get_triangle_indices(attribute_indices_offset: u32, triangle_index: u32) -> vec3<u32> {
    let base = attribute_indices_offset + (triangle_index * 3u);
    return vec3<u32>(
        attribute_indices[base],
        attribute_indices[base + 1u],
        attribute_indices[base + 2u],
    );
}

// Matches `MegaTexture::new` padding. Keep both in sync.
const ATLAS_PADDING: f32 = 8.0;

fn compute_texture_lod(
    tex_info: TextureInfo,
    coords: vec2<i32>,
    triangle_indices_current: vec3<u32>,
    triangle_index: u32,
    barycentric: vec3<f32>,
    attribute_indices_offset: u32,
    attribute_data_offset: u32,
    vertex_attribute_stride: u32,
    screen_dims: vec2<i32>,
    texture_enabled: bool,
) -> f32 {
    if (!texture_enabled) {
        return 0.0;
    }

    let uv_center = texture_uv(
        attribute_data_offset,
        triangle_indices_current,
        barycentric,
        tex_info,
        vertex_attribute_stride,
    );

    let uv_right = sample_neighbor_uv(
        coords,
        vec2<i32>(1, 0),
        tex_info,
        triangle_index,
        attribute_indices_offset,
        attribute_data_offset,
        vertex_attribute_stride,
        screen_dims,
        uv_center,
    );

    let uv_up = sample_neighbor_uv(
        coords,
        vec2<i32>(0, 1),
        tex_info,
        triangle_index,
        attribute_indices_offset,
        attribute_data_offset,
        vertex_attribute_stride,
        screen_dims,
        uv_center,
    );

    let tex_size = max(
        vec2<f32>(f32(tex_info.size.x), f32(tex_info.size.y)),
        vec2<f32>(1.0, 1.0),
    );

    let uv_dx = (uv_right - uv_center) * tex_size;
    let uv_dy = (uv_up - uv_center) * tex_size;
    let gradient = max(length(uv_dx), length(uv_dy));
    let lod = log2(max(gradient, 1e-6));
    let max_mip = log2(max(f32(tex_info.size.x), f32(tex_info.size.y)));

    var clamped_lod = clamp(lod, 0.0, max_mip);
    let clamp_u = tex_info.address_mode_u == ADDRESS_MODE_CLAMP_TO_EDGE;
    let clamp_v = tex_info.address_mode_v == ADDRESS_MODE_CLAMP_TO_EDGE;
    let oob_u = clamp_u && (uv_center.x < 0.0 || uv_center.x > 1.0);
    let oob_v = clamp_v && (uv_center.y < 0.0 || uv_center.y > 1.0);
    if (oob_u || oob_v) {
        let max_clamp_lod = log2(ATLAS_PADDING);
        clamped_lod = min(clamped_lod, max_clamp_lod);
    }

    return clamped_lod;
}

fn sample_neighbor_uv(
    coords: vec2<i32>,
    offset: vec2<i32>,
    tex_info: TextureInfo,
    triangle_index: u32,
    attribute_indices_offset: u32,
    attribute_data_offset: u32,
    vertex_attribute_stride: u32,
    screen_dims: vec2<i32>,
    fallback_uv: vec2<f32>,
) -> vec2<f32> {
    let neighbor = coords + offset;
    if (neighbor.x < 0 || neighbor.y < 0 || neighbor.x >= screen_dims.x || neighbor.y >= screen_dims.y) {
        return fallback_uv;
    }

    let neighbor_visibility = textureLoad(visibility_data_tex, neighbor, 0);
    let neighbor_triangle_index = bitcast<u32>(neighbor_visibility.x);
    if (neighbor_triangle_index == f32_max || neighbor_triangle_index != triangle_index) {
        return fallback_uv;
    }

    let barycentric = vec3<f32>(
        neighbor_visibility.z,
        neighbor_visibility.w,
        1.0 - neighbor_visibility.z - neighbor_visibility.w,
    );
    let neighbor_triangle_indices =
        get_triangle_indices(attribute_indices_offset, neighbor_triangle_index);

    return texture_uv(
        attribute_data_offset,
        neighbor_triangle_indices,
        barycentric,
        tex_info,
        vertex_attribute_stride,
    );
}
