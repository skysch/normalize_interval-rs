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
use tine::Tine;
use tine::Tine::*;
use utilities::Split;

// Standard library imports.
use std::collections::BTreeSet;
use std::collections::btree_set;
use std::collections;
use std::iter::FromIterator;

// Local enum shortcuts.
use bound::Bound::*;



////////////////////////////////////////////////////////////////////////////////
// TineTree
////////////////////////////////////////////////////////////////////////////////
/// A possibly noncontiguous collection of `RawInterval`s of the type `T`.
/// Implemented as an ordered list of `Tine`s. Used to implement the internal
/// state of `Selection`.
///
/// Informally, a `TineTree` acts like a number line with markers (`Tine`s) on
/// it for each `Interval` bound in a possibly disjoint union of `Interval`s.
/// 
/// [`RawInterval`]: raw_interval/struct.RawInterval.html
/// [`Selection`]: selection/struct.Selection.html
/// [`Tine`]: tine_tree/struct.Tine.html
/// [`Interval`]: interval/struct.Interval.html
///
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct TineTree<T>(BTreeSet<Tine<T>>) where T: Ord + Clone;

impl<T> TineTree<T> where T: Ord + Clone {
    ////////////////////////////////////////////////////////////////////////////
    // Constructors
    ////////////////////////////////////////////////////////////////////////////

    /// Constructs an empty `TineTree`.
    pub fn new() -> Self {
        TineTree(BTreeSet::new())
    }

    /// Constructs a `TineTree` from a `RawInterval`.
    pub fn from_raw_interval(interval: RawInterval<T>) -> Self {
        TineTree(BTreeSet::from_iter(Tine::from_raw_interval(interval)))
    }

    ////////////////////////////////////////////////////////////////////////////
    // Bound accessors
    ////////////////////////////////////////////////////////////////////////////

    /// Returns the lower [`Bound`] of the `TineTree`, or `None` if the 
    /// `TineTree` is empty.
    #[inline]
    pub fn lower_bound(&self) -> Option<Bound<T>> {
        self.0.iter().next().cloned().map(Tine::into_inner)
    }

    /// Returns the upper [`Bound`] of the `TineTree`, or `None` if the 
    /// `TineTree` is empty.
    #[inline]
    pub fn upper_bound(&self) -> Option<Bound<T>> {
        self.0.iter().next_back().cloned().map(Tine::into_inner)
    }


    ////////////////////////////////////////////////////////////////////////////
    // Query operations
    ////////////////////////////////////////////////////////////////////////////
    
    /// Returns `true` if the `TineTree` is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns `true` if the `TineTree` contains the given point.
    pub fn contains(&self, point: &T) -> bool {
        for interval in self.iter_intervals() {
            if interval.contains(point) {return true;}
        }
        false
    }

    ////////////////////////////////////////////////////////////////////////////
    // Set Operations
    ////////////////////////////////////////////////////////////////////////////

    /// Returns a `TineTree` containing all points not in present in the 
    /// `TineTree`.
    pub fn complement(&self) -> Self {
        // Early exit if we're complementing an empty interval.
        if self.0.is_empty() {
            return RawInterval::Full.into();
        }

        let mut complement = TineTree::new();
        let mut tine_iter = self.0.iter();
        
        // Early exit if we're complementing a point interval.
        if self.0.len() == 1 {
            let tine = tine_iter
                .next()
                .expect("nonempty TineTree")
                .clone()
                .invert();
            debug_assert!(tine.is_point_exclude());

            complement.0.insert(Lower(Infinite));
            complement.0.insert(tine);
            complement.0.insert(Upper(Infinite));
            return complement;
        }        

        // Get first and last to handle infinite bounds.
        match tine_iter.next() {
            Some(&Lower(Infinite)) => {/* Do Nothing. */},
            Some(tine)            => {
                complement.0.insert(Lower(Infinite));
                complement.0.insert(tine.clone().invert());
            },
            _ => unreachable!("TineTree len > 1"),
        }
        match tine_iter.next_back() {
            Some(&Upper(Infinite)) => {/* Do Nothing. */},
            Some(tine)            => {
                complement.0.insert(Upper(Infinite));
                complement.0.insert(tine.clone().invert());
            },
            _ => unreachable!("TineTree len > 0"),
        }

        // Invert all remaining tines.
        for tine in tine_iter {
            complement.0.insert(tine.clone().invert());
        }

        complement
    }

