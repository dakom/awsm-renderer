{% include "util/math.wgsl" %}
{% include "camera.wgsl" %}

{% if morphs.any() %}
    {% include "vertex/morph.wgsl" %}
{% endif %}

{% if skins > 0 %}
    {% include "vertex/skin.wgsl" %}
{% endif %}

{% include "vertex/mesh.wgsl" %}

{% include "fragment/pbr.wgsl" %}