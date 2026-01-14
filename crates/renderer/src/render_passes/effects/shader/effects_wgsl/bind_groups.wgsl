@group(0) @binding(0) var composite_tex: texture_2d<f32>;
{% if multisampled_geometry %}
    @group(0) @binding(1) var depth_tex: texture_depth_multisampled_2d;
{% else %}
    @group(0) @binding(1) var depth_tex: texture_depth_2d;
{% endif %}
@group(0) @binding(2) var effects_tex: texture_storage_2d<rgba16float, write>;
