//! Data structures for storing units and fast filtering and finding ones that needed.
#![warn(missing_docs)]

use crate::{distance::Distance, geometry::Point2, ids::UnitTypeId, unit::Unit};
use indexmap::{
	map::{IntoIter, Iter, IterMut, Keys, Values, ValuesMut},
	IndexMap, IndexSet,
};
use rustc_hash::FxHasher;
use std::{
	hash::BuildHasherDefault,
	iter::{FromIterator, Sum},
	ops::{Index, IndexMut},
};

type FxIndexMap<K, V> = IndexMap<K, V, BuildHasherDefault<FxHasher>>;

/// Structured collection of all possible units. Can be accessed through [`units`] bot's field.
///
/// [`units`]: crate::bot::Bot::units
#[derive(Default, Clone)]
pub struct AllUnits {
	/// All the units including owned, enemies and neutral.
	pub all: Units,
	/// Your's only units.
	pub my: PlayerUnits,
	/// Opponent's units, on current step.
	pub enemy: PlayerUnits,
	/// Opponent's units, but contains some units from previous steps, marked as snapshots or burrowed.
	pub cached: PlayerUnits,
	/// All mineral fields on the map.
	pub mineral_fields: Units,
	/// All vespene geysers on the map.
	pub vespene_geysers: Units,
	/// All resources (both minerals and geysers) on the map.
	pub resources: Units,
	/// Destructable rocks and other trash.
	pub destructables: Units,
	/// Watchtowers reveal area around them if there're any ground units near.
	pub watchtowers: Units,
	/// Inhubitor zones slow down movement speed of nearby units.
	pub inhibitor_zones: Units,
}
impl AllUnits {
	pub(crate) fn clear(&mut self) {
		self.all.clear();
		self.my.clear();
		self.enemy.clear();
		self.mineral_fields.clear();
		self.vespene_geysers.clear();
		self.resources.clear();
		self.destructables.clear();
		self.watchtowers.clear();
		self.inhibitor_zones.clear();
	}
}

/// Structured player units (yours or opponent's).
#[derive(Default, Clone)]
pub struct PlayerUnits {
	/// All player units (includes both units and structures).
	pub all: Units,
	/// Units only, without structures.
	pub units: Units,
	/// Structures only.
	pub structures: Units,
	/// From all structures only townhalls here.
	pub townhalls: Units,
	/// Workers only (doesn't include MULEs).
	pub workers: Units,
	/// The gas buildings on geysers used to gather gas.
	pub gas_buildings: Units,
	/// Most of zerg units are morphed from it (Populated for zergs only).
	pub larvas: Units,
	/// Kind of things that appear when you order worker to build something but construction didn't started yet.
	pub placeholders: Units,
}
impl PlayerUnits {
	pub(crate) fn clear(&mut self) {
		self.all.clear();
		self.units.clear();
		self.structures.clear();
		self.townhalls.clear();
		self.workers.clear();
		self.gas_buildings.clear();
		self.larvas.clear();
		self.placeholders.clear();
	}
}

/// Convinient [`Unit`]s collection.
///
// [`Unit`]: crate::unit::Unit
#[derive(Default, Clone)]
pub struct Units(FxIndexMap<u64, Unit>);
impl Units {
	// HashMap methods

	/// Constrructs new empty units collection.
	#[inline]
	pub fn new() -> Self {
		Self(Default::default())
	}

	/// Constructs new units collection with given capacity.
	#[inline]
	pub fn with_capacity(n: usize) -> Self {
		Self(IndexMap::with_capacity_and_hasher(
			n,
			BuildHasherDefault::<FxHasher>::default(),
		))
	}

	/// Returns current capacity of the collection.
	#[inline]
	pub fn capacity(&self) -> usize {
		self.0.capacity()
	}

	/// Reserves additional capacity in the collection.
	#[inline]
	pub fn reserve(&mut self, additional: usize) {
		self.0.reserve(additional);
	}

	/// Shrinks the capacity as much as possible.
	#[inline]
	pub fn shrink_to_fit(&mut self) {
		self.0.shrink_to_fit();
	}

	/// Returns first unit in the collection.
	#[inline]
	pub fn first(&self) -> Option<&Unit> {
		self.0.values().next()
	}

	/// Inserts unit in the collection.
	///
	/// If collection already contains unit with the same tag,
	/// replaces it and returns previous unit.
	#[inline]
	pub fn push(&mut self, u: Unit) -> Option<Unit> {
		self.0.insert(u.tag, u)
	}

	/// Removes and returns last unit from the collection.
	///
	/// Returns `None` if the collection is empty.
	#[inline]
	pub fn pop(&mut self) -> Option<Unit> {
		self.0.pop().map(|i| i.1)
	}

	/// Removes and returns unit with given tag.
	///
	/// Returns `None` if there's no unit with such tag in the collection.
	#[inline]
	pub fn remove(&mut self, u: u64) -> Option<Unit> {
		self.0.remove(&u)
	}

	/// Returns an iterator over the units of the collection.
	#[inline]
	pub fn iter(&self) -> Values<u64, Unit> {
		self.0.values()
	}

	/// Returns mutable iterator over the units of the collection.
	#[inline]
	pub fn iter_mut(&mut self) -> ValuesMut<u64, Unit> {
		self.0.values_mut()
	}

