pub struct AntiAliasing {
    // if None, no MSAA
    pub msaa_sample_count: Option<u8>,
    pub smaa: bool,
}

impl Default for AntiAliasing {
    fn default() -> Self {
        Self {
            msaa_sample_count: Some(4),
            smaa: true,
        }
    }
}
