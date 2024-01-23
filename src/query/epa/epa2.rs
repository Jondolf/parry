//! Two-dimensional penetration depth queries using the Expanding Polytope Algorithm.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use num::Bounded;

use crate::math::{Isometry, Real, UnitVector, Vector};
use crate::query::gjk::{self, CSOPoint, ConstantOrigin, VoronoiSimplex};
use crate::shape::SupportMap;
use crate::utils;

#[derive(Copy, Clone, PartialEq)]
struct FaceId {
    id: usize,
    neg_dist: Real,
}

impl FaceId {
    fn new(id: usize, neg_dist: Real) -> Option<Self> {
        if neg_dist > gjk::EPS_TOLERANCE {
            None
        } else {
            Some(FaceId { id, neg_dist })
        }
    }
}

impl Eq for FaceId {}

impl PartialOrd for FaceId {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.neg_dist.partial_cmp(&other.neg_dist)
    }
}

impl Ord for FaceId {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        if self.neg_dist < other.neg_dist {
            Ordering::Less
        } else if self.neg_dist > other.neg_dist {
            Ordering::Greater
        } else {
            Ordering::Equal
        }
    }
}

#[derive(Clone, Debug)]
struct Face {
    pts: [usize; 2],
    normal: UnitVector,
    proj: Vector,
    bcoords: [Real; 2],
    deleted: bool,
}

impl Face {
    pub fn new(vertices: &[CSOPoint], pts: [usize; 2]) -> (Self, bool) {
        if let Some((proj, bcoords)) =
            project_origin(vertices[pts[0]].point, vertices[pts[1]].point)
        {
            (Self::new_with_proj(vertices, proj, bcoords, pts), true)
        } else {
            (
                Self::new_with_proj(vertices, Vector::ZERO, [0.0; 2], pts),
                false,
            )
        }
    }

    pub fn new_with_proj(
        vertices: &[CSOPoint],
        proj: Vector,
        bcoords: [Real; 2],
        pts: [usize; 2],
    ) -> Self {
        let normal;
        let deleted;

        if let Ok(n) = utils::ccw_face_normal([vertices[pts[0]].point, vertices[pts[1]].point]) {
            normal = n;
            deleted = false;
        } else {
            normal = UnitVector::new_unchecked(Vector::ZERO);
            deleted = true;
        }

        Face {
            pts,
            normal,
            proj,
            bcoords,
            deleted,
        }
    }

    pub fn closest_points(&self, vertices: &[CSOPoint]) -> (Vector, Vector) {
        (
            vertices[self.pts[0]].orig1 * self.bcoords[0]
                + vertices[self.pts[1]].orig1 * self.bcoords[1],
            vertices[self.pts[0]].orig2 * self.bcoords[0]
                + vertices[self.pts[1]].orig2 * self.bcoords[1],
        )
    }
}

/// The Expanding Polytope Algorithm in 2D.
pub struct EPA {
    vertices: Vec<CSOPoint>,
    faces: Vec<Face>,
    heap: BinaryHeap<FaceId>,
}

impl EPA {
    /// Creates a new instance of the 2D Expanding Polytope Algorithm.
    pub fn new() -> Self {
        EPA {
            vertices: Vec::new(),
            faces: Vec::new(),
            heap: BinaryHeap::new(),
        }
    }

    fn reset(&mut self) {
        self.vertices.clear();
        self.faces.clear();
        self.heap.clear();
    }

    /// Projects the origin on boundary the given shape.
    ///
    /// The origin is assumed to be inside of the shape. If it is outside, use
    /// the GJK algorithm instead.
    ///
    /// Return `None` if the origin is not inside of the shape or if
    /// the EPA algorithm failed to compute the projection.
    ///
    /// Return the projected point in the local-space of `g`.
    pub fn project_origin<G: ?Sized>(
        &mut self,
        m: Isometry,
        g: &G,
        simplex: &VoronoiSimplex,
    ) -> Option<Vector>
    where
        G: SupportMap,
    {
        self.closest_points(m.inverse(), g, &ConstantOrigin, simplex)
            .map(|(p, _, _)| p)
    }

