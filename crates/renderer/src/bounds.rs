use glam::{Mat4, Vec3};

// Axis-Aligned Bounding Box (AABB) structure
#[derive(Debug, Clone)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn extend(&mut self, other: &Self) {
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
    }

    pub fn transform(&mut self, mat: &Mat4) {
        self.min = mat.transform_point3(self.min);
        self.max = mat.transform_point3(self.max);
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }
}