    /// Returns a `TineTree` containing all points in present in both of the 
    /// `TineTree`s.
    pub fn intersect(&self, other: &Self) -> Self {
        let mut intersection = Self::new();
        let mut self_intervals = self.iter_intervals();
        let mut other_intervals = other.iter_intervals();

        while let Some(self_interval) = self_intervals.next() {
            'segment: loop {
                if let Some(other_interval) = other_intervals.next() {
                    let i = self_interval.intersect(&other_interval);
                    if !i.is_empty() {
                        intersection.union_in_place(&i);
                    } else {
                        // Nothing more overlapping in this segment.
                        break 'segment;
                    }

                } else {
                    // Nothing more overlapping anywhere.
                    return intersection;
                }
            }
        }
        intersection
    }

    /// Returns a `TineTree` containing all points in present in either of the 
    /// `TineTree`s.
    pub fn union(&self, other: &Self) -> Self {
        let mut union = self.clone();
        for interval in other.iter_intervals() {
            union.union_in_place(&interval);
        }
        union
    }

    /// Returns a `TineTree` containing the intersection of the given 
    /// `TineTree`'s intervals.    
    pub fn minus(&self, other: &Self) -> Self {
        let mut minus = self.clone();
        for interval in other.iter_intervals() {
            minus.minus_in_place(&interval);
        }
        minus
    }

    /// Returns the smallest `RawInterval` containing all of the points in the 
    /// `TineTree`.
    pub fn enclose(&self) -> RawInterval<T> {
        // Early exit if we're enclosing an empty interval.
        if self.0.is_empty() {
            return RawInterval::Empty;
        } 

        let mut tine_iter = self.0.iter();

        // Early exit if we're enclosing a point interval.
        if self.0.len() == 1 {
            let tine = tine_iter
                .next()
                .expect("nonempty TineTree");
            debug_assert!(tine.is_point_include());
            let pt = tine
                .as_ref()
                .expect("point Tine value")
                .clone();
            return RawInterval::Point(pt);
        } 

        // Get first and last tines.
        let lb = tine_iter
            .next()
            .expect("first tine with len > 1")
            .clone()
            .into_inner();
        let ub = tine_iter
            .next_back()
            .expect("last tine with len > 1")
            .clone()
            .into_inner();

        RawInterval::new(lb, ub)
    }

    /// Returns the smallest closed `RawInterval` containing all of the points
    /// in the `TineTree`.
    pub fn closure(&self) -> RawInterval<T> {
        self.enclose().closure()
    }

    ////////////////////////////////////////////////////////////////////////////
    // In-place operations
    ////////////////////////////////////////////////////////////////////////////

    /// Intersects the given interval with the contents of the tree.
    pub fn intersect_in_place(&mut self, interval: &RawInterval<T>) {
        // Early exit if we're intersecting a full interval or are empty.
        if self.0.is_empty() || interval.is_full() {return};

        // Early exit if we're intersection an empty interval.
        if interval.is_empty() {
            *self = TineTree::new();
            return;
        }

        // Early exit if we're intersection a point interval.
        if let &RawInterval::Point(ref pt) = interval {
            if self.contains(pt) {
                *self = TineTree::from_raw_interval(interval.clone());
            } else {
                *self = TineTree::new();
            }
            return;
        }

        match Tine::from_raw_interval(interval.clone()) {
            Split::Zero                   => {
                *self = TineTree::new();
                return;
            },
            Split::One(Point(Include(p))) => {
                if self.contains(&p) {
                    *self = TineTree::from_raw_interval(RawInterval::Point(p));
                } else {
                    *self = TineTree::new();
                }
                return;
            },
            Split::Two(l, u)              => {
                self.intersect_proper_interval(l, u)
            },
            _ => unreachable!("invalid Tine from interval"),
        }
    }

    fn intersect_proper_interval(&mut self, l: Tine<T>, u: Tine<T>) {
        let mut ts = self.interior_split_for_proper_interval(&l, &u);

        // Merge tines if overlap or use given ones. We should only have `None`
        // in the case of a intersection annhiliation.
        let merged_l = if ts[2].is_some() {
            ts[2].take().and_then(|lower| lower.intersect(&l))
        } else {
            Some(l)
        };

        let merged_u = if ts[3].is_some() {
            ts[3].take().and_then(|upper| upper.intersect(&u))
        } else {
            Some(u)
        };
        
        // Ensure inner tines have the correct bounds.
        debug_assert!(merged_l
            .as_ref()
            .map(Tine::is_lower_bound)
            .unwrap_or(true));
        debug_assert!(merged_u
            .as_ref()
            .map(Tine::is_upper_bound)
            .unwrap_or(true));

        
        // We need to detect whether the point is inside or outside an interval.
        // To do this, we look at the tines inside and outside the interval.
        let open_before = ts[0]
            .as_ref()
            .map(Tine::is_lower_bound)
            .unwrap_or(false);
        let closed_after = ts[5]
            .as_ref()
            .map(Tine::is_upper_bound)
            .unwrap_or(false);

        let in_l = ts[1]
            .as_ref()
            .map(Tine::is_upper_bound)
            .unwrap_or(false);
        let in_r = ts[4]
            .as_ref()
            .map(Tine::is_lower_bound)
            .unwrap_or(false);


        // Insert tines into the tree, ignoring them if the are not wrapped by a
        // surrounding interval, or not wrapping a surrounding interval.
        match (open_before, merged_l, in_l, in_r, merged_u, closed_after) {
            (_,     Some(l), true,  true,  Some(u), _   )  |
            (_,     Some(l), false, false, Some(u), _   )  => {
                // (   ) (   )
                //   (     )
                //     O R
                // (     )
                //   ( )
                //     O R
                // (     )
                // (  )
                //     O R
                // (     )
                //    (  )
                self.0.insert(l);
                self.0.insert(u);
            },
            (true, Some(l),  true,  false, _,       false) => {
                // (   )
                //   (   )
                //     O R
                // (   ) ( )
                //   (   )
                self.0.insert(l);
            },
            (false, _,       false, true,  Some(u), true)  => {
                //   (   )
                // (   )
                //     O R
                // ( ) (   )
                //   (   )
                self.0.insert(u);
            },
            (false, _,       false, false, _,       false) => {
                // )     (
                // (     )
                //     O R
                //   ( )
                // (     )
                /* Do nothing. */
            },
            _ => unreachable!("invalid bounds for intersection interval"),
        }
    }

    /// Unions the given interval with the contents of the tree.
    pub fn union_in_place(&mut self, interval: &RawInterval<T>) {
        // Early exit if we're unioning a full interval.
        if interval.is_full() {
            *self = TineTree::from_raw_interval(RawInterval::Full);
            return;
        }

        match Tine::from_raw_interval(interval.clone()) {
            Split::Zero      => return,
            Split::One(p)    => self.union_point_interval(p),
            Split::Two(l, u) => self.union_proper_interval(l, u),
        }
    }

    fn union_point_interval(&mut self, p: Tine<T>) {
        let mut ts = self.exterior_split_for_point_interval(&p);

        let p = if ts[1].is_some() {
            if let Some(merged) = ts[1]
                .take()
                .and_then(|pt| pt.union(&p)) 
            {
                merged
            } else {
                // If the point annhilates, then we've already joined two
                // intervals by removing the Point(Exclude(_)) from the tree in
                // exterior_split_for_point_interval. So nothing else needs to happen.
                return;
            }
        } else {
            p
        };
        
        // We need to detect whether the point is inside or outside an interval.
        // To do this, we look at the tines before and after the interval.
        let open_before = ts[0]
            .as_ref()
            .map(Tine::is_lower_bound)
            .unwrap_or(false);
        let closed_after = ts[2]
            .as_ref()
            .map(Tine::is_upper_bound)
            .unwrap_or(false);

        // Insert tine into the tree, ignoring it if it is wrapped by a
        // surrounding interval.
        match (open_before, closed_after) {
            (true,  true)  => {
                // (   )
                //   |
                // Do nothing.
            },
            (true,  false) => {
                // ( )   ( )
                //   |
                debug_assert!(!p.is_lower_bound());
                self.0.insert(p);
            },
            (false, true)  => {
                // ( )   ( )
                //       |
                debug_assert!(!p.is_upper_bound());
                self.0.insert(p);
            },
            (false, false) => {
                // ( )   ( )
                //     |
                self.0.insert(p);
            },
        }
    }

    fn union_proper_interval(&mut self, l: Tine<T>, u: Tine<T>) {
        let mut ts = self.exterior_split_for_proper_interval(&l, &u);

        // Merge tines if overlap or use given one. We should only have `None`
        // in the case of a union annhiliation.
        let merged_l = if ts[1].is_some() {
            ts[1].take().and_then(|lower| lower.union(&l))
        } else {
            Some(l)
        };

        let merged_u = if ts[2].is_some() {
            ts[2].take().and_then(|upper| upper.union(&u))
        } else {
            Some(u)
        };

        // Ensure inner tines have the correct bounds.
        debug_assert!(merged_l
            .as_ref()
            .map(Tine::is_lower_bound)
            .unwrap_or(true));
        debug_assert!(merged_u
            .as_ref()
            .map(Tine::is_upper_bound)
            .unwrap_or(true));

        // We need to detect whether the interval is inside or outside an 
        // existing interval. To do this, we look at the tines before and after
        // the interval.
        let open_before = ts[0]
            .as_ref()
            .map(Tine::is_lower_bound)
            .unwrap_or(false);
        let closed_after = ts[3]
            .as_ref()
            .map(Tine::is_upper_bound)
            .unwrap_or(false);
        
        // Insert tines into the tree, ignoring them if the are wrapped by a
        // surrounding interval.
        match (open_before, merged_l, merged_u, closed_after) {
            (true,  Some(l), Some(u), true)  => {
                // ( ) ( )
                //   ( )
                if l.is_upper_bound() {self.0.insert(l);}
                if u.is_lower_bound() {self.0.insert(u);}
            },
            (true,  Some(l), Some(u), false) => {
                // ( ) ( ) ( )
                //   (   )
                //     O R
                // ( ) ( )
                //   (   )
                if l.is_upper_bound() {self.0.insert(l);}
                debug_assert!(!u.is_lower_bound());
                self.0.insert(u);
            },
            (false, Some(l), Some(u), true)  => {
                // ( ) ( ) ( )
                //     (   )
                //     O R
                // ( ) ( ) ( )
                // [   )
                debug_assert!(!l.is_upper_bound());
                self.0.insert(l);
                if u.is_lower_bound() {self.0.insert(u);}

            },
            (false, Some(l), Some(u), false) => {
                // ( ) ( ) ( )
                //     [ ]
                //     O R
                // ( ) ( )
                //     [ ]
                //     O R
                // ( ) ( ) ( )
                // [     )
                //     O R
                // ( ) ( )
                // [     ]
                debug_assert!(!l.is_upper_bound());
                self.0.insert(l);
                debug_assert!(!u.is_lower_bound());
                self.0.insert(u);
            },

            (true,  Some(l), None,    true)  => {
                // ( ) ( ) ( )
                //   (     ]
                if l.is_point_exclude() {self.0.insert(l);}
            },
            (false, Some(l), None,    true)  => {
                // ( ) ( ) ( )
                //     [   ]
                //     O R
                // ( ) ( ) ( )
                // [       ]
                debug_assert!(!l.is_upper_bound());
                self.0.insert(l);
            },

            (true,  None,    Some(u), true)  => {
                // ( ) ( ) ( )
                //   [     )
                if u.is_point_exclude() {self.0.insert(u);}
            },
            (true,  None,    Some(u), false)  => {
                // ( ) ( ) ( )
                //   [   ]
                //     O R
                // ( ) ( ) ( )
                //   [       ]
                debug_assert!(!u.is_lower_bound());
                self.0.insert(u);
            },

            (true,  None,    None,    true) => {
                // ( ) ( ) ( )
                //   [     ] 
                // Do nothing.
            },
            _ => unreachable!("invalid bounds for union interval"),
        }
    }

    /// Minuses the given interval from the contents of the tree.
    pub fn minus_in_place(&mut self, interval: &RawInterval<T>) {
        // Early exit if we're minusing an empty interval or are empty.
        if self.0.is_empty() || interval.is_empty() {return};

        // Early exit if we're minusing a full interval.
        if interval.is_full() {
            *self = TineTree::new();
            return;
        }

        match Tine::from_raw_interval(interval.clone()) {
            Split::Zero      => return,
            Split::One(p)    => self.minus_point_interval(p),
            Split::Two(l, u) => self.minus_proper_interval(l, u),
        }
    }

    fn minus_point_interval(&mut self, p: Tine<T>) {
        let mut ts = self.exterior_split_for_point_interval(&p);

        let p = if ts[1].is_some() {
            if let Some(merged) = ts[1]
                .take()
                .and_then(|pt| pt.minus(&p)) 
            {
                merged
            } else {
                // If the point annhilates, then we've already joined two
                // intervals by removing the Point(Exclude(_)) from the tree in
                // minus_split_tree_point. So nothing else needs to happen.
                return;
            }
        } else {
            p
        };
        
        // We need to detect whether the point is inside or outside an interval.
        // To do this, we look at the tines before and after the interval.
        let open_before = ts[0]
            .as_ref()
            .map(Tine::is_lower_bound)
            .unwrap_or(false);
        let closed_after = ts[2]
            .as_ref()
            .map(Tine::is_upper_bound)
            .unwrap_or(false);

        // Insert tine into the tree, ignoring it if it is wrapped by a
        // surrounding interval.
        // NOTE: We cannot have a Point(Exclude) here, because those will never
        // result from an interval-tine conversion.
        match (open_before, closed_after) {
            (true,  true)  => {
                // (   )
                //   |
                self.0.insert(p.invert());
            },
            (true,  false) => {
                // ( )   ( )
                //   |
                debug_assert!(p.is_upper_bound());
                self.0.insert(p);
            },
            (false, true)  => {
                // ( )   ( )
                //       |
                debug_assert!(p.is_lower_bound());
                self.0.insert(p);
            },
            (false, false) => {
                // ( )   ( )
                //     |
                // Do nothing.
            },
        }
    }

    fn minus_proper_interval(&mut self, l: Tine<T>, u: Tine<T>) {
        let mut ts = self.exterior_split_for_proper_interval(&l, &u);

        // Merge tines if overlap
        let merged_l = if ts[1].is_some() {
            ts[1].take().and_then(|lower| lower.minus(&l))
        } else {
            Some(l)
        };

        let merged_u = if ts[2].is_some() {
            ts[2].take().and_then(|upper| upper.minus(&u))
        } else {
            Some(u)
        };

        // We need to detect whether the interval is inside or outside an 
        // existing interval. To do this, we look at the tines before and after
        // the interval.
        let open_before = ts[0]
            .as_ref()
            .map(Tine::is_lower_bound)
            .unwrap_or(false);
        let closed_after = ts[3]
            .as_ref()
            .map(Tine::is_upper_bound)
            .unwrap_or(false);
        
        println!("Minus {} {} {} {}", open_before, merged_l.is_some(), merged_u.is_some(), closed_after);
        // Insert tines into the tree, ignoring them if the are not wrapped by a
        // surounding interval.
        match (open_before, merged_l, merged_u, closed_after) {
            (true,  Some(l), Some(u), true)  => {
                // ( ) ( )
                //  (   )
                //     O R
                // ( ) ( )
                //   ( )
                self.0.insert(if l.is_upper_bound() {l} else {l.invert()});
                self.0.insert(if u.is_lower_bound() {u} else {u.invert()});
            },
            (true,  Some(l), upper,   false)  => {
                // ( )
                //  ( )
                //     O R
                // ( )
                //   ( )
                //     O R
                // (   )
                //   ( )
                //     O R
                // (   ]
                //   ( )
                //     O R
                self.0.insert(if l.is_upper_bound() {l} else {l.invert()});
                if let Some(Point(Include(p))) = upper {
                    self.0.insert(Point(Include(p)));
                }
            },
            (false, lower,   Some(u), true)   => {
                //  ( )
                // ( )
                //     O R
                //   ( )
                // ( )
                //     O R
                // (   )
                // ( )
                //     O R
                // [   )
                // ( )
                self.0.insert(if u.is_lower_bound() {u} else {u.invert()});
                if let Some(Point(Include(p))) = lower {
                    self.0.insert(Point(Include(p)));
                }
            },
            (false, Some(l), Some(u), false)  => {
                //  ( )
                // (   )
                //     O R
                // [ ]
                // ( )
                if l.is_point_include() {self.0.insert(l);}
                if u.is_point_include() {self.0.insert(u);}
            },

            (false, Some(l), None,    false)  => {
                // [ )
                // ( )
                //     O R
                //   |
                // ( ]
                if l.is_point_include() {self.0.insert(l);}
            },

            (false, None,    Some(u), false)  => {
                // ( ]
                // ( )
                //     O R
                // |
                // [ )
                if u.is_point_include() {self.0.insert(u);}
            },

            (false, None,    None,    false)  => {
                // ( )
                // ( )
                // Do nothing.
            },
            _ => unreachable!("invalid bounds for minus interval"),
        }
    }

    /// Splits the tine tree into three sections for an interval-like Tine for
    /// an intersect.
    //
    // The array is returned with the following semantics:
    // ```rust
    // [
    //     0 => Copy of the first tine less than the lower tine.
    //     1 => Copy of the first tine greater than the lower tine.
    //     2 => The tine equal to the lower tine.
    //     3 => The tine equal to the upper tine.
    //     4 => Copy of the first tine less than the upper tine.
    //     5 => Copy of the first tine greater than the upper tine.
    // ]
    // ```
    //
    // Any tines not between lower and upper are dropped.
    fn interior_split_for_proper_interval(
        &mut self,
        lower: &Tine<T>,
        upper: &Tine<T>) 
        -> [Option<Tine<T>>; 6]
    {
        debug_assert!(lower < upper);
        let mut res = [None, None, None, None, None, None];

        // Get lower and upper if they are in the tree.
        res[2] = self.0.take(lower);
        res[3] = self.0.take(upper);
        
        // Get before and after points and drop anything not in the center.
        let mut center = self.0.split_off(lower);
        let right_side = center.split_off(upper);

        {
            let mut backward = self.0.iter();
            res[0] = backward.next_back().cloned();

            let mut forward = center.iter();
            res[1] = forward.next().cloned();
        }

        {
            let mut backward = center.iter().rev();
            res[4] = backward.next().cloned();

            let mut forward = right_side.iter();
            res[5] = forward.next().cloned();
        }

        debug_assert_eq!(res[1].is_some(), res[4].is_some());
        
        self.0 = center;
        res
    }

    /// Splits the tine tree into three sections for a point-like Tine for a
    /// union.
    //
    // The array is returned with the following semantics:
    // ```rust
    // [
    //     0 => Copy of the first tine less than the given tine.
    //     1 => The tine equal to the given tine.
    //     2 => Copy of the first tine greater than the given tine.
    // ]
    // ```
    fn exterior_split_for_point_interval(&mut self, tine: &Tine<T>)
        -> [Option<Tine<T>>; 3]
    {
        let mut res = [None, None, None];

        // Get pt if it is in the tree.
        res[1] = self.0.take(&tine);

        // Get before and after points.
        let mut right_side = self.0.split_off(&tine);
        res[0] = self.0.iter().next_back().cloned();
        res[2] = right_side.iter().next().cloned();

        self.0.append(&mut right_side);
        res
    }

    /// Splits the tine tree into three sections for an interval-like Tine for a
    /// union or minus.
    //
    // The array is returned with the following semantics:
    // ```rust
    // [
    //     0 => Copy of the first tine less than the lower tine.
    //     1 => The tine equal to the lower tine.
    //     2 => The tine equal to the upper tine.
    //     3 => Copy of the first tine greater than the upper tine.
    // ]
    // ```
    //
    // Any tines between lower and upper are dropped.
    fn exterior_split_for_proper_interval(
        &mut self,
        lower: &Tine<T>,
        upper: &Tine<T>)
        -> [Option<Tine<T>>; 4]
    {
        let mut res = [None, None, None, None];

        // Get lower and upper if they are in the tree.
        res[1] = self.0.take(lower);
        res[2] = self.0.take(upper);

        // Get before and after points and drop anything in the center.
        let mut center = self.0.split_off(&lower);
        {
            let mut backward = self.0.iter();
            res[0] = backward.next_back().cloned();
        }

        let mut right_side = center.split_off(&upper);
        {
            let mut forward = right_side.iter();
            res[3] = forward.next().cloned();
        }
        
        self.0.append(&mut right_side);
        res
    }

    ////////////////////////////////////////////////////////////////////////////
    // Iterator conversions
    ////////////////////////////////////////////////////////////////////////////

    /// Returns an iterator over each of the `RawInterval`s in the tree.
    pub fn iter_intervals(&self) -> RawIntervalIter<T> {
        RawIntervalIter {
            tine_iter: self.0.iter(),
            saved_lower: None,
            saved_upper: None,
        }
    }
}



