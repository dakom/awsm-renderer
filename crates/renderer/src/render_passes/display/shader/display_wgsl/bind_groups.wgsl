@group(0) @binding(0) var composite_texture: texture_2d<f32>;
{% if multisampled_geometry %}
    @group(0) @binding(1) var depth_tex: texture_depth_multisampled_2d;
{% else %}
    @group(0) @binding(1) var depth_tex: texture_depth_2d;
{% endif %}
