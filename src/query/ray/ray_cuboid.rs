use crate::bounding_volume::Aabb;
use crate::math::Real;
use crate::query::{Ray, RayCast, RayIntersection};
use crate::shape::Cuboid;

impl RayCast for Cuboid {
    #[inline]
    fn cast_local_ray(&self, ray: &Ray, max_toi: Real, solid: bool) -> Option<Real> {
        let dl = -self.half_extents;
        let ur = self.half_extents;
        Aabb::new(dl, ur).cast_local_ray(ray, max_toi, solid)
    }

    #[inline]
    fn cast_local_ray_and_get_normal(
        &self,
        ray: &Ray,
        max_toi: Real,
        solid: bool,
    ) -> Option<RayIntersection> {
        let dl = -self.half_extents;
        let ur = self.half_extents;
        Aabb::new(dl, ur).cast_local_ray_and_get_normal(ray, max_toi, solid)
    }
}