	/// Returns an iterator over (tag, unit) pairs of the collection.
	#[inline]
	pub fn pairs(&self) -> Iter<u64, Unit> {
		self.0.iter()
	}

	/// Returns mutable iterator over (tag, unit) pairs of the collection.
	#[inline]
	pub fn pairs_mut(&mut self) -> IterMut<u64, Unit> {
		self.0.iter_mut()
	}

	/// Returns an iterator over unit tags of the collection.
	#[inline]
	pub fn tags(&self) -> Keys<u64, Unit> {
		self.0.keys()
	}

	/// Returns `true` if collection contains no units.
	#[inline]
	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}

	/// Returns the current number of units in the collection.
	#[inline]
	pub fn len(&self) -> usize {
		self.0.len()
	}

	/// Removes all units from the collection, while preserving its capacity.
	#[inline]
	pub fn clear(&mut self) {
		self.0.clear()
	}

	/// Checks if the collection contains unit with given tag.
	#[inline]
	pub fn contains_tag(&self, tag: u64) -> bool {
		self.0.contains_key(&tag)
	}

	/// Returns a reference to unit with given tag or `None` if there's no unit with such tag.
	#[inline]
	pub fn get(&self, tag: u64) -> Option<&Unit> {
		self.0.get(&tag)
	}

	/// Returns a mutable reference to unit with given tag or `None` if there's no unit with such tag.
	#[inline]
	pub fn get_mut(&mut self, tag: u64) -> Option<&mut Unit> {
		self.0.get_mut(&tag)
	}

	// Units methods

	/// Searches for units with given tags and makes new collection of them.
	///
	/// Warning: This method will clone units in order to create a new collection
	/// and will be evaluated initially. When applicable prefer using [`find_tags`]
	/// on the iterator over units, since it's lazily evaluated and doesn't do any cloning operations.
	///
	/// [`find_tags`]: UnitsIterator::find_tags
	pub fn find_tags<'a, T: IntoIterator<Item = &'a u64>>(&self, tags: T) -> Self {
		tags.into_iter()
			.filter_map(|tag| self.0.get(tag).cloned())
			.collect()
	}
	/// Leaves only units of given type and makes a new collection of them.
	///
	/// Warning: This method will clone units in order to create a new collection
	/// and will be evaluated initially. When applicable prefer using [`of_type`]
	/// on the iterator over units, since it's lazily evaluated and doesn't do any cloning operations.
	///
	/// [`of_type`]: UnitsIterator::of_type
	pub fn of_type(&self, unit_type: UnitTypeId) -> Self {
		self.filter(|u| u.type_id == unit_type)
	}
	/// Excludes all units of given type and makes a new collection of remaining units.
	///
	/// Warning: This method will clone units in order to create a new collection
	/// and will be evaluated initially. When applicable prefer using [`exclude_type`]
	/// on the iterator over units, since it's lazily evaluated and doesn't do any cloning operations.
	///
	/// [`exclude_type`]: UnitsIterator::exclude_type
	pub fn exclude_type(&self, unit_type: UnitTypeId) -> Self{
		self.filter(|u| u.type_id != unit_type)
	}
	/// Returns central position of all units in the collection or `None` if collection is empty.
	pub fn center(&self) -> Option<Point2> {
		if self.is_empty() {
			None
		} else {
			Some(self.sum(|u| u.position) / self.len() as f32)
		}
	}
	/// Leaves only non-flying units and makes new collection of them.
	///
	/// Warning: This method will clone units in order to create a new collection
	/// and will be evaluated initially. When applicable prefer using [`ground`]
	/// on the iterator over units, since it's lazily evaluated and doesn't do any cloning operations.
	///
	/// [`ground`]: UnitsIterator::ground
	pub fn ground(&self) -> Self {
		self.filter(|u| !u.is_flying)
	}
	/// Leaves only flying units and makes new collection of them.
	///
	/// Warning: This method will clone units in order to create a new collection
	/// and will be evaluated initially. When applicable prefer using [`flying`]
	/// on the iterator over units, since it's lazily evaluated and doesn't do any cloning operations.
	///
	/// [`flying`]: UnitsIterator::flying
	pub fn flying(&self) -> Self {
		self.filter(|u| u.is_flying)
	}
	/// Leaves only ready structures and makes new collection of them.
	///
	/// Warning: This method will clone units in order to create a new collection
	/// and will be evaluated initially. When applicable prefer using [`ready`]
	/// on the iterator over units, since it's lazily evaluated and doesn't do any cloning operations.
	///
	/// [`ready`]: UnitsIterator::ready
	pub fn ready(&self) -> Self {
		self.filter(|u| u.is_ready())
	}
	/// Leaves only structures in-progress and makes new collection of them.
	///
	/// Warning: This method will clone units in order to create a new collection
	/// and will be evaluated initially. When applicable prefer using [`not_ready`]
	/// on the iterator over units, since it's lazily evaluated and doesn't do any cloning operations.
	///
	/// [`not_ready`]: UnitsIterator::not_ready
	pub fn not_ready(&self) -> Self {
		self.filter(|u| !u.is_ready())
	}
	/// Leaves only units with no orders and makes new collection of them.
	///
	/// Warning: This method will clone units in order to create a new collection
	/// and will be evaluated initially. When applicable prefer using [`idle`]
	/// on the iterator over units, since it's lazily evaluated and doesn't do any cloning operations.
	///
	/// [`idle`]: UnitsIterator::idle
	pub fn idle(&self) -> Self {
		self.filter(|u| u.is_idle())
	}
	/// Leaves only units with no orders or that almost finished their orders and makes new collection of them.
	///
	/// Warning: This method will clone units in order to create a new collection
	/// and will be evaluated initially. When applicable prefer using [`almost_idle`]
	/// on the iterator over units, since it's lazily evaluated and doesn't do any cloning operations.
	///
	/// [`almost_idle`]: UnitsIterator::almost_idle
	pub fn almost_idle(&self) -> Self {
		self.filter(|u| u.is_almost_idle())
	}
	/// Leaves only units with no orders and makes new collection of them.
	/// Unlike [`idle`] this takes reactor on terran buildings into account.
	///
	/// Warning: This method will clone units in order to create a new collection
	/// and will be evaluated initially. When applicable prefer using [`unused`]
	/// on the iterator over units, since it's lazily evaluated and doesn't do any cloning operations.
	///
	/// [`idle`]: Self::idle
	/// [`unused`]: UnitsIterator::unused
	pub fn unused(&self) -> Self {
		self.filter(|u| u.is_unused())
	}
	/// Leaves only units with no orders or that almost finished their orders and makes new collection of them.
	/// Unlike [`almost_idle`] this takes reactor on terran buildings into account.
	///
	/// Warning: This method will clone units in order to create a new collection
	/// and will be evaluated initially. When applicable prefer using [`almost_unused`]
	/// on the iterator over units, since it's lazily evaluated and doesn't do any cloning operations.
	///
	/// [`almost_idle`]: Self::almost_idle
	/// [`almost_unused`]: UnitsIterator::almost_unused
	pub fn almost_unused(&self) -> Self {
		self.filter(|u| u.is_almost_unused())
	}
	/// Leaves only units in attack range of given unit and makes new collection of them.
	///
	/// Warning: This method will clone units in order to create a new collection
	/// and will be evaluated initially. When applicable prefer using [`in_range_of`]
	/// on the iterator over units, since it's lazily evaluated and doesn't do any cloning operations.
	///
	/// [`in_range_of`]: UnitsIterator::in_range_of
	pub fn in_range_of(&self, unit: &Unit, gap: f32) -> Self {
		self.filter(|u| unit.in_range(u, gap))
	}
	/// Leaves only units that are close enough to attack given unit and makes new collection of them.
	///
	/// Warning: This method will clone units in order to create a new collection
	/// and will be evaluated initially. When applicable prefer using [`in_range`]
	/// on the iterator over units, since it's lazily evaluated and doesn't do any cloning operations.
	///
	/// [`in_range`]: UnitsIterator::in_range
	pub fn in_range(&self, unit: &Unit, gap: f32) -> Self {
		self.filter(|u| u.in_range(unit, gap))
	}
	/// Leaves only units in attack range of given unit and makes new collection of them.
	/// Unlike [`in_range_of`] this takes range upgrades into account.
	///
	/// Warning: This method will clone units in order to create a new collection
	/// and will be evaluated initially. When applicable prefer using [`in_real_range_of`]
	/// on the iterator over units, since it's lazily evaluated and doesn't do any cloning operations.
	///
	/// [`in_range_of`]: Self::in_range_of
	/// [`in_real_range_of`]: UnitsIterator::in_real_range_of
	pub fn in_real_range_of(&self, unit: &Unit, gap: f32) -> Self {
		self.filter(|u| unit.in_real_range(u, gap))
	}
	/// Leaves only units that are close enough to attack given unit and makes new collection of them.
	/// Unlike [`in_range`] this takes range upgrades into account.
	///
	/// Warning: This method will clone units in order to create a new collection
	/// and will be evaluated initially. When applicable prefer using [`in_real_range`]
	/// on the iterator over units, since it's lazily evaluated and doesn't do any cloning operations.
	///
	/// [`in_range`]: Self::in_range
	/// [`in_real_range`]: UnitsIterator::in_real_range
	pub fn in_real_range(&self, unit: &Unit, gap: f32) -> Self {
		self.filter(|u| u.in_real_range(unit, gap))
	}
	/// Leaves only units visible on current step and makes new collection of them.
	///
	/// Warning: This method will clone units in order to create a new collection
	/// and will be evaluated initially. When applicable prefer using [`visible`]
	/// on the iterator over units, since it's lazily evaluated and doesn't do any cloning operations.
	///
	/// [`visible`]: UnitsIterator::visible
	pub fn visible(&self) -> Self {
		self.filter(|u| u.is_visible())
	}

	/// Sorts the collection by given function.
	pub fn sort<T, F>(&mut self, f: F)
	where
		T: PartialOrd,
		F: Fn(&Unit) -> T,
	{
		self.0.sort_by(cmp_by2(f));
	}
	/// Makes new collection sorted by given function.
	/// Leaves original collection untouched.
	pub fn sorted<T, F>(&self, f: F) -> Self
	where
		T: PartialOrd,
		F: Fn(&Unit) -> T,
	{
		let mut sorted = self.clone();
		sorted.0.sort_by(cmp_by2(f));
		sorted
	}
}

