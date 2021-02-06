//! Parallelism for Units collection.

use super::{cmp, cmp_by2, Container, FxIndexMap, Units};
use crate::{distance::Distance, geometry::Point2, ids::UnitTypeId, unit::Unit};
use indexmap::map::rayon::{ParIter, ParIterMut, ParKeys, ParValues, ParValuesMut};
use rayon::{iter::plumbing::*, prelude::*};
use std::{borrow::Borrow, cmp::Ordering, iter::Sum};

#[inline]
fn cmp_by<U, T, F>(f: F) -> impl Fn(&&U, &&U) -> Ordering
where
	T: PartialOrd,
	F: Fn(&U) -> T + Send + Sync,
{
	move |a, b| f(a).partial_cmp(&f(b)).unwrap()
}

impl Units {
	/// Returns parallel iterator over the units of the collection.
	#[inline]
	pub fn par_iter(&self) -> ParValues<u64, Unit> {
		self.0.par_values()
	}

	/// Returns mutable parallel iterator over the units of the collection.
	#[inline]
	pub fn par_iter_mut(&mut self) -> ParValuesMut<u64, Unit> {
		self.0.par_values_mut()
	}

	/// Returns parallel iterator over (tag, unit) pairs of the collection.
	#[inline]
	pub fn par_pairs(&self) -> ParIter<u64, Unit> {
		self.0.par_iter()
	}

	/// Returns mutable parallel iterator over (tag, unit) pairs of the collection.
	#[inline]
	pub fn par_pairs_mut(&mut self) -> ParIterMut<u64, Unit> {
		self.0.par_iter_mut()
	}

	/// Returns parallel iterator over unit tags of the collection.
	#[inline]
	pub fn par_tags(&self) -> ParKeys<u64, Unit> {
		self.0.par_keys()
	}

	/// Leaves only units that match given predicate and makes new collection of them.
	///
	/// Warning: This method will clone units in order to create a new collection
	/// and will be evaluated initially. When applicable prefer using [`filter`]
	/// on the iterator over units, since it's lazily evaluated and doesn't do any cloning operations.
	///
	/// [`filter`]: Iterator::filter
	pub fn filter<F>(&self, f: F) -> Self
	where
		F: Fn(&&Unit) -> bool + Sync + Send,
	{
		Self(self.par_iter().filter(f).map(|u| (u.tag(), u.clone())).collect())
	}

	/// Leaves only units of given types and makes a new collection of them.
	///
	/// Warning: This method will clone units in order to create a new collection
	/// and will be evaluated initially. When applicable prefer using [`of_types`]
	/// on the iterator over units, since it's lazily evaluated and doesn't do any cloning operations.
	///
	/// [`of_types`]: super::UnitsIterator::of_types
	pub fn of_types<T: Container<UnitTypeId> + Sync>(&self, types: &T) -> Self {
		self.filter(|u| types.contains(&u.type_id()))
	}

	/// Excludes units of given types and makes a new collection of remaining units.
	///
	/// Warning: This method will clone units in order to create a new collection
	/// and will be evaluated initially. When applicable prefer using [`exclude_types`]
	/// on the iterator over units, since it's lazily evaluated and doesn't do any cloning operations.
	///
	/// [`exclude_types`]: super::UnitsIterator::exclude_types
	pub fn exclude_types<T: Container<UnitTypeId> + Sync>(&self, types: &T) -> Self {
		self.filter(|u| !types.contains(&u.type_id()))
	}

