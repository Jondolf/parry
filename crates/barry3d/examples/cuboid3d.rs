use barry3d::math::Vector3;
use barry3d::shape::Cuboid;

fn main() {
    let cuboid = Cuboid::new(Vector3::new(2.0f32, 1.0, 3.0));

    assert!(cuboid.half_extents.x == 2.0);
    assert!(cuboid.half_extents.y == 1.0);
    assert!(cuboid.half_extents.z == 3.0);
}
