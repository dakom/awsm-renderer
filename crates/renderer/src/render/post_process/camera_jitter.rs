use glam::{Mat4, Vec2};

pub struct PostProcessCameraJitter {
    frame_index: u32,
    max_samples: u32,
    jitter_scale: f32,
}

impl PostProcessCameraJitter {
    pub fn new() -> Self {
        Self {
            frame_index: 0,
            max_samples: 16,    // Standard for AAA TAA - can be adjusted
            jitter_scale: 0.02, // Very subtle jitter - 2% of a pixel
        }
    }

    pub fn new_with_samples(max_samples: u32) -> Self {
        Self {
            frame_index: 0,
            max_samples,
            jitter_scale: 0.02,
        }
    }

    pub fn new_with_scale(max_samples: u32, jitter_scale: f32) -> Self {
        Self {
            frame_index: 0,
            max_samples,
            jitter_scale,
        }
    }

    /// Generate Halton sequence value for given index and base
    fn halton(index: u32, base: u32) -> f32 {
        let mut result = 0.0;
        let mut fraction = 1.0 / base as f32;
        let mut i = index;

        while i > 0 {
            result += (i % base) as f32 * fraction;
            i /= base;
            fraction /= base as f32;
        }

        result
    }

    /// Generate jitter offset using Halton sequence
    fn get_halton_jitter(&self, frame_index: u32) -> Vec2 {
        // Use bases 2 and 3 for X and Y to avoid correlation
        let x = Self::halton(frame_index, 2) - 0.5; // Center around 0
        let y = Self::halton(frame_index, 3) - 0.5; // Center around 0
        Vec2::new(x, y)
    }

    pub fn apply(&mut self, projection: &mut Mat4, screen_width: u32, screen_height: u32) {
        let sample_index = (self.frame_index % self.max_samples) + 1; // Start from 1 to avoid (0,0)
        let jitter = self.get_halton_jitter(sample_index);

        // Scale down the jitter to sub-pixel range (typically 0.5 to 1.0 pixel max)
        let jitter_scale = 0.25; // Adjust this value to control jitter strength
        let scaled_jitter = jitter * jitter_scale;

        let jitter_offset = scaled_jitter / Vec2::new(screen_width as f32, screen_height as f32);

        // projection.w_axis.x += jitter_offset.x * 2.0;
        // projection.w_axis.y += jitter_offset.y * 2.0;

        self.frame_index = self.frame_index.wrapping_add(1);
    }

    /// Reset the jitter sequence (useful for deterministic rendering)
    pub fn reset(&mut self) {
        self.frame_index = 0;
    }

    /// Get current frame index for external synchronization
    pub fn frame_index(&self) -> u32 {
        self.frame_index
    }

    /// Set maximum samples in the sequence
    pub fn set_max_samples(&mut self, max_samples: u32) {
        self.max_samples = max_samples;
    }
}
