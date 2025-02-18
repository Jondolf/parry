//! Bounding sphere.

use crate::bounding_volume::BoundingVolume;
use crate::math::{Isometry, Real, UnitVector, Vector};

#[cfg(feature = "rkyv")]
use rkyv::{bytecheck, CheckBytes};

/// A Bounding Sphere.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "bytemuck", derive(bytemuck::Pod, bytemuck::Zeroable))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Deserialize, rkyv::Serialize, CheckBytes),
    archive(as = "Self")
)]
#[derive(Debug, PartialEq, Copy, Clone)]
#[repr(C)]
pub struct BoundingSphere {
    pub center: Vector,
    pub radius: Real,
}

impl BoundingSphere {
    /// Creates a new bounding sphere.
    pub fn new(center: Vector, radius: Real) -> BoundingSphere {
        BoundingSphere { center, radius }
    }

    /// The bounding sphere center.
    #[inline]
    pub fn center(&self) -> Vector {
        self.center
    }

    /// The bounding sphere radius.
    #[inline]
    pub fn radius(&self) -> Real {
        self.radius
    }

    /// Transforms this bounding sphere by `m`.
    #[inline]
    pub fn transform_by(&self, m: Isometry) -> BoundingSphere {
        BoundingSphere::new(m.translation + self.center, self.radius)
    }
}

impl BoundingVolume for BoundingSphere {
    #[inline]
    fn center(&self) -> Vector {
        self.center()
    }

    #[inline]
    fn intersects(&self, other: &BoundingSphere) -> bool {
        // FIXME: refactor that with the code from narrow_phase::ball_ball::collide(...) ?
        let delta_pos = other.center - self.center;
        let distance_squared = delta_pos.length_squared();
        let sum_radius = self.radius + other.radius;

        distance_squared <= sum_radius * sum_radius
    }

    #[inline]
    fn contains(&self, other: &BoundingSphere) -> bool {
        let delta_pos = other.center - self.center;
        let distance = delta_pos.length();

        distance + other.radius <= self.radius
    }

    #[inline]
    fn merge(&mut self, other: &BoundingSphere) {
        if let Ok(dir) = UnitVector::new(other.center() - self.center()) {
            let s_center_dir = self.center.dot(*dir);
            let o_center_dir = other.center.dot(*dir);

            let right;
            let left;

            if s_center_dir + self.radius > o_center_dir + other.radius {
                right = self.center + *dir * self.radius;
            } else {
                right = other.center + *dir * other.radius;
            }

            if -s_center_dir + self.radius > -o_center_dir + other.radius {
                left = self.center - *dir * self.radius;
            } else {
                left = other.center - *dir * other.radius;
            }

            self.center = (left + right) / 2.0;
            self.radius = right.distance(self.center);
        } else if other.radius > self.radius {
            self.radius = other.radius
        }
    }

    #[inline]
    fn merged(&self, other: &BoundingSphere) -> BoundingSphere {
        let mut res = self.clone();

        res.merge(other);

        res
    }

    #[inline]
    fn loosen(&mut self, amount: Real) {
        assert!(amount >= 0.0, "The loosening margin must be positive.");
        self.radius = self.radius + amount
    }

    #[inline]
    fn loosened(&self, amount: Real) -> BoundingSphere {
        assert!(amount >= 0.0, "The loosening margin must be positive.");
        BoundingSphere::new(self.center, self.radius + amount)
    }

    #[inline]
    fn tighten(&mut self, amount: Real) {
        assert!(amount >= 0.0, "The tightening margin must be positive.");
        assert!(amount <= self.radius, "The tightening margin is to large.");
        self.radius = self.radius - amount
    }

    #[inline]
    fn tightened(&self, amount: Real) -> BoundingSphere {
        assert!(amount >= 0.0, "The tightening margin must be positive.");
        assert!(amount <= self.radius, "The tightening margin is to large.");
        BoundingSphere::new(self.center, self.radius - amount)
    }
}
