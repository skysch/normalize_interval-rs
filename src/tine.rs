// Copyright 2018 Skylor R. Schermer.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.
////////////////////////////////////////////////////////////////////////////////
//!
////////////////////////////////////////////////////////////////////////////////



// Local imports.
use bound::Bound;
use raw_interval::RawInterval;
use utilities::Split;

// Standard library imports.
use std::cmp::Ordering;

// Local enum shortcuts.
use bound::Bound::*;
use std::cmp::Ordering::*;
use self::Tine::*;


////////////////////////////////////////////////////////////////////////////////
// Tine
////////////////////////////////////////////////////////////////////////////////
/// A portion of an interval.
///
/// Tines are used to implement ordering over the interval bounds in such a way
/// that the `TineTree` will always be able to split at the appropriate place
/// for a given bound type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tine<T> {
    /// The lower `Bound` of an `Interval`.
    Lower(Bound<T>),
    /// A combined upper and lower `Bound` of an `Interval`.
    Point(Bound<T>),
    /// The upper `Bound` of an `Interval`.
    Upper(Bound<T>),
}


impl<T> Tine<T> where T: PartialOrd + Ord + Clone {
    /// Returns the set of `Tine`s representing the given interval.
    pub fn from_raw_interval(interval: RawInterval<T>) -> Split<Self> {
        use raw_interval::RawInterval::*;
        match interval {
            Empty           => Split::Zero,
            Point(p)        => Split::One(Tine::Point(Include(p))),
            Open(l, r)      => Split::Two(Lower(Exclude(l)), Upper(Exclude(r))),
            LeftOpen(l, r)  => Split::Two(Lower(Exclude(l)), Upper(Include(r))),
            RightOpen(l, r) => Split::Two(Lower(Include(l)), Upper(Exclude(r))),
            Closed(l, r)    => Split::Two(Lower(Include(l)), Upper(Include(r))),
            UpTo(r)         => Split::Two(Lower(Infinite),   Upper(Exclude(r))),
            UpFrom(l)       => Split::Two(Lower(Exclude(l)), Upper(Infinite)),
            To(r)           => Split::Two(Lower(Infinite),   Upper(Include(r))),
            From(l)         => Split::Two(Lower(Include(l)), Upper(Infinite)),
            Full            => Split::Two(Lower(Infinite),   Upper(Infinite)),
        }
    }

    /// Returns `true` if the `Tine` represents a lower bound.
    pub fn is_lower_bound(&self) -> bool {
        match self {
            &Lower(_)          => true,
            &Point(Exclude(_)) => true,
            _                  => false,
        }
    }

    /// Returns `true` if the `Tine` represents an upper bound.
    pub fn is_upper_bound(&self) -> bool {
        match self {
            &Upper(_)          => true,
            &Point(Exclude(_)) => true,
            _                  => false,
        }
    }

    /// Returns `true` if the `Tine` represents a single point.
    pub fn is_point_include(&self) -> bool {
        match self {
            &Point(Include(_)) => true,
            _                  => false,
        }
    }

    /// Returns `true` if the `Tine` represents an upper and lower bound, 
    /// excluding the referenced point.
    pub fn is_point_exclude(&self) -> bool {
        match self {
            &Point(Exclude(_)) => true,
            _                  => false,
        }
    }

    /// Returns a reference to the `Bound` point, or `None` if the `Bound` is 
    /// `Infinite`
    pub fn as_ref(&self) -> Option<&T> {
        match self {
            &Lower(ref x) => x.as_ref(),
            &Point(ref x) => x.as_ref(),
            &Upper(ref x) => x.as_ref(),
        }
    }

    /// Returns the inner `Bound`.
    pub fn into_inner(self) -> Bound<T> {
        match self {
            Lower(x) => x,
            Point(x) => x,
            Upper(x) => x,
        }
    }

