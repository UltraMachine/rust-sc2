//! Iterator adaptors for Units.

use super::Container;
use crate::{ids::UnitTypeId, unit::Unit};
use indexmap::map::IntoIter;
use std::borrow::Borrow;

/// Owned iterator over Units.
pub struct IntoUnits(pub(super) IntoIter<u64, Unit>);

impl Iterator for IntoUnits {
	type Item = Unit;

	#[inline]
	fn next(&mut self) -> Option<Self::Item> {
		self.0.next().map(|x| x.1)
	}

	#[inline]
	fn size_hint(&self) -> (usize, Option<usize>) {
		self.0.size_hint()
	}

	#[inline]
	fn count(self) -> usize {
		self.0.len()
	}

	#[inline]
	fn nth(&mut self, n: usize) -> Option<Self::Item> {
		self.0.nth(n).map(|x| x.1)
	}

	#[inline]
	fn last(mut self) -> Option<Self::Item> {
		self.next_back()
	}
}

impl DoubleEndedIterator for IntoUnits {
	fn next_back(&mut self) -> Option<Self::Item> {
		self.0.next_back().map(|x| x.1)
	}
}

impl ExactSizeIterator for IntoUnits {
	fn len(&self) -> usize {
		self.0.len()
	}
}

// Macros to generate iterator implementation here

pub(crate) fn filter_fold<T, Acc>(
	mut pred: impl FnMut(&T) -> bool,
	mut fold: impl FnMut(Acc, T) -> Acc,
) -> impl FnMut(Acc, T) -> Acc {
	move |acc, u| if pred(&u) { fold(acc, u) } else { acc }
}

macro_rules! iterator_methods {
	() => {
		#[inline]
		fn next(&mut self) -> Option<Self::Item> {
			let pred = self.predicate();
			self.iter.find(|u| pred(u.borrow()))
		}

		#[inline]
		fn size_hint(&self) -> (usize, Option<usize>) {
			(0, self.iter.size_hint().1)
		}

		#[inline]
		fn count(self) -> usize {
			let pred = self.predicate();
			self.iter.map(|u| pred(u.borrow()) as usize).sum()
		}

		#[inline]
		fn fold<Acc, Fold>(self, init: Acc, fold: Fold) -> Acc
		where
			Fold: FnMut(Acc, Self::Item) -> Acc,
		{
			let pred = self.predicate();
			self.iter
				.fold(init, filter_fold(|u| pred(u.borrow()), fold))
		}
	};
}

macro_rules! double_ended_iterator_methods {
	() => {
		#[inline]
		fn next_back(&mut self) -> Option<Self::Item> {
			let pred = self.predicate();
			self.iter.rfind(|u| pred(u.borrow()))
		}

		#[inline]
		fn rfold<Acc, Fold>(self, init: Acc, fold: Fold) -> Acc
		where
			Fold: FnMut(Acc, Self::Item) -> Acc,
		{
			let pred = self.predicate();
			self.iter
				.rfold(init, filter_fold(|u| pred(u.borrow()), fold))
		}
	};
}

macro_rules! impl_simple_iterator {
	($name:ident $(<$a:lifetime>)?) => {
		impl<$($a,)? I> Iterator for $name<$($a,)? I>
		where
			I: Iterator,
			I::Item: Borrow<Unit>,
		{
			type Item = I::Item;

			iterator_methods!();
		}

		impl<$($a,)? I> DoubleEndedIterator for $name<$($a,)? I>
		where
			I: DoubleEndedIterator,
			I::Item: Borrow<Unit>,
		{
			double_ended_iterator_methods!();
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

// Iterator adaptors here

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

impl<'a, I, T> Iterator for FindTags<'a, I, T>
where
	I: Iterator,
	I::Item: Borrow<Unit>,
	T: Container<u64>,
{
	type Item = I::Item;

	iterator_methods!();
}

impl<'a, I, T> DoubleEndedIterator for FindTags<'a, I, T>
where
	I: DoubleEndedIterator,
	I::Item: Borrow<Unit>,
	T: Container<u64>,
{
	double_ended_iterator_methods!();
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

impl<'a, I, T> Iterator for OfTypes<'a, I, T>
where
	I: Iterator,
	I::Item: Borrow<Unit>,
	T: Container<UnitTypeId>,
{
	type Item = I::Item;

	iterator_methods!();
}

impl<'a, I, T> DoubleEndedIterator for OfTypes<'a, I, T>
where
	I: DoubleEndedIterator,
	I::Item: Borrow<Unit>,
	T: Container<UnitTypeId>,
{
	double_ended_iterator_methods!();
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

impl<'a, I, T> Iterator for ExcludeTypes<'a, I, T>
where
	I: Iterator,
	I::Item: Borrow<Unit>,
	T: Container<UnitTypeId>,
{
	type Item = I::Item;

	iterator_methods!();
}

impl<'a, I, T> DoubleEndedIterator for ExcludeTypes<'a, I, T>
where
	I: DoubleEndedIterator,
	I::Item: Borrow<Unit>,
	T: Container<UnitTypeId>,
{
	double_ended_iterator_methods!();
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

/// Helper trait for iterators over units.
pub trait UnitsIterator: Iterator + Sized
where
	Self::Item: Borrow<Unit>,
{
	/// Searches for unit with given tag and returns it if found.
	fn find_tag(mut self, tag: u64) -> Option<Self::Item> {
		self.find(|u| u.borrow().tag() == tag)
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

impl<I> UnitsIterator for I
where
	I: Iterator + Sized,
	I::Item: Borrow<Unit>,
{
}
