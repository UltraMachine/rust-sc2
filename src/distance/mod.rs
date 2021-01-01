//! Traits for comparing distance between points and units.

use crate::{geometry::Point2, units::iter::filter_fold};
use std::{cmp::Ordering, vec::IntoIter};

#[cfg(feature = "rayon")]
pub mod rayon;

/// Basic trait for comparing distance.
pub trait Distance: Into<Point2> {
	/// Calculates squared euclidean distance from `self` to `other`.
	fn distance_squared<P: Into<Point2>>(self, other: P) -> f32 {
		let a = self.into();
		let b = other.into();

		let dx = a.x - b.x;
		let dy = a.y - b.y;

		dx * dx + dy * dy
	}

	/// Calculates euclidean distance from `self` to `other`.
	#[inline]
	fn distance<P: Into<Point2>>(self, other: P) -> f32 {
		self.distance_squared(other).sqrt()
	}
	/// Checks if distance between `self` and `other` is less than given `distance`.
	#[inline]
	fn is_closer<P: Into<Point2>>(self, distance: f32, other: P) -> bool {
		self.distance_squared(other) < distance * distance
	}
	/// Checks if distance between `self` and `other` is greater than given `distance`.
	#[inline]
	fn is_further<P: Into<Point2>>(self, distance: f32, other: P) -> bool {
		self.distance_squared(other) > distance * distance
	}
}

impl<T: Into<Point2>> Distance for T {}

#[inline]
fn cmp<T: PartialOrd>(a: &T, b: &T) -> Ordering {
	a.partial_cmp(&b).unwrap()
}

#[inline]
fn dist_to<T, P>(target: P) -> impl Fn(&T, &T) -> Ordering
where
	T: Distance + Copy,
	P: Into<Point2> + Copy,
{
	let f = move |u: &T| u.distance_squared(target);
	move |a, b| f(a).partial_cmp(&f(b)).unwrap()
}

/// Helper trait for iterators of items implementing [`Distance`].
pub trait DistanceIterator: Iterator + Sized
where
	Self::Item: Distance + Copy,
{
	/// Filters all items closer than given `distance` to `target`.
	fn closer<T: Into<Point2>>(self, distance: f32, target: T) -> Closer<Self> {
		Closer::new(self, distance, target.into())
	}
	/// Filters all items further than given `distance` to `target`.
	fn further<T: Into<Point2>>(self, distance: f32, target: T) -> Further<Self> {
		Further::new(self, distance, target.into())
	}

	/// Returns closest to `target` item in iterator.
	fn closest<T: Into<Point2>>(self, target: T) -> Option<Self::Item> {
		let target = target.into();
		self.min_by(dist_to(target))
	}
	/// Returns furthest to `target` item in iterator.
	fn furthest<T: Into<Point2>>(self, target: T) -> Option<Self::Item> {
		let target = target.into();
		self.max_by(dist_to(target))
	}

	/// Returns distance to closest to `target` item in iterator.
	fn closest_distance<T: Into<Point2>>(self, target: T) -> Option<f32> {
		self.closest_distance_squared(target).map(|dist| dist.sqrt())
	}
	/// Returns distance to furthest to `target` item in iterator.
	fn furthest_distance<T: Into<Point2>>(self, target: T) -> Option<f32> {
		self.furthest_distance_squared(target).map(|dist| dist.sqrt())
	}

	/// Returns squared distance to closest to `target` item in iterator.
	fn closest_distance_squared<T: Into<Point2>>(self, target: T) -> Option<f32> {
		let target = target.into();
		self.map(|u| u.distance_squared(target)).min_by(cmp)
	}
	/// Returns squared distance to furthest to `target` item in iterator.
	fn furthest_distance_squared<T: Into<Point2>>(self, target: T) -> Option<f32> {
		let target = target.into();
		self.map(|u| u.distance_squared(target)).max_by(cmp)
	}

	/// Returns iterator of items sorted by distance to `target`.
	///
	/// This sort is stable (i.e., does not reorder equal elements) and `O(n * log(n))` worst-case.
	///
	/// When applicable, unstable sorting is preferred because it is generally faster than stable sorting
	/// and it doesn't allocate auxiliary memory. See [`sort_unstable_by_distance`](Self::sort_unstable_by_distance).
	fn sort_by_distance<T: Into<Point2>>(self, target: T) -> IntoIter<Self::Item> {
		let mut v: Vec<_> = self.collect();
		let target = target.into();
		v.sort_by(dist_to(target));
		v.into_iter()
	}
	/// Returns iterator of items sorted by distance to `target`.
	///
	/// This sort is unstable (i.e., may reorder equal elements),
	/// in-place (i.e., does not allocate), and `O(n * log(n))` worst-case.
	fn sort_unstable_by_distance<T: Into<Point2>>(self, target: T) -> IntoIter<Self::Item> {
		let mut v: Vec<_> = self.collect();
		let target = target.into();
		v.sort_unstable_by(dist_to(target));
		v.into_iter()
	}
}