    /// Unifies two equal `Tines` by including any coincident points. Returns 
    /// `None` if all points in the boundry region are included.
    pub fn union(self, other: &Self) -> Option<Self> {
        debug_assert!(self.as_ref() == other.as_ref(),
            "cannot union unequal tines");

        match (self, other) {
            (Lower(Include(l)), &Lower(Exclude(_))) => Some(Lower(Include(l))),
            (Lower(Exclude(l)), &Lower(Include(_))) => Some(Lower(Include(l))),
            (Lower(lb),         &Lower(_))          => Some(Lower(lb)),

            (Lower(Include(l)), &Point(Include(_))) => Some(Lower(Include(l))),
            (Lower(Include(_)), &Point(Exclude(_))) => None,
            (Lower(Exclude(l)), &Point(Include(_))) => Some(Lower(Include(l))),
            (Lower(Exclude(l)), &Point(Exclude(_))) => Some(Point(Exclude(l))),

            (Lower(Include(_)), &Upper(Include(_))) => None,
            (Lower(Include(_)), &Upper(Exclude(_))) => None,
            (Lower(Exclude(_)), &Upper(Include(_))) => None,
            (Lower(Exclude(l)), &Upper(Exclude(_))) => Some(Point(Exclude(l))),

            (Point(Include(l)), &Lower(_))          => Some(Lower(Include(l))),
            (Point(Include(l)), &Point(Include(_))) => Some(Point(Include(l))),
            (Point(Include(l)), &Upper(_))          => Some(Upper(Include(l))),
            (Point(Include(_)), _)                  => None,

            (Point(Exclude(l)), &Lower(Exclude(_))) => Some(Point(Exclude(l))),
            (Point(Exclude(l)), &Point(Exclude(_))) => Some(Point(Exclude(l))),
            (Point(Exclude(l)), &Upper(Exclude(_))) => Some(Point(Exclude(l))),
            (Point(Exclude(_)), _)                  => None,

            (Upper(Include(_)), &Lower(Include(_))) => None,
            (Upper(Include(_)), &Lower(Exclude(_))) => None,
            (Upper(Exclude(_)), &Lower(Include(_))) => None,
            (Upper(Exclude(l)), &Lower(Exclude(_))) => Some(Point(Exclude(l))),

            (Upper(Include(l)), &Point(Include(_))) => Some(Upper(Include(l))),
            (Upper(Include(_)), &Point(Exclude(_))) => None,
            (Upper(Exclude(l)), &Point(Include(_))) => Some(Upper(Include(l))),
            (Upper(Exclude(l)), &Point(Exclude(_))) => Some(Point(Exclude(l))),

            (Upper(Include(l)), &Upper(Exclude(_))) => Some(Upper(Include(l))),
            (Upper(Exclude(l)), &Upper(Include(_))) => Some(Upper(Include(l))),
            (Upper(lb),         &Upper(_))          => Some(Upper(lb)),

            _ => unreachable!("invalid tine union"),
        }
    }

    /// Unifies two equal `Tines` by excluding any non-coincident points.
    /// Returns `None` if none of the points in the boundry region are included.
    pub fn intersect(self, other: &Self) -> Option<Self> {
        debug_assert!(self.as_ref() == other.as_ref(),
            "cannot intersect unequal tines");

        match (self, other) {
            (Lower(Include(l)), &Lower(Exclude(_))) => Some(Lower(Exclude(l))),
            (Lower(Exclude(l)), &Lower(Include(_))) => Some(Lower(Exclude(l))),
            (Lower(lb),         &Lower(_))          => Some(Lower(lb)),

            (Lower(Include(l)), &Point(Include(_))) => Some(Point(Include(l))),
            (Lower(Exclude(l)), &Point(Exclude(_))) => Some(Lower(Exclude(l))),
            (Lower(_),          &Point(_))          => None,

            (Lower(Include(l)), &Upper(Include(_))) => Some(Point(Include(l))),
            (Lower(_),          &Upper(_))          => None,

            (Point(Include(l)), &Lower(Include(_))) => Some(Point(Include(l))),
            (Point(Include(l)), &Point(Include(_))) => Some(Point(Include(l))),
            (Point(Include(l)), &Upper(Include(_))) => Some(Point(Include(l))),
            (Point(Include(_)), _)                  => None,

            (Point(Exclude(l)), &Lower(_))          => Some(Lower(Exclude(l))),
            (Point(Exclude(_)), &Point(Include(_))) => None,
            (Point(Exclude(l)), &Point(Exclude(_))) => Some(Point(Exclude(l))),
            (Point(Exclude(l)), &Upper(_))          => Some(Upper(Exclude(l))),

            (Upper(Include(l)), &Lower(Include(_))) => Some(Point(Include(l))),
            (Upper(_),          &Lower(_))          => None,

            (Upper(Include(l)), &Point(Include(_))) => Some(Point(Include(l))),
            (Upper(Exclude(l)), &Point(Exclude(_))) => Some(Upper(Exclude(l))),
            (Upper(_),          &Point(_))          => None,

            (Upper(Include(l)), &Upper(Exclude(_))) => Some(Upper(Exclude(l))),
            (Upper(Exclude(l)), &Upper(Include(_))) => Some(Upper(Exclude(l))),
            (Upper(lb),         &Upper(_))          => Some(Upper(lb)),

            _ => unreachable!("invalid tine intersect"),
        }
    }