impl FromIterator<Unit> for Units {
	#[inline]
	fn from_iter<I: IntoIterator<Item = Unit>>(iter: I) -> Self {
		Self(iter.into_iter().map(|u| (u.tag, u)).collect())
	}
}
impl FromIterator<(u64, Unit)> for Units {
	#[inline]
	fn from_iter<I: IntoIterator<Item = (u64, Unit)>>(iter: I) -> Self {
		Self(iter.into_iter().collect())
	}
}

impl IntoIterator for Units {
	type Item = (u64, Unit);
	type IntoIter = IntoIter<u64, Unit>;

	#[inline]
	fn into_iter(self) -> Self::IntoIter {
		self.0.into_iter()
	}
}
impl<'a> IntoIterator for &'a Units {
	type Item = (&'a u64, &'a Unit);
	type IntoIter = Iter<'a, u64, Unit>;

	#[inline]
	fn into_iter(self) -> Self::IntoIter {
		self.0.iter()
	}
}
impl<'a> IntoIterator for &'a mut Units {
	type Item = (&'a u64, &'a mut Unit);
	type IntoIter = IterMut<'a, u64, Unit>;

	#[inline]
	fn into_iter(self) -> Self::IntoIter {
		self.0.iter_mut()
	}
}

impl Extend<Unit> for Units {
	#[inline]
	fn extend<T: IntoIterator<Item = Unit>>(&mut self, iter: T) {
		self.0.extend(iter.into_iter().map(|u| (u.tag, u)));
	}
}
impl Extend<(u64, Unit)> for Units {
	#[inline]
	fn extend<T: IntoIterator<Item = (u64, Unit)>>(&mut self, iter: T) {
		self.0.extend(iter);
	}
}

