/*************** START math.wgsl ******************/
{% include "utils_wgsl/math.wgsl" %}
/*************** END math.wgsl ******************/

/*************** START meta.wgsl ******************/
{% include "geometry_and_transparency_wgsl/vertex/geometry_mesh_meta.wgsl" %}
/*************** END meta.wgsl ******************/

/*************** START camera.wgsl ******************/
{% include "geometry_and_transparency_wgsl/vertex/camera.wgsl" %}
/*************** END camera.wgsl ******************/

/*************** START transform.wgsl ******************/
{% include "geometry_and_transparency_wgsl/vertex/transform.wgsl" %}
/*************** END transform.wgsl ******************/

/*************** START morph.wgsl ******************/
{% include "geometry_and_transparency_wgsl/vertex/morph.wgsl" %}
/*************** END morph.wgsl ******************/

/*************** START skin.wgsl ******************/
{% include "geometry_and_transparency_wgsl/vertex/skin.wgsl" %}
/*************** END skin.wgsl ******************/

/*************** START apply.wgsl ******************/
{% include "geometry_and_transparency_wgsl/vertex/apply_vertex.wgsl" %}
/*************** END apply.wgsl ******************/

/*************** START vertex_color.wgsl ******************/
{% include "opaque_and_transparency_wgsl/vertex_color.wgsl" %}
/*************** END vertex_color.wgsl ******************/

/*************** START textures.wgsl ******************/
{% include "opaque_and_transparency_wgsl/textures.wgsl" %}
/*************** END textures.wgsl ******************/

/*************** START material.wgsl ******************/
{% include "opaque_and_transparency_wgsl/pbr/material.wgsl" %}
/*************** END material.wgsl ******************/

/*************** START material.wgsl ******************/
{% include "opaque_and_transparency_wgsl/pbr/material_color.wgsl" %}
/*************** END material.wgsl ******************/

/*************** START mesh_meta.wgsl ******************/
{% include "opaque_and_transparency_wgsl/material_mesh_meta.wgsl" %}
/*************** END mesh_meta.wgsl ******************/

/*************** START lights.wgsl ******************/
{% include "opaque_and_transparency_wgsl/pbr/lighting/lights.wgsl" %}
/*************** END lights.wgsl ******************/

/*************** START brdf.wgsl ******************/
{% include "opaque_and_transparency_wgsl/pbr/lighting/brdf.wgsl" %}
/*************** END brdf.wgsl ******************/

/*************** START unlit.wgsl ******************/
{% include "opaque_and_transparency_wgsl/pbr/lighting/unlit.wgsl" %}
/*************** END unlit.wgsl ******************/

/*************** START texture_uvs.wgsl ******************/
{% include "material_transparent_wgsl/helpers/texture_uvs.wgsl" %}
/*************** END texture_uvs.wgsl ******************/

/*************** START material_shading.wgsl ******************/
{% include "material_transparent_wgsl/helpers/material_shading.wgsl" %}
/*************** END material_shading.wgsl ******************/

/*************** START material_color.wgsl ******************/
{% include "material_transparent_wgsl/helpers/material_color_calc.wgsl" %}
/*************** END material_color.wgsl ******************/

/*************** START vertex_color_attrib.wgsl ******************/
{% include "material_transparent_wgsl/helpers/vertex_color_attrib.wgsl" %}
/*************** END vertex_color_attrib.wgsl ******************/