	/// Leaves only units closer than given distance to target and makes new collection of them.
	///
	/// Warning: This method will clone units in order to create a new collection
	/// and will be evaluated initially. When applicable prefer using [`closer`]
	/// on the iterator over units, since it's lazily evaluated and doesn't do any cloning operations.
	///
	/// [`closer`]: crate::distance::DistanceIterator::closer
	pub fn closer<P: Into<Point2> + Copy + Sync>(&self, distance: f32, target: P) -> Self {
		self.filter(|u| u.is_closer(distance, target))
	}
	/// Leaves only units further than given distance to target and makes new collection of them.
	///
	/// Warning: This method will clone units in order to create a new collection
	/// and will be evaluated initially. When applicable prefer using [`further`]
	/// on the iterator over units, since it's lazily evaluated and doesn't do any cloning operations.
	///
	/// [`further`]: crate::distance::DistanceIterator::further
	pub fn further<P: Into<Point2> + Copy + Sync>(&self, distance: f32, target: P) -> Self {
		self.filter(|u| u.is_further(distance, target))
	}

	/// Returns closest from the collection unit to given target.
	pub fn closest<P: Into<Point2> + Copy + Sync>(&self, target: P) -> Option<&Unit> {
		self.min(|u| u.distance_squared(target))
	}
	/// Returns furthest from the collection unit to given target.
	pub fn furthest<P: Into<Point2> + Copy + Sync>(&self, target: P) -> Option<&Unit> {
		self.max(|u| u.distance_squared(target))
	}

	/// Returns distance from closest unit in the collection to given target.
	pub fn closest_distance<P: Into<Point2> + Copy + Sync>(&self, target: P) -> Option<f32> {
		self.min_value(|u| u.distance_squared(target))
			.map(|dist| dist.sqrt())
	}
	/// Returns distance from furthest unit in the collection to given target.
	pub fn furthest_distance<P: Into<Point2> + Copy + Sync>(&self, target: P) -> Option<f32> {
		self.max_value(|u| u.distance_squared(target))
			.map(|dist| dist.sqrt())
	}

	/// Returns squared distance from closest unit in the collection to given target.
	pub fn closest_distance_squared<P: Into<Point2> + Copy + Sync>(&self, target: P) -> Option<f32> {
		self.min_value(|u| u.distance_squared(target))
	}
	/// Returns squared distance from furthest unit in the collection to given target.
	pub fn furthest_distance_squared<P: Into<Point2> + Copy + Sync>(&self, target: P) -> Option<f32> {
		self.max_value(|u| u.distance_squared(target))
	}

	/// Returns sum of given unit values.
	pub fn sum<T, F>(&self, f: F) -> T
	where
		T: Sum + Send,
		F: Fn(&Unit) -> T + Send + Sync,
	{
		self.par_iter().map(f).sum::<T>()
	}

	/// Returns unit with minimum given predicate.
	pub fn min<T, F>(&self, f: F) -> Option<&Unit>
	where
		T: PartialOrd,
		F: Fn(&Unit) -> T + Send + Sync,
	{
		self.par_iter().min_by(cmp_by(f))
	}
	/// Returns minimum of given unit values.
	pub fn min_value<T, F>(&self, f: F) -> Option<T>
	where
		T: PartialOrd + Send,
		F: Fn(&Unit) -> T + Send + Sync,
	{
		self.par_iter().map(f).min_by(cmp)
	}

	/// Returns unit with maximum given predicate.
	pub fn max<T, F>(&self, f: F) -> Option<&Unit>
	where
		T: PartialOrd,
		F: Fn(&Unit) -> T + Sync + Send,
	{
		self.par_iter().max_by(cmp_by(f))
	}
	/// Returns maximum of given unit values.
	pub fn max_value<T, F>(&self, f: F) -> Option<T>
	where
		T: PartialOrd + Send,
		F: Fn(&Unit) -> T + Sync + Send,
	{
		self.par_iter().map(f).max_by(cmp)
	}

	/// Parallelly sorts the collection by given function.
	pub fn par_sort<T, F>(&mut self, f: F)
	where
		T: PartialOrd,
		F: Fn(&Unit) -> T + Sync + Send,
	{
		self.0.par_sort_by(cmp_by2(f));
	}
	/// Makes new collection parallelly sorted by given function.
	/// Leaves original collection untouched.
	pub fn par_sorted<T, F>(&self, f: F) -> Self
	where
		T: PartialOrd,
		F: Fn(&Unit) -> T + Sync + Send,
	{
		let mut sorted = self.clone();
		sorted.0.par_sort_by(cmp_by2(f));
		sorted
	}
}

