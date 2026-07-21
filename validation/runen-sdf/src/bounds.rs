use glam::Vec3;

use crate::error::{ValidationError, ensure_finite_vec3, ensure_non_negative};

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct Bounds3 {
    min: Vec3,
    max: Vec3,
}

impl Bounds3 {
    pub fn try_new(min: Vec3, max: Vec3) -> Result<Self, ValidationError> {
        ensure_finite_vec3(min, "bounds min")?;
        ensure_finite_vec3(max, "bounds max")?;
        if !min.cmple(max).all() {
            return Err(ValidationError::InvalidBounds);
        }
        Ok(Self { min, max })
    }

    pub fn from_center_half_extents(
        center: Vec3,
        half_extents: Vec3,
    ) -> Result<Self, ValidationError> {
        ensure_finite_vec3(center, "bounds center")?;
        ensure_finite_vec3(half_extents, "bounds half extents")?;
        if !half_extents.cmpge(Vec3::ZERO).all() {
            return Err(ValidationError::InvalidBounds);
        }
        Self::try_new(center - half_extents, center + half_extents)
    }

    pub const fn min(self) -> Vec3 {
        self.min
    }

    pub const fn max(self) -> Vec3 {
        self.max
    }

    pub fn center(self) -> Vec3 {
        self.min + (self.max - self.min) * 0.5
    }

    pub fn half_extents(self) -> Vec3 {
        (self.max - self.min) * 0.5
    }

    pub fn contains_point(self, point: Vec3) -> bool {
        point.is_finite() && point.cmpge(self.min).all() && point.cmple(self.max).all()
    }

    pub fn union(self, other: Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }

    pub fn intersection(self, other: Self) -> Option<Self> {
        let min = self.min.max(other.min);
        let max = self.max.min(other.max);
        min.cmple(max).all().then_some(Self { min, max })
    }

    pub fn corners(self) -> [Vec3; 8] {
        [
            Vec3::new(self.min.x, self.min.y, self.min.z),
            Vec3::new(self.min.x, self.min.y, self.max.z),
            Vec3::new(self.min.x, self.max.y, self.min.z),
            Vec3::new(self.min.x, self.max.y, self.max.z),
            Vec3::new(self.max.x, self.min.y, self.min.z),
            Vec3::new(self.max.x, self.min.y, self.max.z),
            Vec3::new(self.max.x, self.max.y, self.min.z),
            Vec3::new(self.max.x, self.max.y, self.max.z),
        ]
    }

    pub(crate) fn translated(self, offset: Vec3) -> Option<Self> {
        let min = self.min + offset;
        let max = self.max + offset;
        (min.is_finite() && max.is_finite()).then_some(Self { min, max })
    }

    pub(crate) fn expanded(self, padding: Vec3) -> Option<Self> {
        let min = self.min - padding;
        let max = self.max + padding;
        (min.is_finite() && max.is_finite()).then_some(Self { min, max })
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FieldBounds {
    Unbounded,
    Empty,
    Bounded(Bounds3),
}

impl FieldBounds {
    pub const fn bounded(bounds: Bounds3) -> Self {
        Self::Bounded(bounds)
    }

    pub const fn as_bounds(self) -> Option<Bounds3> {
        match self {
            Self::Bounded(bounds) => Some(bounds),
            Self::Unbounded | Self::Empty => None,
        }
    }

    pub fn union(self, other: Self) -> Self {
        match (self, other) {
            (Self::Unbounded, _) | (_, Self::Unbounded) => Self::Unbounded,
            (Self::Empty, value) | (value, Self::Empty) => value,
            (Self::Bounded(a), Self::Bounded(b)) => Self::Bounded(a.union(b)),
        }
    }

    pub fn intersection(self, other: Self) -> Self {
        match (self, other) {
            (Self::Empty, _) | (_, Self::Empty) => Self::Empty,
            (Self::Unbounded, value) | (value, Self::Unbounded) => value,
            (Self::Bounded(a), Self::Bounded(b)) => {
                a.intersection(b).map_or(Self::Empty, Self::Bounded)
            }
        }
    }

    pub(crate) fn translated(self, offset: Vec3) -> Self {
        match self {
            Self::Unbounded => Self::Unbounded,
            Self::Empty => Self::Empty,
            Self::Bounded(bounds) => bounds
                .translated(offset)
                .map_or(Self::Unbounded, Self::Bounded),
        }
    }

    pub(crate) fn expanded_scalar(self, padding: f32) -> Self {
        if ensure_non_negative(padding, "bounds padding").is_err() {
            return Self::Unbounded;
        }
        self.expanded_vector(Vec3::splat(padding))
    }

    pub(crate) fn expanded_vector(self, padding: Vec3) -> Self {
        if !padding.is_finite() || !padding.cmpge(Vec3::ZERO).all() {
            return Self::Unbounded;
        }
        match self {
            Self::Unbounded => Self::Unbounded,
            Self::Empty => Self::Empty,
            Self::Bounded(bounds) => bounds
                .expanded(padding)
                .map_or(Self::Unbounded, Self::Bounded),
        }
    }

    pub(crate) fn map_corners(self, mapper: impl Fn(Vec3) -> Vec3) -> Self {
        let bounds = match self {
            Self::Unbounded => return Self::Unbounded,
            Self::Empty => return Self::Empty,
            Self::Bounded(bounds) => bounds,
        };

        let corners = bounds.corners();
        let first = mapper(corners[0]);
        if !first.is_finite() {
            return Self::Unbounded;
        }

        let mut min = first;
        let mut max = first;
        for corner in corners.into_iter().skip(1) {
            let mapped = mapper(corner);
            if !mapped.is_finite() {
                return Self::Unbounded;
            }
            min = min.min(mapped);
            max = max.max(mapped);
        }

        Self::Bounded(Bounds3 { min, max })
    }
}
