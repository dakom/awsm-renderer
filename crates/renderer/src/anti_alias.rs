use crate::{bind_groups::BindGroupCreate, AwsmRenderer};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AntiAliasing {
    // if None, no MSAA
    pub msaa_sample_count: Option<u32>,
    pub smaa: bool,
}

impl Default for AntiAliasing {
    fn default() -> Self {
        Self {
            msaa_sample_count: Some(4),
            smaa: false,
        }
    }
}

impl AwsmRenderer {
    pub fn set_anti_aliasing(&mut self, aa: AntiAliasing) {
        self.anti_aliasing = aa;
        self.bind_groups
            .mark_create(BindGroupCreate::AntiAliasingChange);
    }
}