impl Index<u64> for Units {
	type Output = Unit;

	#[inline]
	fn index(&self, tag: u64) -> &Self::Output {
		&self.0[&tag]
	}
}
impl IndexMut<u64> for Units {
	#[inline]
	fn index_mut(&mut self, tag: u64) -> &mut Self::Output {
		&mut self.0[&tag]
	}
}

impl Index<usize> for Units {
	type Output = Unit;

	#[inline]
	fn index(&self, i: usize) -> &Self::Output {
		&self.0[i]
	}
}
impl IndexMut<usize> for Units {
	#[inline]
	fn index_mut(&mut self, i: usize) -> &mut Self::Output {
		&mut self.0[i]
	}
}

use std::cmp::Ordering;

#[inline]
fn cmp<T: PartialOrd>(a: &T, b: &T) -> Ordering {
	a.partial_cmp(&b).unwrap()
}

#[cfg(not(feature = "rayon"))]
#[inline]
fn cmp_by<U, T, F>(f: F) -> impl Fn(&&U, &&U) -> Ordering
where
	T: PartialOrd,
	F: Fn(&U) -> T,
{
	move |a, b| f(a).partial_cmp(&f(b)).unwrap()
}

#[inline]
fn cmp_by2<K, V, T, F>(f: F) -> impl Fn(&K, &V, &K, &V) -> Ordering
where
	T: PartialOrd,
	F: Fn(&V) -> T,
{
	move |_, a, _, b| f(a).partial_cmp(&f(b)).unwrap()
}

#[cfg(not(feature = "rayon"))]
impl Units {
	/// Leaves only units that match given predicate and makes new collection of them.
	///
	/// Warning: This method will clone units in order to create a new collection
	/// and will be evaluated initially. When applicable prefer using [`filter`]
	/// on the iterator over units, since it's lazily evaluated and doesn't do any cloning operations.
	///
	/// [`filter`]: UnitsIterator::filter
	pub fn filter<F>(&self, f: F) -> Self
	where
		F: Fn(&&Unit) -> bool,
	{
		Self(self.iter().filter(f).map(|u| (u.tag, u.clone())).collect())
	}
	/// Leaves only units of given types and makes a new collection of them.
	///
	/// Warning: This method will clone units in order to create a new collection
	/// and will be evaluated initially. When applicable prefer using [`of_types`]
	/// on the iterator over units, since it's lazily evaluated and doesn't do any cloning operations.
	///
	/// [`of_types`]: UnitsIterator::of_types
	pub fn of_types<T: Container<UnitTypeId>>(&self, types: &T) -> Self {
		self.filter(|u| types.contains(&u.type_id))
	}

	/// Excludes units of given types and makes a new collection of remaining units.
	///
	/// Warning: This method will clone units in order to create a new collection
	/// and will be evaluated initially. When applicable prefer using [`exclude_types`]
	/// on the iterator over units, since it's lazily evaluated and doesn't do any cloning operations.
	///
	/// [`exclude_types`]: UnitsIterator::exclude_types
	pub fn exclude_types<T: Container<UnitTypeId>>(&self, types: &T) -> Self{
		self.filter(|u| !types.contains(&u.type_id))
	}

