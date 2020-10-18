//! Parallelism for iterators over elements implementing [`Distance`](super::Distance).

use super::{cmp, dist_to, Distance};
use crate::geometry::Point2;
use rayon::{iter::plumbing::*, prelude::*, vec::IntoIter as IntoParIter};

/// Helper trait for parallel iterators implementing [`Distance`].
pub trait ParDistanceIterator: ParallelIterator
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
	/// Returns distance to furthest to target item in iterator.
	fn furthest_distance<T: Into<Point2>>(self, target: T) -> Option<f32> {
		self.furthest_distance_squared(target).map(|dist| dist.sqrt())
	}

	/// Returns squared distance to closest to `target` item in iterator.
	fn closest_distance_squared<T: Into<Point2>>(self, target: T) -> Option<f32> {
		let target = target.into();
		self.map(|u| u.distance_squared(target)).min_by(cmp)
	}
	/// Returns squared distance to furthest to target item in iterator.
	fn furthest_distance_squared<T: Into<Point2>>(self, target: T) -> Option<f32> {
		let target = target.into();
		self.map(|u| u.distance_squared(target)).max_by(cmp)
	}

	/// Returns iterator of items sorted by distance to `target`.
	///
	/// This sort is stable (i.e. does not reorder equal elements) and `O(n log n)` worst-case.
	///
	/// When applicable, unstable sorting is preferred because it is generally faster than stable sorting
	/// and it doesn't allocate auxiliary memory. See [`sort_unstable_by_distance`](Self::sort_unstable_by_distance).
	fn sort_by_distance<T: Into<Point2>>(self, target: T) -> IntoParIter<Self::Item> {
		let mut v = Vec::from_par_iter(self);
		let target = target.into();
		v.par_sort_by(dist_to(target));
		v.into_par_iter()
	}
	/// Returns iterator of items sorted by distance to target.
	///
	/// This sort is unstable (i.e. may reorder equal elements),
	/// in-place (i.e. does not allocate), and `O(n log n)` worst-case.
	fn sort_unstable_by_distance<T: Into<Point2>>(self, target: T) -> IntoParIter<Self::Item> {
		let mut v = Vec::from_par_iter(self);
		let target = target.into();
		v.par_sort_unstable_by(dist_to(target));
		v.into_par_iter()
	}
}

/// Helper trait for parallel sorting by distance `slice` and `Vec` of elements implementing [`Distance`].
pub trait ParDistanceSlice<T>: ParallelSliceMut<T>
where
	T: Distance + Copy + Send,
{
	/// Sorts slice in parallel by distance to target.
	///
	/// This sort is stable (i.e. does not reorder equal elements) and `O(n log n)` worst-case.
	///
	/// When applicable, unstable sorting is preferred because it is generally faster than stable sorting
	/// and it doesn't allocate auxiliary memory.
	/// See [`par_sort_unstable_by_distance`](Self::par_sort_unstable_by_distance).
	fn par_sort_by_distance<P: Into<Point2>>(&mut self, target: P) {
		let target = target.into();
		self.par_sort_by(dist_to(target))
	}
	/// Sorts slice in parallel by distance to target.
	///
	/// This sort is unstable (i.e. may reorder equal elements),
	/// in-place (i.e. does not allocate), and `O(n log n)` worst-case.
	fn par_sort_unstable_by_distance<P: Into<Point2>>(&mut self, target: P) {
		let target = target.into();
		self.par_sort_unstable_by(dist_to(target))
	}
}

/// Helper trait for parallel iterator of points, used to find center of these points.
pub trait ParCenter: ParallelIterator
where
	Self::Item: Into<Point2>,
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

impl<I> ParCenter for I
where
	I: ParallelIterator,
	I::Item: Into<Point2>,
{
}

impl<I> ParDistanceIterator for I
where
	I: ParallelIterator,
	I::Item: Distance + Copy,
{
}

impl<T: Distance + Copy + Send> ParDistanceSlice<T> for [T] {}

// Macros to generate parallel iterator implementation here

macro_rules! iterator_methods {
	() => {
		fn drive_unindexed<C>(self, consumer: C) -> C::Result
		where
			C: UnindexedConsumer<Self::Item>,
		{
			let pred = self.predicate();
			self.iter
				.drive_unindexed(FilterConsumer::new(consumer, &pred))
		}
	};
}

macro_rules! impl_simple_iterator {
	($name:ident) => {
		impl<I> ParallelIterator for $name<I>
		where
			I: ParallelIterator,
			I::Item: Distance + Copy,
		{
			type Item = I::Item;

			iterator_methods!();
		}
	};
}

// Consumer implementation

struct FilterConsumer<'p, C, P> {
	base: C,
	filter_op: &'p P,
}

impl<'p, C, P> FilterConsumer<'p, C, P> {
	fn new(base: C, filter_op: &'p P) -> Self {
		FilterConsumer { base, filter_op }
	}
}

impl<'p, T, C, P: 'p> Consumer<T> for FilterConsumer<'p, C, P>
where
	C: Consumer<T>,
	P: Fn(&T) -> bool + Sync,
	T: Distance + Copy,
{
	type Folder = FilterFolder<'p, C::Folder, P>;
	type Reducer = C::Reducer;
	type Result = C::Result;

	fn split_at(self, index: usize) -> (Self, Self, C::Reducer) {
		let (left, right, reducer) = self.base.split_at(index);
		(
			FilterConsumer::new(left, self.filter_op),
			FilterConsumer::new(right, self.filter_op),
			reducer,
		)
	}

	fn into_folder(self) -> Self::Folder {
		FilterFolder {
			base: self.base.into_folder(),
			filter_op: self.filter_op,
		}
	}

	fn full(&self) -> bool {
		self.base.full()
	}
}

impl<'p, T, C, P: 'p> UnindexedConsumer<T> for FilterConsumer<'p, C, P>
where
	C: UnindexedConsumer<T>,
	P: Fn(&T) -> bool + Sync,
	T: Distance + Copy,
{
	fn split_off_left(&self) -> Self {
		FilterConsumer::new(self.base.split_off_left(), &self.filter_op)
	}

	fn to_reducer(&self) -> Self::Reducer {
		self.base.to_reducer()
	}
}

struct FilterFolder<'p, C, P> {
	base: C,
	filter_op: &'p P,
}

impl<'p, C, P, T> Folder<T> for FilterFolder<'p, C, P>
where
	C: Folder<T>,
	P: Fn(&T) -> bool + 'p,
	T: Distance + Copy,
{
	type Result = C::Result;

	fn consume(self, item: T) -> Self {
		let filter_op = self.filter_op;
		if filter_op(&item) {
			let base = self.base.consume(item);
			FilterFolder { base, filter_op }
		} else {
			self
		}
	}

	fn complete(self) -> Self::Result {
		self.base.complete()
	}

	fn full(&self) -> bool {
		self.base.full()
	}
}

// Parallel Iterator adaptors here

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
