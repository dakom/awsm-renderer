pub struct ShaderCode<'a> {
    pub code: &'a str,
    pub label: Option<&'a str>,
}

impl<'a> ShaderCode<'a> {
    pub fn new(code: &'a str, label: Option<&'a str>) -> Self {
        Self { code, label }
    }
}

impl From<ShaderCode<'_>> for web_sys::GpuShaderModuleDescriptor {
    fn from(shader_code: ShaderCode) -> Self {
        let descriptor = web_sys::GpuShaderModuleDescriptor::new(shader_code.code);

        if let Some(label) = shader_code.label {
            descriptor.set_label(label);
        }

        descriptor
    }
}
