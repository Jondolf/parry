// https://github.com/dimforge/barry/issues/242

use barry3d::math::{Isometry3, Rotation3, Vector3};
use barry3d::query::Ray;
use barry3d::shape::{Ball, Cuboid, Shape};
use bevy_math::Quat;

fn run_test<S>(name: &str, shape: S)
where
    S: Shape,
{
    let mut rng = oorandom::Rand32::new(42);

    for _ in 0..1000 {
        let ray_origin =
            Vector3::new(rng.rand_float(), rng.rand_float(), rng.rand_float()).normalize() * 5.0;
        let ray = Ray::new(ray_origin, Vector3::ZERO - ray_origin);

        let rotation = if rng.rand_float() < 0.01 {
            Quat::IDENTITY
        } else {
            Quat::from_xyzw(
                rng.rand_float(),
                rng.rand_float(),
                rng.rand_float(),
                rng.rand_float(),
            )
            .normalize()
        };
        let position = Isometry3::from_rotation(Rotation3(rotation));

        let intersection = shape
            .cast_ray_and_get_normal(position, &ray, std::f32::MAX, true)
            .expect(&format!(
                "Ray {:?} did not hit Shape {} rotated with {:?}",
                ray, name, rotation
            ));

        let point = ray.origin + ray.dir * intersection.toi;
        let point_nudged_in = point + intersection.normal * -0.001;
        let point_nudged_out = point + intersection.normal * 0.001;

        assert!(
            shape.contains_point(position, point_nudged_in),
            "Shape {} rotated with {:#?} does not contain point nudged in {:#?}",
            name,
            rotation.to_axis_angle().0,
            point_nudged_in,
        );

        assert!(
            !shape.contains_point(position, point_nudged_out),
            "Shape {} rotated with {:#?} does contains point nudged out {:#?}",
            name,
            rotation.to_axis_angle().0,
            point_nudged_out,
        );

        let new_ray = Ray::new(point_nudged_out, ray_origin - point_nudged_out);

        assert!(
            shape
                .cast_ray_and_get_normal(position, &new_ray, std::f32::MAX, true)
                .is_none(),
            "Ray {:#?} from outside Shape {} rotated with {:#?} did hit at t={}",
            ray,
            name,
            rotation,
            shape
                .cast_ray_and_get_normal(position, &new_ray, std::f32::MAX, true)
                .expect("recurring ray cast produced a different answer")
                .toi
        );
    }
}

#[test]
fn shape_ray_cast_points_to_surface() {
    run_test("ball with radius 1", Ball::new(1.0));
    run_test(
        "cube with half-side 1",
        Cuboid::new(Vector3::new(1.0, 1.0, 1.0)),
    );
    run_test("tall rectangle", Cuboid::new(Vector3::new(1.0, 1.0, 0.5)));
    run_test(
        "tall and slim rectangle",
        Cuboid::new(Vector3::new(0.5, 1.0, 0.5)),
    );
}
