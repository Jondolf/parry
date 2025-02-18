use crate::bounding_volume::BoundingSphere;
use crate::math::{Isometry, Vector};
use crate::shape::Cone;

impl Cone {
    /// Computes the world-space bounding sphere of this cone, transformed by `pos`.
    #[inline]
    pub fn bounding_sphere(&self, pos: Isometry) -> BoundingSphere {
        let bv: BoundingSphere = self.local_bounding_sphere();
        bv.transform_by(pos)
    }

    /// Computes the local-space bounding sphere of this cone.
    #[inline]
    pub fn local_bounding_sphere(&self) -> BoundingSphere {
        let radius = (self.radius.powi(2) + self.half_height.powi(2)).sqrt();

        BoundingSphere::new(Vector::ZERO, radius)
    }
}
