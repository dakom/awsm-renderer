{% include "util/identity.wgsl" %}
{% include "camera.wgsl" %}

{% match vertex_shader_kind %}
    {% when VertexShaderKind::Mesh %}
        {% include "vertex/mesh.wgsl" %}
    {% when VertexShaderKind::Quad %}
        {% include "vertex/quad.wgsl" %}
    {% when _ %}
{% endmatch %}
{% include "fragment/input/fragment_input.wgsl" %}

{% match fragment_shader_kind %}
    {% when FragmentShaderKind::DebugNormals %}
        {% include "fragment/debug_normals.wgsl" %}
    {% when FragmentShaderKind::Pbr %}
        {% include "fragment/pbr.wgsl" %}
    {% when _ %}
{% endmatch %}