{% if multisampled_geometry %}
    @group(0) @binding(0) var material_tex: texture_multisampled_2d<f32>;
{% else %}
    @group(0) @binding(0) var material_tex: texture_2d<f32>;
{% endif %}

@group(0) @binding(1) var composite_tex: texture_storage_2d<rgba16float, write>;
