use crate::{camera::CameraExt, AwsmRenderer};

impl AwsmRenderer {
    // just a convenience function to update non-GPU properties
    // pair this with .render() once a frame and everything should run smoothly
    // but real-world you may want to update transforms more often for physics, for example
    pub fn update_all(
        &mut self,
        global_time_delta: f64,
        camera: &impl CameraExt,
    ) -> crate::error::Result<()> {
        self.update_animations(global_time_delta)?;
        self.update_transforms();
        self.update_skins();
        self.update_camera(camera)?;

        Ok(())
    }
}
