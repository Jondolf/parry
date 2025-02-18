use crate::math::{Isometry, Real, UnitVector, Vector};
use crate::query::gjk::{self, CSOPoint, GJKResult, VoronoiSimplex};
use crate::shape::SupportMap;

use num::Bounded;

/// Distance between support-mapped shapes.
pub fn distance_support_map_support_map<G1: ?Sized, G2: ?Sized>(
    pos12: Isometry,
    g1: &G1,
    g2: &G2,
) -> Real
where
    G1: SupportMap,
    G2: SupportMap,
{
    distance_support_map_support_map_with_params(pos12, g1, g2, &mut VoronoiSimplex::new(), None)
}

/// Distance between support-mapped shapes.
///
/// This allows a more fine grained control other the underlying GJK algorigtm.
pub fn distance_support_map_support_map_with_params<G1: ?Sized, G2: ?Sized>(
    pos12: Isometry,
    g1: &G1,
    g2: &G2,
    simplex: &mut VoronoiSimplex,
    init_dir: Option<Vector>,
) -> Real
where
    G1: SupportMap,
    G2: SupportMap,
{
    // FIXME: or m2.translation - m1.translation ?
    let dir = init_dir.unwrap_or_else(|| -pos12.translation);

    if let Ok(dir) = UnitVector::new(dir) {
        simplex.reset(CSOPoint::from_shapes(pos12, g1, g2, dir));
    } else {
        simplex.reset(CSOPoint::from_shapes(pos12, g1, g2, UnitVector::X));
    }

    match gjk::closest_points(pos12, g1, g2, Real::max_value(), true, simplex) {
        GJKResult::Intersection => 0.0,
        GJKResult::ClosestPoints(p1, p2, _) => p1.distance(p2),
        GJKResult::Proximity(_) => unreachable!(),
        GJKResult::NoIntersection(_) => 0.0, // FIXME: GJK did not converge.
    }
}