/// Helper trait for sorting by distance `slice` and `Vec` of elements implementing [`Distance`].
pub trait DistanceSlice<T> {
	/// Sorts slice by distance to target.
	///
	/// This sort is stable (i.e., does not reorder equal elements) and `O(n * log(n))` worst-case.
	///
	/// When applicable, unstable sorting is preferred because it is generally faster than stable sorting
	/// and it doesn't allocate auxiliary memory. See [`sort_unstable_by_distance`](Self::sort_unstable_by_distance).
	fn sort_by_distance<P: Into<Point2>>(&mut self, target: P);
	/// Sorts slice by distance to target.
	///
	/// This sort is unstable (i.e., may reorder equal elements),
	/// in-place (i.e., does not allocate), and `O(n * log(n))` worst-case.
	fn sort_unstable_by_distance<P: Into<Point2>>(&mut self, target: P);
}

/// Helper trait for iterator of points, used to find center of these points.
pub trait Center: Iterator + Sized
where
	Self::Item: Into<Point2>,
{
	/// Returns center of all iterated points or `None` if iterator is empty.
	fn center(self) -> Option<Point2> {
		let (sum, len) = self.fold((Point2::default(), 0), |(sum, len), p| (sum + p.into(), len + 1));
		if len > 0 {
			Some(sum / len as f32)
		} else {
			None
		}
	}
}

// Implementations
impl<I> Center for I
where
	I: Iterator + Sized,
	I::Item: Into<Point2>,
{
}

impl<I> DistanceIterator for I
where
	I: Iterator + Sized,
	I::Item: Distance + Copy,
{
}

impl<T: Distance + Copy> DistanceSlice<T> for [T] {
	fn sort_by_distance<P: Into<Point2>>(&mut self, target: P) {
		let target = target.into();
		self.sort_by(dist_to(target))
	}
	fn sort_unstable_by_distance<P: Into<Point2>>(&mut self, target: P) {
		let target = target.into();
		self.sort_unstable_by(dist_to(target))
	}
}

// Macros to generate iterator implementation here

macro_rules! iterator_methods {
	() => {
		#[inline]
		fn next(&mut self) -> Option<Self::Item> {
			let pred = self.predicate();
			self.iter.find(pred)
		}

		#[inline]
		fn size_hint(&self) -> (usize, Option<usize>) {
			(0, self.iter.size_hint().1)
		}

		#[inline]
		fn count(self) -> usize {
			let pred = self.predicate();
			self.iter.map(|u| pred(&u) as usize).sum()
		}

		#[inline]
		fn fold<Acc, Fold>(self, init: Acc, fold: Fold) -> Acc
		where
			Fold: FnMut(Acc, Self::Item) -> Acc,
		{
			let pred = self.predicate();
			self.iter.fold(init, filter_fold(pred, fold))
		}
	};
}

macro_rules! double_ended_iterator_methods {
	() => {
		#[inline]
		fn next_back(&mut self) -> Option<Self::Item> {
			let pred = self.predicate();
			self.iter.rfind(pred)
		}

		#[inline]
		fn rfold<Acc, Fold>(self, init: Acc, fold: Fold) -> Acc
		where
			Fold: FnMut(Acc, Self::Item) -> Acc,
		{
			let pred = self.predicate();
			self.iter.rfold(init, filter_fold(pred, fold))
		}
	};
}

macro_rules! impl_simple_iterator {
	($name:ident) => {
		impl<I> Iterator for $name<I>
		where
			I: Iterator,
			I::Item: Distance + Copy,
		{
			type Item = I::Item;

			iterator_methods!();
		}

		impl<I> DoubleEndedIterator for $name<I>
		where
			I: DoubleEndedIterator,
			I::Item: Distance + Copy,
		{
			double_ended_iterator_methods!();
		}
	};
}

// Iterator adaptors here

/// An iterator that filters items closer than given distance to target.
#[derive(Clone)]
pub struct Closer<I> {
	iter: I,
	distance: f32,
	target: Point2,
}
impl<I> Closer<I> {
	fn new(iter: I, distance: f32, target: Point2) -> Self {
		Self {
			iter,
			distance,
			target,
		}
	}

	fn predicate<T: Distance + Copy>(&self) -> impl Fn(&T) -> bool {
		let distance = self.distance;
		let target = self.target;
		move |u| u.is_closer(distance, target)
	}
}
impl_simple_iterator!(Closer);

/// An iterator that filters items further than given distance to target.
#[derive(Clone)]
pub struct Further<I> {
	iter: I,
	distance: f32,
	target: Point2,
}
impl<I> Further<I> {
	fn new(iter: I, distance: f32, target: Point2) -> Self {
		Self {
			iter,
			distance,
			target,
		}
	}

	fn predicate<T: Distance + Copy>(&self) -> impl Fn(&T) -> bool {
		let distance = self.distance;
		let target = self.target;
		move |u| u.is_further(distance, target)
	}
}
impl_simple_iterator!(Further);