impl IntoParallelIterator for Units {
	type Item = Unit;
	type Iter = IntoParUnits;

	#[inline]
	fn into_par_iter(self) -> Self::Iter {
		IntoParUnits(self.0)
	}
}
impl<'a> IntoParallelIterator for &'a Units {
	type Item = &'a Unit;
	type Iter = ParValues<'a, u64, Unit>;

	#[inline]
	fn into_par_iter(self) -> Self::Iter {
		self.0.par_values()
	}
}
impl<'a> IntoParallelIterator for &'a mut Units {
	type Item = &'a mut Unit;
	type Iter = ParValuesMut<'a, u64, Unit>;

	#[inline]
	fn into_par_iter(self) -> Self::Iter {
		self.0.par_values_mut()
	}
}

impl ParallelExtend<Unit> for Units {
	#[inline]
	fn par_extend<T: IntoParallelIterator<Item = Unit>>(&mut self, par_iter: T) {
		self.0.par_extend(par_iter.into_par_iter().map(|u| (u.tag(), u)));
	}
}
impl ParallelExtend<(u64, Unit)> for Units {
	#[inline]
	fn par_extend<T: IntoParallelIterator<Item = (u64, Unit)>>(&mut self, par_iter: T) {
		self.0.par_extend(par_iter);
	}
}

impl FromParallelIterator<Unit> for Units {
	#[inline]
	fn from_par_iter<I: IntoParallelIterator<Item = Unit>>(par_iter: I) -> Self {
		Self(par_iter.into_par_iter().map(|u| (u.tag(), u)).collect())
	}
}
impl FromParallelIterator<(u64, Unit)> for Units {
	#[inline]
	fn from_par_iter<I: IntoParallelIterator<Item = (u64, Unit)>>(par_iter: I) -> Self {
		Self(par_iter.into_par_iter().collect())
	}
}