////////////////////////////////////////////////////////////////////////////////
// Conversion traits
////////////////////////////////////////////////////////////////////////////////
impl<T> From<RawInterval<T>> for TineTree<T>
    where T: PartialOrd + Ord + Clone
{
    fn from(interval: RawInterval<T>) -> Self {
        TineTree::from_raw_interval(interval)
    }
}

impl<T, I> From<I> for TineTree<T>
    where
        T: PartialOrd + Ord + Clone,
        I: Iterator<Item=RawInterval<T>>
{
    fn from(iter: I) -> Self {
        let mut tine_tree = TineTree::new();
        for interval in iter {
            tine_tree.union_in_place(&interval);
        }
        tine_tree
    }
}

impl<T> FromIterator<RawInterval<T>> for TineTree<T>
    where T: PartialOrd + Ord + Clone
{
    fn from_iter<I>(iter: I) -> Self
        where I: IntoIterator<Item=RawInterval<T>>
    {
        let mut tine_tree = TineTree::new();
        for interval in iter.into_iter() {
            tine_tree.union_in_place(&interval);
        }
        tine_tree
    }
}

impl<T> IntoIterator for TineTree<T>
    where T: PartialOrd + Ord + Clone 
{
    type Item = RawInterval<T>;
    type IntoIter = IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            inner: self.0.into_iter(),
            saved_lower: None,
            saved_upper: None,
        }
    }
}


