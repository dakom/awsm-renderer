{% include "geometry_and_transparency_wgsl/vertex/meta.wgsl" %}
{% include "geometry_and_transparency_wgsl/vertex/camera.wgsl" %}
{% include "geometry_and_transparency_wgsl/vertex/transform.wgsl" %}
{% include "geometry_and_transparency_wgsl/vertex/morph.wgsl" %}
{% include "geometry_and_transparency_wgsl/vertex/skin.wgsl" %}
{% include "geometry_and_transparency_wgsl/vertex/apply.wgsl" %}


//***** MAIN *****
@vertex
fn vert_main(input: VertexInput) -> VertexOutput {
    return apply_vertex(input);
}