    /// Unifies two equal `Tines` by excluding any coincident points. Returns
    /// `None` if none of the points in the boundry region are included.
    pub fn minus(self, other: &Self) -> Option<Self> {
        debug_assert!(self.as_ref() == other.as_ref(),
            "cannot intersect unequal tines");

        match (self, other) {
            (Lower(Include(l)), &Lower(Exclude(_))) => Some(Point(Include(l))),
            (Lower(_),          &Lower(_))          => None,

            (Lower(Include(l)), &Point(Include(_))) => Some(Lower(Exclude(l))),
            (Lower(_),          &Point(_))          => None,

            (Lower(Include(l)), &Upper(Include(_))) => Some(Lower(Exclude(l))),
            (Lower(Include(l)), &Upper(Exclude(_))) => Some(Lower(Include(l))),
            (Lower(Exclude(l)), &Upper(Include(_))) => Some(Lower(Exclude(l))),
            (Lower(Exclude(l)), &Upper(Exclude(_))) => Some(Lower(Exclude(l))),

            (Point(Include(_)), &Lower(Include(_))) => None,
            (Point(Include(_)), &Point(Include(_))) => None,
            (Point(Include(_)), &Upper(Include(_))) => None,
            (Point(Include(l)), _)                  => Some(Point(Include(l))),

            (Point(Exclude(l)), &Lower(_))          => Some(Lower(Exclude(l))),
            (Point(Exclude(l)), &Point(Include(_))) => Some(Point(Exclude(l))),
            (Point(Exclude(_)), &Point(Exclude(_))) => None,
            (Point(Exclude(l)), &Upper(_))          => Some(Upper(Exclude(l))),

            (Upper(Include(l)), &Lower(Include(_))) => Some(Upper(Exclude(l))),
            (Upper(Include(l)), &Lower(Exclude(_))) => Some(Upper(Include(l))),
            (Upper(Exclude(l)), &Lower(Include(_))) => Some(Upper(Exclude(l))),
            (Upper(Exclude(l)), &Lower(Exclude(_))) => Some(Upper(Exclude(l))),

            (Upper(Include(l)), &Point(Include(_))) => Some(Upper(Exclude(l))),
            (Upper(_),          &Point(_))          => None,

            (Upper(Include(l)), &Upper(Exclude(_))) => Some(Point(Include(l))),
            (Upper(_),          &Upper(_))          => None,

            _ => unreachable!("invalid tine intersect"),
        }
    }

    /// Returns the `Tine` with its boundaries inverted.
    pub fn invert(self) -> Self {
        match self {
            Lower(Include(p)) => Upper(Exclude(p)),
            Lower(Exclude(p)) => Upper(Include(p)),
            Point(Include(p)) => Point(Exclude(p)),
            Point(Exclude(p)) => Point(Include(p)),
            Upper(Include(p)) => Lower(Exclude(p)),
            Upper(Exclude(p)) => Lower(Include(p)),
            _ => panic!("cannot invert infinite Tine"),
        }
    }
}


impl<T> PartialOrd for Tine<T> where T: PartialOrd + Ord + Clone {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if let (Some(l), Some(r)) = (self.as_ref(), other.as_ref()) {
            // Compare points.
            l.partial_cmp(r)
        } else {
            // Compare infinite bounds.
            match (self, other) {
                (&Lower(Infinite), &Lower(Infinite)) => Some(Equal),
                (&Upper(Infinite), &Upper(Infinite)) => Some(Equal),

                (&Lower(Infinite), _)                => Some(Less),
                (&Upper(Infinite), _)                => Some(Greater),

                (_,                &Upper(Infinite)) => Some(Less),
                (_,                &Lower(Infinite)) => Some(Greater),
                // Point(Infinite) is nonsense.
                _ => unreachable!("invalid Tine value"),
            }
        }
    }
}

// Tine ordering is total.
impl<T> Ord for Tine<T> where T: PartialOrd + Ord + Clone {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}