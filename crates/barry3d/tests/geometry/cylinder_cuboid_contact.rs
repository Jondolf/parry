use barry3d::math::{Isometry3, Vector3};
use barry3d::query;
use barry3d::shape::{Cuboid, Cylinder};

// Issue #157.
#[test]
fn cylinder_cuboid_contact() {
    let cyl = Cylinder::new(0.925, 0.5);
    let cyl_at = Isometry3::from_xyz(10.97, 0.925, 61.02);
    let cuboid = Cuboid::new(Vector3::new(0.05, 0.75, 0.5));
    let cuboid_at = Isometry3::from_xyz(11.50, 0.75, 60.5);
    let distance =
        query::details::distance_support_map_support_map(cyl_at.inv_mul(cuboid_at), &cyl, &cuboid);

    let intersecting = query::details::intersection_test_support_map_support_map(
        cyl_at.inv_mul(cuboid_at),
        &cyl,
        &cuboid,
    );

    let contact = query::details::contact_support_map_support_map(
        cyl_at.inv_mul(cuboid_at),
        &cyl,
        &cuboid,
        10.0,
    );

    assert!(distance == 0.0);
    assert!(intersecting);
    assert!(contact.is_some());
}
