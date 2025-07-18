{% include "util/identity.wgsl" %}
{% include "camera.wgsl" %}

{% match vertex_shader_kind %}
    {% when VertexShaderKind::Mesh %}
        {% include "vertex/mesh.wgsl" %}
        {% include "fragment/input/mesh_input.wgsl" %}
    {% when VertexShaderKind::Quad %}
        {% include "vertex/quad.wgsl" %}
        {% include "fragment/input/quad_input.wgsl" %}
    {% when _ %}
{% endmatch %}

{% match fragment_shader_kind %}
    {% when FragmentShaderKind::DebugNormals %}
        {% include "fragment/debug_normals.wgsl" %}
    {% when FragmentShaderKind::Pbr %}
        {% include "fragment/pbr.wgsl" %}
    {% when FragmentShaderKind::PostProcess %}
        {% include "fragment/post_process.wgsl" %}
    {% when _ %}
{% endmatch %}