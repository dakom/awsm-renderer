{% include "camera.wgsl" %}

{% if has_morphs %}
    {% include "vertex/morph.wgsl" %}
{% endif %}

{% if skin_joint_sets > 0 %}
    {% include "vertex/skin.wgsl" %}
{% endif %}

{% include "vertex/mesh.wgsl" %}

{% include "fragment/pbr.wgsl" %}