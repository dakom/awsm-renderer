{% include "util/math.wgsl" %}
{% include "camera.wgsl" %}

{% if has_morphs %}
    {% include "vertex/morph.wgsl" %}
{% endif %}

{% if has_skins %}
    {% include "vertex/skin.wgsl" %}
{% endif %}

{% include "vertex/mesh.wgsl" %}

{% include "fragment/pbr.wgsl" %}