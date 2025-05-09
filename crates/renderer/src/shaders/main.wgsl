{% include "util/math.wgsl" %}
{% include "camera.wgsl" %}

{% include "vertex/mesh.wgsl" %}
{% include "fragment/fragment_input.wgsl" %}

{% match fragment_shader_kind %}
    {% when FragmentShaderKind::DebugNormals %}
        {% include "fragment/debug_normals.wgsl" %}
    {% when FragmentShaderKind::Pbr %}
        {% include "fragment/pbr.wgsl" %}
    {% when _ %}
{% endmatch %}