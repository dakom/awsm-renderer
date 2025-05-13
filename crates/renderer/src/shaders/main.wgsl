{% include "util/identity.wgsl" %}
{% include "camera.wgsl" %}

{% include "vertex/mesh.wgsl" %}
{% include "fragment/input/fragment_input.wgsl" %}

{% match fragment_shader_kind %}
    {% when FragmentShaderKind::DebugNormals %}
        {% include "fragment/debug_normals.wgsl" %}
    {% when FragmentShaderKind::Pbr %}
        {% include "fragment/pbr.wgsl" %}
    {% when _ %}
{% endmatch %}