/// Helper trait for parallel iterators over units.
pub trait ParUnitsIterator: ParallelIterator
where
	Self::Item: Borrow<Unit>,
{
	/// Searches for unit with given tag and returns it if found.
	fn find_tag(self, tag: u64) -> Option<Self::Item> {
		self.find_any(|u| u.borrow().tag() == tag)
	}
	/// Leaves only units with given tags.
	fn find_tags<T: Container<u64>>(self, tags: &T) -> FindTags<Self, T> {
		FindTags::new(self, tags)
	}
	/// Leaves only units of given type.
	fn of_type(self, unit_type: UnitTypeId) -> OfType<Self> {
		OfType::new(self, unit_type)
	}
	/// Excludes units of given type.
	fn exclude_type(self, unit_type: UnitTypeId) -> ExcludeType<Self> {
		ExcludeType::new(self, unit_type)
	}
	/// Leaves only units of given types.
	fn of_types<T: Container<UnitTypeId>>(self, types: &T) -> OfTypes<Self, T> {
		OfTypes::new(self, types)
	}
	/// Excludes units of given types.
	fn exclude_types<T: Container<UnitTypeId>>(self, types: &T) -> ExcludeTypes<Self, T> {
		ExcludeTypes::new(self, types)
	}
	/// Leaves only non-flying units.
	fn ground(self) -> Ground<Self> {
		Ground::new(self)
	}
	/// Leaves only flying units.
	fn flying(self) -> Flying<Self> {
		Flying::new(self)
	}
	/// Leaves only ready structures.
	fn ready(self) -> Ready<Self> {
		Ready::new(self)
	}
	/// Leaves only structures in-progress.
	fn not_ready(self) -> NotReady<Self> {
		NotReady::new(self)
	}
	/// Leaves only units with no orders.
	fn idle(self) -> Idle<Self> {
		Idle::new(self)
	}
	/// Leaves only units with no orders or that almost finished their orders.
	fn almost_idle(self) -> AlmostIdle<Self> {
		AlmostIdle::new(self)
	}
	/// Leaves only units with no orders.
	/// Unlike [`idle`](Self::idle) this takes reactor on terran buildings into account.
	fn unused(self) -> Unused<Self> {
		Unused::new(self)
	}
	/// Leaves only units with no orders or that almost finished their orders.
	/// Unlike [`almost_idle`](Self::almost_idle) this takes reactor on terran buildings into account.
	fn almost_unused(self) -> AlmostUnused<Self> {
		AlmostUnused::new(self)
	}
	/// Leaves only units visible on current step.
	fn visible(self) -> Visible<Self> {
		Visible::new(self)
	}
	/// Leaves only units in attack range of given unit.
	fn in_range_of(self, unit: &Unit, gap: f32) -> InRangeOf<Self> {
		InRangeOf::new(self, unit, gap)
	}
	/// Leaves only units that are close enough to attack given unit.
	fn in_range(self, unit: &Unit, gap: f32) -> InRange<Self> {
		InRange::new(self, unit, gap)
	}
	/// Leaves only units in attack range of given unit.
	/// Unlike [`in_range_of`](Self::in_range_of) this takes range upgrades into account.
	fn in_real_range_of(self, unit: &Unit, gap: f32) -> InRealRangeOf<Self> {
		InRealRangeOf::new(self, unit, gap)
	}
	/// Leaves only units that are close enough to attack given unit.
	/// Unlike [`in_range`](Self::in_range) this takes range upgrades into account.
	fn in_real_range(self, unit: &Unit, gap: f32) -> InRealRange<Self> {
		InRealRange::new(self, unit, gap)
	}
}

impl<I> ParUnitsIterator for I
where
	I: ParallelIterator,
	I::Item: Borrow<Unit>,
{
}

/// Owned parallel iterator over Units.
pub struct IntoParUnits(FxIndexMap<u64, Unit>);

impl ParallelIterator for IntoParUnits {
	type Item = Unit;

	fn drive_unindexed<C>(self, consumer: C) -> C::Result
	where
		C: UnindexedConsumer<Self::Item>,
	{
		self.0.into_par_iter().map(|x| x.1).drive_unindexed(consumer)
	}

	fn opt_len(&self) -> Option<usize> {
		Some(self.0.len())
	}
}

impl IndexedParallelIterator for IntoParUnits {
	fn drive<C>(self, consumer: C) -> C::Result
	where
		C: Consumer<Self::Item>,
	{
		self.0.into_par_iter().map(|x| x.1).drive(consumer)
	}

	fn len(&self) -> usize {
		self.0.len()
	}

	fn with_producer<CB>(self, callback: CB) -> CB::Output
	where
		CB: ProducerCallback<Self::Item>,
	{
		self.0.into_par_iter().map(|x| x.1).with_producer(callback)
	}
}

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
	($name:ident $(<$a:lifetime>)?) => {
		impl<$($a,)? I> ParallelIterator for $name<$($a,)? I>
		where
			I: ParallelIterator,
			I::Item: Borrow<Unit>,
		{
			type Item = I::Item;

			iterator_methods!();
		}
	};
}

