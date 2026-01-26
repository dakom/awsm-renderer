use crate::{camera::CameraMatrices, AwsmRenderer};

impl AwsmRenderer {
    /// Convenience helper to update non-GPU properties once per frame.
    ///
    /// Pair this with `render()` for a simple frame loop; for physics-heavy scenes,
    /// you may want to update transforms more frequently.
    pub fn update_all(
        &mut self,
        global_time_delta: f64,
        camera_matrices: CameraMatrices,
    ) -> crate::error::Result<()> {
        self.update_animations(global_time_delta)?;
        self.update_transforms();
        self.update_camera(camera_matrices)?;

        Ok(())
    }
}