////////////////////////////////////////////////////////////////////////////////
// IntoIter
////////////////////////////////////////////////////////////////////////////////
/// An owning `Iterator` over the `TineTree`s `RawInterval`s.
pub struct IntoIter<T> {
    inner: btree_set::IntoIter<Tine<T>>,
    saved_lower: Option<Tine<T>>,
    saved_upper: Option<Tine<T>>,
}

impl<T> Iterator for IntoIter<T> where T: PartialOrd + Ord + Clone {
    type Item = RawInterval<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.saved_lower
            .take()
            .or_else(|| self.inner.next())
            .map(|lower| {
                if let Point(Include(p)) = lower {
                    // Next tine is a single point.
                    RawInterval::Point(p)
                } else {
                    // Next tine must be a lower bound of an interval.
                    debug_assert!(lower.is_lower_bound());

                    let upper = self.inner.next().clone()
                        .or_else(|| self.saved_upper.take())
                        .expect("interval is not partial");

                    if upper.is_point_exclude() {
                        self.saved_lower = Some(upper.clone());
                    }

                    // ... and the next tine after must be an upper bound.
                    debug_assert!(upper.is_upper_bound());

                    let lower = lower.into_inner();
                    let upper = upper.into_inner();
                    RawInterval::new(lower, upper)
                }
            })
    }
}

