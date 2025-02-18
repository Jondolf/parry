use crate::bounding_volume::SimdAabb;
use crate::math::{Isometry, Real, SimdBool, SimdReal, SimdVector, Vector, SIMD_WIDTH};
use crate::partitioning::{SimdBestFirstVisitStatus, SimdBestFirstVisitor};
use crate::query::{QueryDispatcher, Ray, SimdRay, TOI};
use crate::shape::{Shape, TypedSimdCompositeShape};
use crate::utils::DefaultStorage;
use simba::simd::{SimdBool as _, SimdPartialOrd, SimdValue};

/// Time Of Impact of a composite shape with any other shape, under translational movement.
pub fn time_of_impact_composite_shape_shape<D: ?Sized, G1: ?Sized>(
    dispatcher: &D,
    pos12: Isometry,
    vel12: Vector,
    g1: &G1,
    g2: &dyn Shape,
    max_toi: Real,
    stop_at_penetration: bool,
) -> Option<TOI>
where
    D: QueryDispatcher,
    G1: TypedSimdCompositeShape<QbvhStorage = DefaultStorage>,
{
    let mut visitor = TOICompositeShapeShapeBestFirstVisitor::new(
        dispatcher,
        pos12,
        vel12,
        g1,
        g2,
        max_toi,
        stop_at_penetration,
    );
    g1.typed_qbvh()
        .traverse_best_first(&mut visitor)
        .map(|res| res.1 .1)
}

/// Time Of Impact of any shape with a composite shape, under translational movement.
pub fn time_of_impact_shape_composite_shape<D: ?Sized, G2: ?Sized>(
    dispatcher: &D,
    pos12: Isometry,
    vel12: Vector,
    g1: &dyn Shape,
    g2: &G2,
    max_toi: Real,
    stop_at_penetration: bool,
) -> Option<TOI>
where
    D: QueryDispatcher,
    G2: TypedSimdCompositeShape<QbvhStorage = DefaultStorage>,
{
    time_of_impact_composite_shape_shape(
        dispatcher,
        pos12.inverse(),
        -(pos12.rotation.inverse() * vel12),
        g2,
        g1,
        max_toi,
        stop_at_penetration,
    )
    .map(|toi| toi.swapped())
}

/// A visitor used to find the time-of-impact between a composite shape and a shape.
pub struct TOICompositeShapeShapeBestFirstVisitor<'a, D: ?Sized, G1: ?Sized + 'a> {
    msum_shift: SimdVector,
    msum_margin: SimdVector,
    ray: SimdRay,

    dispatcher: &'a D,
    pos12: Isometry,
    vel12: Vector,
    g1: &'a G1,
    g2: &'a dyn Shape,
    max_toi: Real,
    stop_at_penetration: bool,
}

impl<'a, D: ?Sized, G1: ?Sized> TOICompositeShapeShapeBestFirstVisitor<'a, D, G1>
where
    D: QueryDispatcher,
    G1: TypedSimdCompositeShape<QbvhStorage = DefaultStorage>,
{
    /// Creates a new visitor used to find the time-of-impact between a composite shape and a shape.
    pub fn new(
        dispatcher: &'a D,
        pos12: Isometry,
        vel12: Vector,
        g1: &'a G1,
        g2: &'a dyn Shape,
        max_toi: Real,
        stop_at_penetration: bool,
    ) -> TOICompositeShapeShapeBestFirstVisitor<'a, D, G1> {
        let ls_aabb2 = g2.compute_aabb(pos12);
        let ray = Ray::new(Vector::ZERO, vel12);

        TOICompositeShapeShapeBestFirstVisitor {
            dispatcher,
            msum_shift: SimdVector::splat(-ls_aabb2.center()),
            msum_margin: SimdVector::splat(ls_aabb2.half_extents()),
            ray: SimdRay::splat(ray),
            pos12,
            vel12,
            g1,
            g2,
            max_toi,
            stop_at_penetration,
        }
    }
}

impl<'a, D: ?Sized, G1: ?Sized> SimdBestFirstVisitor<G1::PartId, SimdAabb>
    for TOICompositeShapeShapeBestFirstVisitor<'a, D, G1>
where
    D: QueryDispatcher,
    G1: TypedSimdCompositeShape<QbvhStorage = DefaultStorage>,
{
    type Result = (G1::PartId, TOI);

    #[inline]
    fn visit(
        &mut self,
        best: Real,
        bv: &SimdAabb,
        data: Option<[Option<&G1::PartId>; SIMD_WIDTH]>,
    ) -> SimdBestFirstVisitStatus<Self::Result> {
        // Compute the minkowski sum of the two Aabbs.
        let msum = SimdAabb {
            mins: bv.mins + self.msum_shift + (-self.msum_margin),
            maxs: bv.maxs + self.msum_shift + self.msum_margin,
        };

        // Compute the TOI.
        let (mask, toi) = msum.cast_local_ray(&self.ray, SimdReal::splat(self.max_toi));

        if let Some(data) = data {
            let better_toi = toi.simd_lt(SimdReal::splat(best));
            let bitmask = (mask & better_toi).bitmask();
            let mut weights = [0.0; SIMD_WIDTH];
            let mut mask = [false; SIMD_WIDTH];
            let mut results = [None; SIMD_WIDTH];

            for ii in 0..SIMD_WIDTH {
                if (bitmask & (1 << ii)) != 0 && data[ii].is_some() {
                    let part_id = *data[ii].unwrap();
                    let mut toi = None;
                    self.g1.map_untyped_part_at(part_id, |part_pos1, g1| {
                        if let Some(part_pos1) = part_pos1 {
                            toi = self
                                .dispatcher
                                .time_of_impact(
                                    part_pos1.inv_mul(self.pos12),
                                    part_pos1.rotation.inverse() * self.vel12,
                                    g1,
                                    self.g2,
                                    self.max_toi,
                                    self.stop_at_penetration,
                                )
                                .unwrap_or(None)
                                .map(|toi| toi.transform1_by(part_pos1));
                        } else {
                            toi = self
                                .dispatcher
                                .time_of_impact(
                                    self.pos12,
                                    self.vel12,
                                    g1,
                                    self.g2,
                                    self.max_toi,
                                    self.stop_at_penetration,
                                )
                                .unwrap_or(None);
                        }
                    });

                    if let Some(toi) = toi {
                        results[ii] = Some((part_id, toi));
                        mask[ii] = toi.toi < best;
                        weights[ii] = toi.toi;
                    }
                }
            }

            SimdBestFirstVisitStatus::MaybeContinue {
                weights: SimdReal::from(weights),
                mask: SimdBool::from(mask),
                results,
            }
        } else {
            SimdBestFirstVisitStatus::MaybeContinue {
                weights: toi,
                mask,
                results: [None; SIMD_WIDTH],
            }
        }
    }
}