    /// Projects the origin on a shape using the EPA algorithm.
    ///
    /// The origin is assumed to be located inside of the shape.
    /// Returns `None` if the EPA fails to converge or if `g1` and `g2` are not penetrating.
    pub fn closest_points<G1: ?Sized, G2: ?Sized>(
        &mut self,
        pos12: Isometry,
        g1: &G1,
        g2: &G2,
        simplex: &VoronoiSimplex,
    ) -> Option<(Vector, Vector, UnitVector)>
    where
        G1: SupportMap,
        G2: SupportMap,
    {
        let _eps: Real = crate::math::DEFAULT_EPSILON;
        let _eps_tol = _eps * 100.0;

        self.reset();

        /*
         * Initialization.
         */
        for i in 0..simplex.dimension() + 1 {
            self.vertices.push(*simplex.point(i));
        }

        if simplex.dimension() == 0 {
            const MAX_ITERS: usize = 100; // If there is no convergence, just use whatever direction was extracted so fare

            // The contact is vertex-vertex.
            // We need to determine a valid normal that lies
            // on both vertices' normal cone.
            let mut n = UnitVector::Y;

            // First, find a vector on the first vertex tangent cone.
            let orig1 = self.vertices[0].orig1;
            for _ in 0..MAX_ITERS {
                let supp1 = g1.local_support_point(*n);
                if let Ok(tangent) = UnitVector::new_with_min(supp1 - orig1, _eps_tol) {
                    if n.dot(*tangent) < _eps_tol {
                        break;
                    }

                    n = UnitVector::new_unchecked(Vector::new(-tangent.y, tangent.x));
                } else {
                    break;
                }
            }

            // Second, ensure the direction lies on the second vertex's tangent cone.
            let orig2 = self.vertices[0].orig2;
            for _ in 0..MAX_ITERS {
                let supp2 = g2.support_point(pos12, *-n);
                if let Ok(tangent) = UnitVector::new_with_min(supp2 - orig2, _eps_tol) {
                    if (-n).dot(*tangent) < _eps_tol {
                        break;
                    }

                    n = UnitVector::new_unchecked(Vector::new(-tangent.y, tangent.x));
                } else {
                    break;
                }
            }

            return Some((Vector::ZERO, Vector::ZERO, n));
        } else if simplex.dimension() == 2 {
            let dp1 = self.vertices[1] - self.vertices[0];
            let dp2 = self.vertices[2] - self.vertices[0];

            if dp1.perp_dot(dp2) < 0.0 {
                self.vertices.swap(1, 2)
            }

            let pts1 = [0, 1];
            let pts2 = [1, 2];
            let pts3 = [2, 0];

            let (face1, proj_is_inside1) = Face::new(&self.vertices, pts1);
            let (face2, proj_is_inside2) = Face::new(&self.vertices, pts2);
            let (face3, proj_is_inside3) = Face::new(&self.vertices, pts3);

            self.faces.push(face1);
            self.faces.push(face2);
            self.faces.push(face3);

            if proj_is_inside1 {
                let dist1 = self.faces[0].normal.dot(self.vertices[0].point);
                self.heap.push(FaceId::new(0, -dist1)?);
            }

            if proj_is_inside2 {
                let dist2 = self.faces[1].normal.dot(self.vertices[1].point);
                self.heap.push(FaceId::new(1, -dist2)?);
            }

            if proj_is_inside3 {
                let dist3 = self.faces[2].normal.dot(self.vertices[2].point);
                self.heap.push(FaceId::new(2, -dist3)?);
            }
        } else {
            let pts1 = [0, 1];
            let pts2 = [1, 0];

            self.faces.push(Face::new_with_proj(
                &self.vertices,
                Vector::ZERO,
                [1.0, 0.0],
                pts1,
            ));
            self.faces.push(Face::new_with_proj(
                &self.vertices,
                Vector::ZERO,
                [1.0, 0.0],
                pts2,
            ));

            let dist1 = self.faces[0].normal.dot(self.vertices[0].point);
            let dist2 = self.faces[1].normal.dot(self.vertices[1].point);

            self.heap.push(FaceId::new(0, dist1)?);
            self.heap.push(FaceId::new(1, dist2)?);
        }

        let mut niter = 0;
        let mut max_dist = Real::max_value();
        let mut best_face_id = *self.heap.peek().unwrap();

        /*
         * Run the expansion.
         */
        while let Some(face_id) = self.heap.pop() {
            // Create new faces.
            let face = self.faces[face_id.id].clone();

            if face.deleted {
                continue;
            }

            let cso_point = CSOPoint::from_shapes(pos12, g1, g2, face.normal);
            let support_point_id = self.vertices.len();
            self.vertices.push(cso_point);

            let candidate_max_dist = cso_point.point.dot(*face.normal);

            if candidate_max_dist < max_dist {
                best_face_id = face_id;
                max_dist = candidate_max_dist;
            }

            let curr_dist = -face_id.neg_dist;

            if max_dist - curr_dist < _eps_tol {
                let best_face = &self.faces[best_face_id.id];
                let cpts = best_face.closest_points(&self.vertices);
                return Some((cpts.0, cpts.1, best_face.normal));
            }

            let pts1 = [face.pts[0], support_point_id];
            let pts2 = [support_point_id, face.pts[1]];

            let new_faces = [
                Face::new(&self.vertices, pts1),
                Face::new(&self.vertices, pts2),
            ];

            for f in new_faces.iter() {
                if f.1 {
                    let dist = f.0.normal.dot(f.0.proj);
                    if dist < curr_dist {
                        // FIXME: if we reach this point, there were issues due to
                        // numerical errors.
                        let cpts = f.0.closest_points(&self.vertices);
                        return Some((cpts.0, cpts.1, f.0.normal));
                    }

                    if !f.0.deleted {
                        self.heap.push(FaceId::new(self.faces.len(), -dist)?);
                    }
                }

                self.faces.push(f.0.clone());
            }

            niter += 1;
            if niter > 10000 {
                return None;
            }
        }

        let best_face = &self.faces[best_face_id.id];
        let cpts = best_face.closest_points(&self.vertices);
        Some((cpts.0, cpts.1, best_face.normal))
    }
}

fn project_origin(a: Vector, b: Vector) -> Option<(Vector, [Real; 2])> {
    let ab = b - a;
    let ap = -a;
    let ab_ap = ab.dot(ap);
    let sqnab = ab.length_squared();

    if sqnab == 0.0 {
        return None;
    }

    let position_on_segment;

    let _eps: Real = gjk::EPS_TOLERANCE;

    if ab_ap < -_eps || ab_ap > sqnab + _eps {
        // Voronoï region of vertex 'a' or 'b'.
        None
    } else {
        // Voronoï region of the segment interior.
        position_on_segment = ab_ap / sqnab;

        let res = a + ab * position_on_segment;

        Some((res, [1.0 - position_on_segment, position_on_segment]))
    }
}
