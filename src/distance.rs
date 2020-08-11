//! Traits for comparing distance between points and units.

use crate::{geometry::Point2, unit::Unit};
use std::{
	cmp::Ordering,
	iter::{Filter, FromIterator},
	vec::IntoIter,
};

/// Basic trait for comparing distance.
pub trait Distance: Sized {
	/// Calculates squared euclidean distance from `self` to `other`.
	fn distance_squared<P: Into<Point2>>(self, other: P) -> f32;

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

impl Distance for Point2 {
	#[inline]
	fn distance_squared<P: Into<Point2>>(self, other: P) -> f32 {
		let other = other.into();
		let dx = self.x - other.x;
		let dy = self.y - other.y;
		dx * dx + dy * dy
	}
}
impl Distance for &Point2 {
	#[inline]
	fn distance_squared<P: Into<Point2>>(self, other: P) -> f32 {
		(*self).distance_squared(other)
	}
}
impl Distance for &Unit {
	#[inline]
	fn distance_squared<P: Into<Point2>>(self, other: P) -> f32 {
		self.position.distance_squared(other)
	}
}

#[inline]
fn cmp<T: PartialOrd>(a: &T, b: &T) -> Ordering {
	a.partial_cmp(&b).unwrap()
}

#[inline]
fn cmp_by<T, P>(target: P) -> impl Fn(&T, &T) -> Ordering
where
	T: Distance + Copy,
	P: Into<Point2> + Copy,
{
	let f = move |u: &T| u.distance_squared(target);
	move |a, b| f(a).partial_cmp(&f(b)).unwrap()
}

/// Helper trait for iterators of items implementing [`Distance`].
pub trait DistanceIterator<'a, T>
where
	Self: Iterator<Item = T> + Sized,
	T: Distance + Copy,
{
	/// Filters all items closer than given `distance` to `target`.
	fn closer<P>(self, distance: f32, target: P) -> Filter<Self, Box<dyn FnMut(&T) -> bool + 'a>>
	where
		P: Into<Point2> + Copy + 'a,
	{
		self.filter(Box::new(move |u| u.is_closer(distance, target)))
	}
	/// Filters all items further than given `distance` to `target`.
	fn further<P>(self, distance: f32, target: P) -> Filter<Self, Box<dyn FnMut(&T) -> bool + 'a>>
	where
		P: Into<Point2> + Copy + 'a,
	{
		self.filter(Box::new(move |u| u.is_further(distance, target)))
	}

	/// Returns closest to `target` item in iterator.
	fn closest<P: Into<Point2> + Copy>(self, target: P) -> Option<T> {
		self.min_by(cmp_by(target))
	}
	/// Returns furthest to `target` item in iterator.
	fn furthest<P: Into<Point2> + Copy>(self, target: P) -> Option<T> {
		self.max_by(cmp_by(target))
	}

	/// Returns distance to closest to `target` item in iterator.
	fn closest_distance<P: Into<Point2> + Copy>(self, target: P) -> Option<f32> {
		self.closest_distance_squared(target).map(|dist| dist.sqrt())
	}
	/// Returns distance to furthest to `target` item in iterator.
	fn furthest_distance<P: Into<Point2> + Copy>(self, target: P) -> Option<f32> {
		self.furthest_distance_squared(target).map(|dist| dist.sqrt())
	}

	/// Returns squared distance to closest to `target` item in iterator.
	fn closest_distance_squared<P: Into<Point2> + Copy>(self, target: P) -> Option<f32> {
		self.map(|u| u.distance_squared(target)).min_by(cmp)
	}
	/// Returns squared distance to furthest to `target` item in iterator.
	fn furthest_distance_squared<P: Into<Point2> + Copy>(self, target: P) -> Option<f32> {
		self.map(|u| u.distance_squared(target)).max_by(cmp)
	}

	/// Returns iterator of items sorted by distance to `target`.
	///
	/// This sort is stable (i.e., does not reorder equal elements) and `O(n * log(n))` worst-case.
	///
	/// When applicable, unstable sorting is preferred because it is generally faster than stable sorting
	/// and it doesn't allocate auxiliary memory. See [`sort_unstable_by_distance`](Self::sort_unstable_by_distance).
	fn sort_by_distance<P: Into<Point2> + Copy>(self, target: P) -> IntoIter<T> {
		let mut v = Vec::from_iter(self);
		v.sort_by(cmp_by(target));
		v.into_iter()
	}
	/// Returns iterator of items sorted by distance to `target`.
	///
	/// This sort is unstable (i.e., may reorder equal elements),
	/// in-place (i.e., does not allocate), and `O(n * log(n))` worst-case.
	fn sort_unstable_by_distance<P: Into<Point2> + Copy>(self, target: P) -> IntoIter<T> {
		let mut v = Vec::from_iter(self);
		v.sort_unstable_by(cmp_by(target));
		v.into_iter()
	}
}

#[cfg(feature = "rayon")]
use rayon::{iter::Filter as ParFilter, prelude::*, vec::IntoIter as IntoParIter};

/// Helper trait for parallel iterators implementing [`Distance`].
#[cfg(feature = "rayon")]
pub trait ParDistanceIterator<'a, T>
where
	Self: ParallelIterator<Item = T>,
	T: Distance + Copy + Send + Sync,
{
	/// Filters all items closer than given `distance` to `target`.
	fn closer<P>(
		self,
		distance: f32,
		target: P,
	) -> ParFilter<Self, Box<dyn Fn(&T) -> bool + Send + Sync + 'a>>
	where
		P: Into<Point2> + Copy + Send + Sync + 'a,
	{
		self.filter(Box::new(move |u| u.is_closer(distance, target)))
	}
	/// Filters all items further than given `distance` to `target`.
	fn further<P>(
		self,
		distance: f32,
		target: P,
	) -> ParFilter<Self, Box<dyn Fn(&T) -> bool + Send + Sync + 'a>>
	where
		P: Into<Point2> + Copy + Send + Sync + 'a,
	{
		self.filter(Box::new(move |u| u.is_further(distance, target)))
	}

	/// Returns closest to `target` item in iterator.
	fn closest<P: Into<Point2> + Copy + Sync + Send>(self, target: P) -> Option<T> {
		self.min_by(cmp_by(target))
	}
	/// Returns furthest to `target` item in iterator.
	fn furthest<P: Into<Point2> + Copy + Sync + Send>(self, target: P) -> Option<T> {
		self.max_by(cmp_by(target))
	}

	/// Returns distance to closest to `target` item in iterator.
	fn closest_distance<P: Into<Point2> + Copy + Sync>(self, target: P) -> Option<f32> {
		self.closest_distance_squared(target).map(|dist| dist.sqrt())
	}
	/// Returns distance to furthest to target item in iterator.
	fn furthest_distance<P: Into<Point2> + Copy + Sync>(self, target: P) -> Option<f32> {
		self.furthest_distance_squared(target).map(|dist| dist.sqrt())
	}

	/// Returns squared distance to closest to `target` item in iterator.
	fn closest_distance_squared<P: Into<Point2> + Copy + Sync>(self, target: P) -> Option<f32> {
		self.map(|u| u.distance_squared(target)).min_by(cmp)
	}
	/// Returns squared distance to furthest to target item in iterator.
	fn furthest_distance_squared<P: Into<Point2> + Copy + Sync>(self, target: P) -> Option<f32> {
		self.map(|u| u.distance_squared(target)).max_by(cmp)
	}

	/// Returns iterator of items sorted by distance to `target`.
	///
	/// This sort is stable (i.e. does not reorder equal elements) and `O(n log n)` worst-case.
	///
	/// When applicable, unstable sorting is preferred because it is generally faster than stable sorting
	/// and it doesn't allocate auxiliary memory. See [`sort_unstable_by_distance`](Self::sort_unstable_by_distance).
	fn sort_by_distance<P: Into<Point2> + Copy + Sync>(self, target: P) -> IntoParIter<T> {
		let mut v = Vec::from_par_iter(self);
		v.par_sort_by(cmp_by(target));
		v.into_par_iter()
	}
	/// Returns iterator of items sorted by distance to target.
	///
	/// This sort is unstable (i.e. may reorder equal elements),
	/// in-place (i.e. does not allocate), and `O(n log n)` worst-case.
	fn sort_unstable_by_distance<P: Into<Point2> + Copy + Sync>(self, target: P) -> IntoParIter<T> {
		let mut v = Vec::from_par_iter(self);
		v.par_sort_unstable_by(cmp_by(target));
		v.into_par_iter()
	}
}

/// Helper trait for sorting by distance `slice` and `Vec` of elements implementing [`Distance`].
pub trait DistanceSlice<T: Distance> {
	/// Sorts slice by distance to target.
	///
	/// This sort is stable (i.e., does not reorder equal elements) and `O(n * log(n))` worst-case.
	///
	/// When applicable, unstable sorting is preferred because it is generally faster than stable sorting
	/// and it doesn't allocate auxiliary memory. See [`sort_unstable_by_distance`](Self::sort_unstable_by_distance).
	fn sort_by_distance<P: Into<Point2> + Copy>(&mut self, target: P);
	/// Sorts slice by distance to target.
	///
	/// This sort is unstable (i.e., may reorder equal elements),
	/// in-place (i.e., does not allocate), and `O(n * log(n))` worst-case.
	fn sort_unstable_by_distance<P: Into<Point2> + Copy>(&mut self, target: P);
}

/// Helper trait for parallel sorting by distance `slice` and `Vec` of elements implementing [`Distance`].
#[cfg(feature = "rayon")]
pub trait ParDistanceSlice<T>
where
	Self: ParallelSliceMut<T>,
	T: Distance + Copy + Send,
{
	/// Sorts slice in parallel by distance to target.
	///
	/// This sort is stable (i.e. does not reorder equal elements) and `O(n log n)` worst-case.
	///
	/// When applicable, unstable sorting is preferred because it is generally faster than stable sorting
	/// and it doesn't allocate auxiliary memory.
	/// See [`par_sort_unstable_by_distance`](Self::par_sort_unstable_by_distance).
	fn par_sort_by_distance<P: Into<Point2> + Copy + Sync>(&mut self, target: P) {
		self.par_sort_by(cmp_by(target))
	}
	/// Sorts slice in parallel by distance to target.
	///
	/// This sort is unstable (i.e. may reorder equal elements),
	/// in-place (i.e. does not allocate), and `O(n log n)` worst-case.
	fn par_sort_unstable_by_distance<P: Into<Point2> + Copy + Sync>(&mut self, target: P) {
		self.par_sort_unstable_by(cmp_by(target))
	}
}

/// Helper trait for iterator of points, used to find center of these points.
pub trait Center<T>
where
	Self: Iterator<Item = T> + Sized,
	T: Into<Point2>,
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

/// Helper trait for parallel iterator of points, used to find center of these points.
#[cfg(feature = "rayon")]
pub trait ParCenter<T>
where
	Self: ParallelIterator<Item = T>,
	T: Into<Point2> + Send,
{
	/// Returns center of all iterated points or `None` if iterator is empty.
	fn center(self) -> Option<Point2> {
		let (sum, len) = self.map(|p| (p.into(), 1)).reduce(
			|| (Point2::default(), 0),
			|(sum1, len1), (sum2, len2)| (sum1 + sum2, len1 + len2),
		);
		if len > 0 {
			Some(sum / len as f32)
		} else {
			None
		}
	}
}

// Implementations
impl<I, T> Center<T> for I
where
	I: Iterator<Item = T> + Sized,
	T: Into<Point2>,
{
}

#[cfg(feature = "rayon")]
impl<I, T> ParCenter<T> for I
where
	I: ParallelIterator<Item = T>,
	T: Into<Point2> + Send,
{
}

impl<'a, I, T> DistanceIterator<'a, T> for I
where
	I: Iterator<Item = T> + Sized,
	T: Distance + Copy,
{
}

#[cfg(feature = "rayon")]
impl<'a, I, T> ParDistanceIterator<'a, T> for I
where
	I: ParallelIterator<Item = T>,
	T: Distance + Copy + Send + Sync,
{
}

impl<T: Distance + Copy> DistanceSlice<T> for [T] {
	fn sort_by_distance<P: Into<Point2> + Copy>(&mut self, target: P) {
		self.sort_by(cmp_by(target))
	}
	fn sort_unstable_by_distance<P: Into<Point2> + Copy>(&mut self, target: P) {
		self.sort_unstable_by(cmp_by(target))
	}
}

#[cfg(feature = "rayon")]
impl<T: Distance + Copy + Send> ParDistanceSlice<T> for [T] {}
