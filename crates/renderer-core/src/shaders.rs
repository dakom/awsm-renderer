pub struct ShaderModuleDescriptor<'a> {
    pub code: &'a str,
    pub label: Option<&'a str>,
}

impl<'a> ShaderModuleDescriptor<'a> {
    pub fn new(code: &'a str, label: Option<&'a str>) -> Self {
        Self { code, label }
    }
}

impl From<ShaderModuleDescriptor<'_>> for web_sys::GpuShaderModuleDescriptor {
    fn from(shader_code: ShaderModuleDescriptor) -> Self {
        let descriptor = web_sys::GpuShaderModuleDescriptor::new(shader_code.code);

        if let Some(label) = shader_code.label {
            descriptor.set_label(label);
        }

        descriptor
    }
}