use askama::Template;

#[derive(Template, Debug, Default)]
#[template(path = "vertex/quad.wgsl", whitespace = "minimize")]
pub struct ShaderTemplateVertexQuad {
}

impl ShaderTemplateVertexQuad {
    pub fn new() -> Self {
        Self::default()
    }
}
