use barry3d::math::{Isometry3, Vector3};
use barry3d::query::{Ray, RayCast};
use barry3d::shape::Cuboid;

fn main() {
    let cube = Cuboid::new(Vector3::new(1.0f32, 1.0, 1.0));
    let ray = Ray::new(Vector3::new(0.0f32, 0.0, -1.0), Vector3::Z);

    assert!(cube.intersects_ray(Isometry3::IDENTITY, &ray, std::f32::MAX));
}
