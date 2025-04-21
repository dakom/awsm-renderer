{% include "camera.wgsl" %}

{% if has_morphs %}
    {% include "vertex/morph.wgsl" %}
{% endif %}

{% include "vertex/mesh.wgsl" %}

{% include "fragment/pbr.wgsl" %}