	/// Leaves only units closer than given distance to target and makes new collection of them.
	///
	/// Warning: This method will clone units in order to create a new collection
	/// and will be evaluated initially. When applicable prefer using [`closer`]
	/// on the iterator over units, since it's lazily evaluated and doesn't do any cloning operations.
	///
	/// [`closer`]: UnitsIterator::closer
	pub fn closer<P: Into<Point2> + Copy>(&self, distance: f32, target: P) -> Self {
		self.filter(|u| u.is_closer(distance, target))
	}
	/// Leaves only units further than given distance to target and makes new collection of them.
	///
	/// Warning: This method will clone units in order to create a new collection
	/// and will be evaluated initially. When applicable prefer using [`further`]
	/// on the iterator over units, since it's lazily evaluated and doesn't do any cloning operations.
	///
	/// [`further`]: UnitsIterator::further
	pub fn further<P: Into<Point2> + Copy>(&self, distance: f32, target: P) -> Self {
		self.filter(|u| u.is_further(distance, target))
	}

	/// Returns closest from the collection unit to given target.
	pub fn closest<P: Into<Point2> + Copy>(&self, target: P) -> Option<&Unit> {
		self.min(|u| u.distance_squared(target))
	}
	/// Returns furthest from the collection unit to given target.
	pub fn furthest<P: Into<Point2> + Copy>(&self, target: P) -> Option<&Unit> {
		self.max(|u| u.distance_squared(target))
	}

	/// Returns distance from closest unit in the collection to given target.
	pub fn closest_distance<P: Into<Point2> + Copy>(&self, target: P) -> Option<f32> {
		self.min_value(|u| u.distance_squared(target))
			.map(|dist| dist.sqrt())
	}
	/// Returns distance from furthest unit in the collection to given target.
	pub fn furthest_distance<P: Into<Point2> + Copy>(&self, target: P) -> Option<f32> {
		self.max_value(|u| u.distance_squared(target))
			.map(|dist| dist.sqrt())
	}

	/// Returns squared distance from closest unit in the collection to given target.
	pub fn closest_distance_squared<P: Into<Point2> + Copy>(&self, target: P) -> Option<f32> {
		self.min_value(|u| u.distance_squared(target))
	}
	/// Returns squared distance from furthest unit in the collection to given target.
	pub fn furthest_distance_squared<P: Into<Point2> + Copy>(&self, target: P) -> Option<f32> {
		self.max_value(|u| u.distance_squared(target))
	}

	/// Returns sum of given unit values.
	pub fn sum<T, F>(&self, f: F) -> T
	where
		T: Sum,
		F: Fn(&Unit) -> T,
	{
		self.iter().map(f).sum::<T>()
	}

	/// Returns unit with minimum given predicate.
	pub fn min<T, F>(&self, f: F) -> Option<&Unit>
	where
		T: PartialOrd,
		F: Fn(&Unit) -> T,
	{
		self.iter().min_by(cmp_by(f))
	}
	/// Returns minimum of given unit values.
	pub fn min_value<T, F>(&self, f: F) -> Option<T>
	where
		T: PartialOrd,
		F: Fn(&Unit) -> T,
	{
		self.iter().map(f).min_by(cmp)
	}

	/// Returns unit with maximum given predicate.
	pub fn max<T, F>(&self, f: F) -> Option<&Unit>
	where
		T: PartialOrd,
		F: Fn(&Unit) -> T,
	{
		self.iter().max_by(cmp_by(f))
	}
	/// Returns maximum of given unit values.
	pub fn max_value<T, F>(&self, f: F) -> Option<T>
	where
		T: PartialOrd,
		F: Fn(&Unit) -> T,
	{
		self.iter().map(f).max_by(cmp)
	}
}

#[cfg(feature = "rayon")]
use indexmap::map::rayon::{IntoParIter, ParIter, ParIterMut, ParKeys, ParValues, ParValuesMut};
#[cfg(feature = "rayon")]
use rayon::prelude::*;

#[cfg(feature = "rayon")]
#[inline]
fn cmp_by<U, T, F>(f: F) -> impl Fn(&&U, &&U) -> Ordering
where
	T: PartialOrd,
	F: Fn(&U) -> T + Send + Sync,
{
	move |a, b| f(a).partial_cmp(&f(b)).unwrap()
}