macro_rules! make_simple_iterator {
	($(#[$attr:meta])* $name:ident, $pred:expr) => {
		$(#[$attr])*
		#[derive(Clone)]
		pub struct $name<I> {
			iter: I,
		}

		impl<I> $name<I> {
			pub(super) fn new(iter: I) -> Self {
				Self { iter }
			}

			fn predicate(&self) -> impl Fn(&Unit) -> bool {
				$pred
			}
		}

		impl_simple_iterator!($name);
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
	P: Fn(&Unit) -> bool + Sync,
	T: Borrow<Unit>,
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
	P: Fn(&Unit) -> bool + Sync,
	T: Borrow<Unit>,
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
	P: Fn(&Unit) -> bool + 'p,
	T: Borrow<Unit>,
{
	type Result = C::Result;

	fn consume(self, item: T) -> Self {
		let filter_op = self.filter_op;
		if filter_op(item.borrow()) {
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

/// An iterator that filters units with given tags.
#[derive(Clone)]
pub struct FindTags<'a, I, T> {
	iter: I,
	tags: &'a T,
}
impl<'a, I, T: Container<u64>> FindTags<'a, I, T> {
	pub(super) fn new(iter: I, tags: &'a T) -> Self {
		Self { iter, tags }
	}

	fn predicate(&self) -> impl Fn(&Unit) -> bool + 'a {
		let tags = self.tags;
		move |u| tags.contains(&u.tag())
	}
}

impl<'a, I, T> ParallelIterator for FindTags<'a, I, T>
where
	I: ParallelIterator,
	I::Item: Borrow<Unit>,
	T: Container<u64> + Sync,
{
	type Item = I::Item;

	iterator_methods!();
}

/// An iterator that filters units of given type.
#[derive(Clone)]
pub struct OfType<I> {
	iter: I,
	unit_type: UnitTypeId,
}
impl<I> OfType<I> {
	pub(super) fn new(iter: I, unit_type: UnitTypeId) -> Self {
		Self { iter, unit_type }
	}

	fn predicate(&self) -> impl Fn(&Unit) -> bool {
		let unit_type = self.unit_type;
		move |u| u.type_id() == unit_type
	}
}
impl_simple_iterator!(OfType);

/// An iterator that filters out units of given type.
#[derive(Clone)]
pub struct ExcludeType<I> {
	iter: I,
	unit_type: UnitTypeId,
}
impl<I> ExcludeType<I> {
	pub(super) fn new(iter: I, unit_type: UnitTypeId) -> Self {
		Self { iter, unit_type }
	}

	fn predicate(&self) -> impl Fn(&Unit) -> bool {
		let unit_type = self.unit_type;
		move |u| u.type_id() != unit_type
	}
}
impl_simple_iterator!(ExcludeType);

/// An iterator that filters units of given types.
#[derive(Clone)]
pub struct OfTypes<'a, I, T> {
	iter: I,
	types: &'a T,
}
impl<'a, I, T: Container<UnitTypeId>> OfTypes<'a, I, T> {
	pub(super) fn new(iter: I, types: &'a T) -> Self {
		Self { iter, types }
	}

	fn predicate(&self) -> impl Fn(&Unit) -> bool + 'a {
		let types = self.types;
		move |u| types.contains(&u.type_id())
	}
}

impl<'a, I, T> ParallelIterator for OfTypes<'a, I, T>
where
	I: ParallelIterator,
	I::Item: Borrow<Unit>,
	T: Container<UnitTypeId> + Sync,
{
	type Item = I::Item;

	iterator_methods!();
}

/// An iterator that filters out units of given types.
#[derive(Clone)]
pub struct ExcludeTypes<'a, I, T> {
	iter: I,
	types: &'a T,
}
impl<'a, I, T: Container<UnitTypeId>> ExcludeTypes<'a, I, T> {
	pub(super) fn new(iter: I, types: &'a T) -> Self {
		Self { iter, types }
	}

	fn predicate(&self) -> impl Fn(&Unit) -> bool + 'a {
		let types = self.types;
		move |u| !types.contains(&u.type_id())
	}
}

impl<'a, I, T> ParallelIterator for ExcludeTypes<'a, I, T>
where
	I: ParallelIterator,
	I::Item: Borrow<Unit>,
	T: Container<UnitTypeId> + Sync,
{
	type Item = I::Item;

	iterator_methods!();
}

make_simple_iterator!(
	/// An iterator that filters ground units.
	Ground,
	|u| !u.is_flying()
);

make_simple_iterator!(
	/// An iterator that filters flying units.
	Flying,
	|u| u.is_flying()
);

make_simple_iterator!(
	/// An iterator that filters ready units and structures.
	Ready,
	|u| u.is_ready()
);

make_simple_iterator!(
	/// An iterator that filters units structures in-progress.
	NotReady,
	|u| !u.is_ready()
);

make_simple_iterator!(
	/// An iterator that filters units with no orders.
	Idle,
	|u| u.is_idle()
);

make_simple_iterator!(
	/// An iterator that filters units with no orders or almost finished orders.
	AlmostIdle,
	|u| u.is_almost_idle()
);

make_simple_iterator!(
	/// An iterator that filters units with no orders (this also handles buildings with reactor).
	Unused,
	|u| u.is_unused()
);

make_simple_iterator!(
	/// An iterator that filters units with no orders or almost finished orders
	/// (this also handles buildings with reactor).
	AlmostUnused,
	|u| u.is_almost_unused()
);

make_simple_iterator!(
	/// An iterator that filters units units visible on current step.
	Visible,
	|u| u.is_visible()
);

/// An iterator that filters units in attack range of given unit.
#[derive(Clone)]
pub struct InRangeOf<'a, I> {
	iter: I,
	unit: &'a Unit,
	gap: f32,
}
impl<'a, I> InRangeOf<'a, I> {
	pub(super) fn new(iter: I, unit: &'a Unit, gap: f32) -> Self {
		Self { iter, unit, gap }
	}