impl<T> DoubleEndedIterator for IntoIter<T>
    where T: PartialOrd + Ord + Clone 
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.saved_upper
            .take()
            .or_else(|| self.inner.next_back())
            .map(|upper| {
                if let Point(Include(p)) = upper {
                    // Next tine is a single point.
                    RawInterval::Point(p)
                } else {
                    // Next tine must be an upper bound of an interval.
                    debug_assert!(upper.is_upper_bound());

                    let lower = self.inner.next_back().clone()
                        .or_else(|| self.saved_lower.take())
                        .expect("interval is not partial");

                    if lower.is_point_exclude() {
                        self.saved_lower = Some(lower.clone());
                    }

                    // ... and the next tine after must be a lower bound.
                    debug_assert!(lower.is_lower_bound());

                    let upper = upper.into_inner();
                    let lower = lower.into_inner();
                    RawInterval::new(lower, upper)
                }
            })
    }
}

////////////////////////////////////////////////////////////////////////////////
// RawIntervalIter
////////////////////////////////////////////////////////////////////////////////
/// An `Iterator` that constructs `RawInterval`s from a sequence of `Tine`s.
pub struct RawIntervalIter<'t, T: 't> {
    tine_iter: collections::btree_set::Iter<'t, Tine<T>>,
    saved_lower: Option<Tine<T>>,
    saved_upper: Option<Tine<T>>,
}

