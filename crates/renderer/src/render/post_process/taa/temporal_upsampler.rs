pub struct TemporalUpsampler {
    pub scale: f32,
}

impl Default for TemporalUpsampler {
    fn default() -> Self {
        Self { scale: 0.75 }
    }
}