	fn predicate(&self) -> impl Fn(&Unit) -> bool + 'a {
		let unit = self.unit;
		let gap = self.gap;
		move |u| unit.in_range(u, gap)
	}
}
impl_simple_iterator!(InRangeOf<'a>);

/// An iterator that filters units close enough to attack given unit.
#[derive(Clone)]
pub struct InRange<'a, I> {
	iter: I,
	unit: &'a Unit,
	gap: f32,
}
impl<'a, I> InRange<'a, I> {
	pub(super) fn new(iter: I, unit: &'a Unit, gap: f32) -> Self {
		Self { iter, unit, gap }
	}

	fn predicate(&self) -> impl Fn(&Unit) -> bool + 'a {
		let unit = self.unit;
		let gap = self.gap;
		move |u| u.in_range(unit, gap)
	}
}
impl_simple_iterator!(InRange<'a>);

/// An iterator that filters units in attack range of given unit (this also handles range upgrades).
#[derive(Clone)]
pub struct InRealRangeOf<'a, I> {
	iter: I,
	unit: &'a Unit,
	gap: f32,
}
impl<'a, I> InRealRangeOf<'a, I> {
	pub(super) fn new(iter: I, unit: &'a Unit, gap: f32) -> Self {
		Self { iter, unit, gap }
	}

	fn predicate(&self) -> impl Fn(&Unit) -> bool + 'a {
		let unit = self.unit;
		let gap = self.gap;
		move |u| unit.in_real_range(u, gap)
	}
}
impl_simple_iterator!(InRealRangeOf<'a>);

/// An iterator that filters units close enough to attack given unit (this also handles range upgrades).
#[derive(Clone)]
pub struct InRealRange<'a, I> {
	iter: I,
	unit: &'a Unit,
	gap: f32,
}
impl<'a, I> InRealRange<'a, I> {
	pub(super) fn new(iter: I, unit: &'a Unit, gap: f32) -> Self {
		Self { iter, unit, gap }
	}

	fn predicate(&self) -> impl Fn(&Unit) -> bool + 'a {
		let unit = self.unit;
		let gap = self.gap;
		move |u| u.in_real_range(unit, gap)
	}
}
impl_simple_iterator!(InRealRange<'a>);