impl<'t, T> Iterator for RawIntervalIter<'t, T>
    where T: PartialOrd + Ord + Clone
{
    type Item = RawInterval<T>;

    fn next(&mut self) -> Option<Self::Item> {
        self.saved_lower
            .take()
            .or_else(|| self.tine_iter.next().cloned())
            .map(|lower| {
                if let Point(Include(p)) = lower {
                    // Next tine is a single point.
                    RawInterval::Point(p)
                } else {
                    // Next tine must be a lower bound of an interval.
                    debug_assert!(lower.is_lower_bound());

                    let upper = self.tine_iter.next().cloned()
                        .or_else(|| self.saved_upper.take())
                        .expect("interval is not partial");

                    if upper.is_point_exclude() {
                        self.saved_lower = Some(upper.clone());
                    }

                    // ... and the next tine after must be an upper bound.
                    debug_assert!(upper.is_upper_bound());

                    let lower = lower.into_inner();
                    let upper = upper.into_inner();
                    RawInterval::new(lower, upper)
                }
            })

    }
}

impl<'t, T> DoubleEndedIterator for RawIntervalIter<'t, T>
    where T: PartialOrd + Ord + Clone 
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.saved_upper
            .take()
            .or_else(|| self.tine_iter.next_back().cloned())
            .map(|upper| {
                if let Point(Include(p)) = upper {
                    // Next tine is a single point.
                    RawInterval::Point(p)
                } else {
                    // Next tine must be an upper bound of an interval.
                    debug_assert!(upper.is_upper_bound());

                    let lower = self.tine_iter.next_back().cloned()
                        .or_else(|| self.saved_lower.take())
                        .expect("interval is not partial");

                    if lower.is_point_exclude() {
                        self.saved_lower = Some(lower.clone());
                    }

                    // ... and the next tine after must be a lower bound.
                    debug_assert!(lower.is_lower_bound());

                    let upper = upper.into_inner();
                    let lower = lower.into_inner();
                    RawInterval::new(lower, upper)
                }
            })
    }
}

////////////////////////////////////////////////////////////////////////////////
// TreeSplit
////////////////////////////////////////////////////////////////////////////////
/// A `TineTree`s elements split out for simplified manipulation.
struct TreeSplit<T> {
    pub before: Option<Tine<T>>,
    pub lower: Option<Tine<T>>,
    pub upper: Option<Tine<T>>,
    pub after: Option<Tine<T>>,
}

impl<T> Default for TreeSplit<T> {
    fn default() -> Self {
        TreeSplit {
            before: None,
            lower: None,
            upper: None,
            after: None,
        }
    }
}