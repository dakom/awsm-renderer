@group(0) @binding(0) var opaque_tex: texture_2d<f32>;

{% if multisampled_geometry %}
    @group(0) @binding(1) var oit_color_tex: texture_multisampled_2d<f32>;
{% else %}
    @group(0) @binding(1) var oit_color_tex: texture_2d<f32>;
{% endif %}

@group(0) @binding(2) var composite_tex: texture_storage_2d<rgba8unorm, write>;