#[cfg(feature = "rayon")]
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
		Self(self.par_iter().filter(f).map(|u| (u.tag, u.clone())).collect())
	}

	/// Leaves only units of given types and makes a new collection of them.
	///
	/// Warning: This method will clone units in order to create a new collection
	/// and will be evaluated initially. When applicable prefer using [`of_types`]
	/// on the iterator over units, since it's lazily evaluated and doesn't do any cloning operations.
	///
	/// [`of_types`]: UnitsIterator::of_types
	pub fn of_types<T: Container<UnitTypeId> + Sync>(&self, types: &T) -> Self {
		self.filter(|u| types.contains(&u.type_id))
	}

	/// Excludes units of given types and makes a new collection of remaining units.
	///
	/// Warning: This method will clone units in order to create a new collection
	/// and will be evaluated initially. When applicable prefer using [`exclude_types`]
	/// on the iterator over units, since it's lazily evaluated and doesn't do any cloning operations.
	///
	/// [`exclude_types`]: UnitsIterator::exclude_types
	pub fn exclude_types<T: Container<UnitTypeId>>(&self, types: &T) -> Self{
		self.filter(|U| !types.contains(&u.type_id))
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

#[cfg(feature = "rayon")]
impl IntoParallelIterator for Units {
	type Item = (u64, Unit);
	type Iter = IntoParIter<u64, Unit>;

	#[inline]
	fn into_par_iter(self) -> Self::Iter {
		self.0.into_par_iter()
	}
}
#[cfg(feature = "rayon")]
impl<'a> IntoParallelIterator for &'a Units {
	type Item = (&'a u64, &'a Unit);
	type Iter = ParIter<'a, u64, Unit>;

	#[inline]
	fn into_par_iter(self) -> Self::Iter {
		self.0.par_iter()
	}
}
#[cfg(feature = "rayon")]
impl<'a> IntoParallelIterator for &'a mut Units {
	type Item = (&'a u64, &'a mut Unit);
	type Iter = ParIterMut<'a, u64, Unit>;

	#[inline]
	fn into_par_iter(self) -> Self::Iter {
		self.0.par_iter_mut()
	}
}

#[cfg(feature = "rayon")]
impl ParallelExtend<Unit> for Units {
	#[inline]
	fn par_extend<T: IntoParallelIterator<Item = Unit>>(&mut self, par_iter: T) {
		self.0.par_extend(par_iter.into_par_iter().map(|u| (u.tag, u)));
	}
}
#[cfg(feature = "rayon")]
impl ParallelExtend<(u64, Unit)> for Units {
	#[inline]
	fn par_extend<T: IntoParallelIterator<Item = (u64, Unit)>>(&mut self, par_iter: T) {
		self.0.par_extend(par_iter);
	}
}

#[cfg(feature = "rayon")]
impl FromParallelIterator<Unit> for Units {
	#[inline]
	fn from_par_iter<I: IntoParallelIterator<Item = Unit>>(par_iter: I) -> Self {
		Self(par_iter.into_par_iter().map(|u| (u.tag, u)).collect())
	}
}
#[cfg(feature = "rayon")]
impl FromParallelIterator<(u64, Unit)> for Units {
	#[inline]
	fn from_par_iter<I: IntoParallelIterator<Item = (u64, Unit)>>(par_iter: I) -> Self {
		Self(par_iter.into_par_iter().collect())
	}
}

/// Joins collections functionality to check if given item is present in it.
/// Used in generics of some units methods.
pub trait Container<T> {
	/// Returns `true` if item is present in the collection.
	fn contains(&self, item: &T) -> bool;
}

use std::{
	collections::{BTreeMap, BTreeSet, HashMap, HashSet},
	hash::{BuildHasher, Hash},
};

impl<T: PartialEq> Container<T> for [T] {
	fn contains(&self, other: &T) -> bool {
		self.iter().any(|item| item == other)
	}
}
impl<T: PartialEq> Container<T> for Vec<T> {
	fn contains(&self, other: &T) -> bool {
		self.iter().any(|item| item == other)
	}
}
impl<T: Eq + Hash, S: BuildHasher> Container<T> for HashSet<T, S> {
	fn contains(&self, item: &T) -> bool {
		self.contains(item)
	}
}
impl<T: Eq + Hash, V, S: BuildHasher> Container<T> for HashMap<T, V, S> {
	fn contains(&self, item: &T) -> bool {
		self.contains_key(item)
	}
}
impl<T: Ord> Container<T> for BTreeSet<T> {
	fn contains(&self, item: &T) -> bool {
		self.contains(item)
	}
}
impl<T: Ord, V> Container<T> for BTreeMap<T, V> {
	fn contains(&self, item: &T) -> bool {
		self.contains_key(item)
	}
}
impl<T: Eq + Hash> Container<T> for IndexSet<T> {
	fn contains(&self, item: &T) -> bool {
		self.contains(item)
	}
}
impl<T: Eq + Hash, V> Container<T> for IndexMap<T, V> {
	fn contains(&self, item: &T) -> bool {
		self.contains_key(item)
	}
}

use std::iter::Filter;

/// Helper trait for iterators over units.
pub trait UnitsIterator<'a>: Iterator<Item = &'a Unit> + Sized {
	/// Searches for unit with given tag and returns it if found.
	fn find_tag(mut self, tag: u64) -> Option<&'a Unit> {
		self.find(|u| u.tag == tag)
	}
	/// Leaves only units with given tags.
	fn find_tags<T>(self, tags: &'a T) -> Filter<Self, Box<dyn FnMut(&&Unit) -> bool + 'a>>
	where
		T: Container<u64>,
	{
		self.filter(Box::new(move |u| tags.contains(&u.tag)))
	}
	/// Leaves only units of given type.
	fn of_type(self, unit_type: UnitTypeId) -> Filter<Self, Box<dyn FnMut(&&Unit) -> bool + 'a>> {
		self.filter(Box::new(move |u| u.type_id == unit_type))
	}
	fn exclude_type(self, unit_type: UnitTypeId) -> Filter<Self, Box<dyn FnMut(&&Unit)->bool + 'a>>{
		self.filter(Box::new(move |u| u.type_id == unit_type))
	/// Excludes units of given type.
	}
	/// Leaves only units of given types.
	fn of_types<T>(self, types: &'a T) -> Filter<Self, Box<dyn FnMut(&&Unit) -> bool + 'a>>
	where
		T: Container<UnitTypeId>,
	{
		self.filter(Box::new(move |u| types.contains(&u.type_id)))
	}
	/// Excludes units of given types.
	fn exclude_types<T>(self, types: &'a T) -> Filter<Self, Box<dyn FnMut(&&Unit) -> bool + 'a>>
	where
		T: Container<UnitTypeId>,
	{
		self.filter(Box::new(move |u| !types.contains(&u.type_id)))
	}
	/// Leaves only non-flying units.
	fn ground(self) -> Filter<Self, Box<dyn FnMut(&&Unit) -> bool + 'a>> {
		self.filter(Box::new(|u| !u.is_flying))
	}
	/// Leaves only flying units.
	fn flying(self) -> Filter<Self, Box<dyn FnMut(&&Unit) -> bool + 'a>> {
		self.filter(Box::new(|u| u.is_flying))
	}
	/// Leaves only ready structures.
	fn ready(self) -> Filter<Self, Box<dyn FnMut(&&Unit) -> bool + 'a>> {
		self.filter(Box::new(|u| u.is_ready()))
	}
	/// Leaves only structures in-progress.
	fn not_ready(self) -> Filter<Self, Box<dyn FnMut(&&Unit) -> bool + 'a>> {
		self.filter(Box::new(|u| !u.is_ready()))
	}
	/// Leaves only units with no orders.
	fn idle(self) -> Filter<Self, Box<dyn FnMut(&&Unit) -> bool + 'a>> {
		self.filter(Box::new(|u| u.is_idle()))
	}
	/// Leaves only units with no orders or that almost finished their orders.
	fn almost_idle(self) -> Filter<Self, Box<dyn FnMut(&&Unit) -> bool + 'a>> {
		self.filter(Box::new(|u| u.is_almost_idle()))
	}
	/// Leaves only units with no orders.
	/// Unlike [`idle`](Self::idle) this takes reactor on terran buildings into account.
	fn unused(self) -> Filter<Self, Box<dyn FnMut(&&Unit) -> bool + 'a>> {
		self.filter(Box::new(|u| u.is_unused()))
	}
	/// Leaves only units with no orders or that almost finished their orders.
	/// Unlike [`almost_idle`](Self::almost_idle) this takes reactor on terran buildings into account.
	fn almost_unused(self) -> Filter<Self, Box<dyn FnMut(&&Unit) -> bool + 'a>> {
		self.filter(Box::new(|u| u.is_almost_unused()))
	}
	/// Leaves only units in attack range of given unit.
	fn in_range_of(self, unit: &'a Unit, gap: f32) -> Filter<Self, Box<dyn FnMut(&&Unit) -> bool + 'a>> {
		self.filter(Box::new(move |u| unit.in_range(u, gap)))
	}
	/// Leaves only units that are close enough to attack given unit.
	fn in_range(self, unit: &'a Unit, gap: f32) -> Filter<Self, Box<dyn FnMut(&&Unit) -> bool + 'a>> {
		self.filter(Box::new(move |u| u.in_range(unit, gap)))
	}
	/// Leaves only units in attack range of given unit.
	/// Unlike [`in_range_of`](Self::in_range_of) this takes range upgrades into account.
	fn in_real_range_of(self, unit: &'a Unit, gap: f32) -> Filter<Self, Box<dyn FnMut(&&Unit) -> bool + 'a>> {
		self.filter(Box::new(move |u| unit.in_real_range(u, gap)))
	}
	/// Leaves only units that are close enough to attack given unit.
	/// Unlike [`in_range`](Self::in_range) this takes range upgrades into account.
	fn in_real_range(self, unit: &'a Unit, gap: f32) -> Filter<Self, Box<dyn FnMut(&&Unit) -> bool + 'a>> {
		self.filter(Box::new(move |u| u.in_real_range(unit, gap)))
	}
	/// Leaves only units visible on current step.
	fn visible(self) -> Filter<Self, Box<dyn FnMut(&&Unit) -> bool + 'a>> {
		self.filter(Box::new(|u| u.is_visible()))
	}
}
#[cfg(feature = "rayon")]
use rayon::iter::Filter as ParFilter;

/// Helper trait for parallel iterators over units.
#[cfg(feature = "rayon")]
pub trait ParUnitsIterator<'a>: ParallelIterator<Item = &'a Unit> {
	/// Searches for unit with given tag and returns it if found.
	fn find_tag(self, tag: u64) -> Option<&'a Unit> {
		self.find_any(|u| u.tag == tag)
	}
	/// Leaves only units with given tags.
	fn find_tags<T>(self, tags: &'a T) -> ParFilter<Self, Box<dyn Fn(&&Unit) -> bool + Send + Sync + 'a>>
	where
		T: Container<u64> + Sync,
	{
		self.filter(Box::new(move |u| tags.contains(&u.tag)))
	}
	/// Leaves only units of given type.
	fn of_type(self, type_id: UnitTypeId) -> ParFilter<Self, Box<dyn Fn(&&Unit) -> bool + Send + Sync + 'a>> {
		self.filter(Box::new(move |u| u.type_id == type_id))
	}
	fn exclude_type(self, type_id: UnitTypeId) -> ParFilter<Self, Box<dyn Fn(&&Unit) -> bool + Send + Sync + 'a>> {
	/// Excludes units of given type.
		self.filter(Box::new(move |u| u.type_id != type_id))
	}
	/// Leaves only units of given types.
	fn of_types<T>(self, types: &'a T) -> ParFilter<Self, Box<dyn Fn(&&Unit) -> bool + Send + Sync + 'a>>
	where
		T: Container<UnitTypeId> + Sync,
	{
		self.filter(Box::new(move |u| types.contains(&u.type_id)))
	}
	/// Excludes units of given types.
	fn exclude_types<T>(self, types: &'a T) -> ParFilter<Self, Box<dyn Fn(&&Unit) -> bool + Send + Sync + 'a>>
	where
		T: Container<UnitTypeId> + Sync,
	{
		self.filter(Box::new(move |u| !types.contains(&u.type_id)))
	}
	/// Leaves only non-flying units.
	fn ground(self) -> ParFilter<Self, Box<dyn Fn(&&Unit) -> bool + Send + Sync + 'a>> {
		self.filter(Box::new(|u| !u.is_flying))
	}
	/// Leaves only flying units.
	fn flying(self) -> ParFilter<Self, Box<dyn Fn(&&Unit) -> bool + Send + Sync + 'a>> {
		self.filter(Box::new(|u| u.is_flying))
	}
	/// Leaves only ready structures.
	fn ready(self) -> ParFilter<Self, Box<dyn Fn(&&Unit) -> bool + Send + Sync + 'a>> {
		self.filter(Box::new(|u| u.is_ready()))
	}
	/// Leaves only structures in-progress.
	fn not_ready(self) -> ParFilter<Self, Box<dyn Fn(&&Unit) -> bool + Send + Sync + 'a>> {
		self.filter(Box::new(|u| !u.is_ready()))
	}
	/// Leaves only units with no orders.
	fn idle(self) -> ParFilter<Self, Box<dyn Fn(&&Unit) -> bool + Send + Sync + 'a>> {
		self.filter(Box::new(|u| u.is_idle()))
	}
	/// Leaves only units with no orders or that almost finished their orders.
	fn almost_idle(self) -> ParFilter<Self, Box<dyn Fn(&&Unit) -> bool + Send + Sync + 'a>> {
		self.filter(Box::new(|u| u.is_almost_idle()))
	}
	/// Leaves only units with no orders.
	/// Unlike [`idle`](Self::idle) this takes reactor on terran buildings into account.
	fn unused(self) -> ParFilter<Self, Box<dyn Fn(&&Unit) -> bool + Send + Sync + 'a>> {
		self.filter(Box::new(|u| u.is_unused()))
	}
	/// Leaves only units with no orders or that almost finished their orders.
	/// Unlike [`almost_idle`](Self::almost_idle) this takes reactor on terran buildings into account.
	fn almost_unused(self) -> ParFilter<Self, Box<dyn Fn(&&Unit) -> bool + Send + Sync + 'a>> {
		self.filter(Box::new(|u| u.is_almost_unused()))
	}
	/// Leaves only units in attack range of given unit.
	fn in_range_of(
		self,
		unit: &'a Unit,
		gap: f32,
	) -> ParFilter<Self, Box<dyn Fn(&&Unit) -> bool + Send + Sync + 'a>> {
		self.filter(Box::new(move |u| unit.in_range(u, gap)))
	}
	/// Leaves only units that are close enough to attack given unit.
	fn in_range(
		self,
		unit: &'a Unit,
		gap: f32,
	) -> ParFilter<Self, Box<dyn Fn(&&Unit) -> bool + Send + Sync + 'a>> {
		self.filter(Box::new(move |u| u.in_range(unit, gap)))
	}
	/// Leaves only units in attack range of given unit.
	/// Unlike [`in_range_of`](Self::in_range_of) this takes range upgrades into account.
	fn in_real_range_of(
		self,
		unit: &'a Unit,
		gap: f32,
	) -> ParFilter<Self, Box<dyn Fn(&&Unit) -> bool + Send + Sync + 'a>> {
		self.filter(Box::new(move |u| unit.in_real_range(u, gap)))
	}
	/// Leaves only units that are close enough to attack given unit.
	/// Unlike [`in_range`](Self::in_range) this takes range upgrades into account.
	fn in_real_range(
		self,
		unit: &'a Unit,
		gap: f32,
	) -> ParFilter<Self, Box<dyn Fn(&&Unit) -> bool + Send + Sync + 'a>> {
		self.filter(Box::new(move |u| u.in_real_range(unit, gap)))
	}
	/// Leaves only units visible on current step.
	fn visible(self) -> ParFilter<Self, Box<dyn Fn(&&Unit) -> bool + Send + Sync + 'a>> {
		self.filter(Box::new(|u| u.is_visible()))
	}
}

impl<'a, I> UnitsIterator<'a> for I where I: Iterator<Item = &'a Unit> + Sized {}

#[cfg(feature = "rayon")]
impl<'a, I> ParUnitsIterator<'a> for I where I: ParallelIterator<Item = &'a Unit> {}
