use crate::{geometry::Point2, ids::UnitTypeId, unit::Unit};
use itertools::Itertools;
use std::{
	collections::{
		hash_map::{IntoIter, Iter, IterMut, Keys, Values, ValuesMut},
		HashMap,
	},
	iter::{FromIterator, Sum},
	ops::{Index, IndexMut},
};

#[derive(Default, Clone)]
pub struct GroupedUnits {
	pub owned: Units,
	pub units: Units,
	pub structures: Units,
	pub townhalls: Units,
	pub workers: Units,
	pub enemies: Units,
	pub enemy_units: Units,
	pub enemy_structures: Units,
	pub enemy_townhalls: Units,
	pub enemy_workers: Units,
	pub mineral_fields: Units,
	pub vespene_geysers: Units,
	pub resources: Units,
	pub destructables: Units,
	pub watchtowers: Units,
	pub inhibitor_zones: Units,
	pub gas_buildings: Units,
	pub larvas: Units,
	pub placeholders: Units,
}

#[derive(Default, Clone)]
pub struct Units {
	units: HashMap<u64, Unit>,
}
impl Units {
	// HashMap methods
	#[inline]
	pub fn new() -> Self {
		Units {
			units: HashMap::new(),
		}
	}

	#[inline]
	pub fn first(&self) -> Option<&Unit> {
		self.units.values().next()
	}

	#[inline]
	pub fn push(&mut self, u: Unit) -> Option<Unit> {
		self.units.insert(u.tag, u)
	}

	#[inline]
	pub fn pop(&mut self) -> Option<Unit> {
		self.units
			.keys()
			.next()
			.copied()
			.and_then(|u| self.units.remove(&u))
	}

	#[inline]
	pub fn remove(&mut self, u: u64) -> Option<Unit> {
		self.units.remove(&u)
	}

	#[inline]
	pub fn iter(&self) -> Values<u64, Unit> {
		self.units.values()
	}

	#[inline]
	pub fn iter_mut(&mut self) -> ValuesMut<u64, Unit> {
		self.units.values_mut()
	}

	#[inline]
	pub fn pairs(&self) -> Iter<u64, Unit> {
		self.units.iter()
	}

	#[inline]
	pub fn pairs_mut(&mut self) -> IterMut<u64, Unit> {
		self.units.iter_mut()
	}

	#[inline]
	pub fn tags(&self) -> Keys<u64, Unit> {
		self.units.keys()
	}

	#[inline]
	pub fn is_empty(&self) -> bool {
		self.units.is_empty()
	}

	#[inline]
	pub fn len(&self) -> usize {
		self.units.len()
	}

	#[inline]
	pub fn clear(&mut self) {
		self.units.clear()
	}

	// Units methods
	pub fn find_tag(&self, tag: u64) -> Option<&Unit> {
		self.units.get(&tag)
	}
	pub fn find_tags<T: Iterator<Item = u64>>(&self, tags: T) -> Self {
		tags.filter_map(|tag| self.units.get(&tag).cloned()).collect()
	}
	pub fn of_type(&self, u_type: UnitTypeId) -> Self {
		self.filter(|u| u.type_id == u_type)
	}
	pub fn of_types<T: Iterator<Item = UnitTypeId> + Clone>(&self, types: T) -> Self {
		self.filter(|u| types.clone().any(|u_type| u.type_id == u_type))
	}
	pub fn center(&self) -> Point2 {
		self.sum(|u| u.position) / (self.len() as f32)
	}
	// Get closest | furthest
	pub fn closest<P: Into<Point2> + Copy>(&self, target: P) -> Option<&Unit> {
		self.partial_min(|u| u.distance_squared(target))
	}
	pub fn furthest<P: Into<Point2> + Copy>(&self, target: P) -> Option<&Unit> {
		self.partial_max(|u| u.distance_squared(target))
	}
	// Get closest | furthest distance
	pub fn closest_distance<P: Into<Point2> + Copy>(&self, target: P) -> Option<f32> {
		self.partial_min_value(|u| u.distance_squared(target))
			.map(|dist| dist.sqrt())
	}
	pub fn furthest_distance<P: Into<Point2> + Copy>(&self, target: P) -> Option<f32> {
		self.partial_max_value(|u| u.distance_squared(target))
			.map(|dist| dist.sqrt())
	}
	// Squared
	pub fn closest_distance_squared<P: Into<Point2> + Copy>(&self, target: P) -> Option<f32> {
		self.partial_min_value(|u| u.distance_squared(target))
	}
	pub fn furthest_distance_squared<P: Into<Point2> + Copy>(&self, target: P) -> Option<f32> {
		self.partial_max_value(|u| u.distance_squared(target))
	}
	// Filter closer | further than distance
	pub fn closer<P: Into<Point2> + Copy>(&self, distance: f32, target: P) -> Units {
		self.filter(|u| u.is_closer(distance, target))
	}
	pub fn further<P: Into<Point2> + Copy>(&self, distance: f32, target: P) -> Units {
		self.filter(|u| u.is_further(distance, target))
	}

	pub fn filter<F>(&self, f: F) -> Self
	where
		F: for<'r> FnMut(&'r Unit) -> bool,
	{
		Self {
			units: self.iter().cloned().filter(f).map(|u| (u.tag, u)).collect(),
		}
	}
	pub fn ground(&self) -> Self {
		self.filter(|u| !u.is_flying)
	}
	pub fn flying(&self) -> Self {
		self.filter(|u| u.is_flying)
	}
	pub fn ready(&self) -> Self {
		self.filter(|u| u.is_ready())
	}
	pub fn not_ready(&self) -> Self {
		self.filter(|u| !u.is_ready())
	}
	pub fn idle(&self) -> Self {
		self.filter(|u| u.is_idle())
	}
	pub fn almost_idle(&self) -> Self {
		self.filter(|u| u.is_almost_idle())
	}
	pub fn unused(&self) -> Self {
		self.filter(|u| u.is_unused())
	}
	pub fn almost_unused(&self) -> Self {
		self.filter(|u| u.is_almost_unused())
	}
	pub fn in_range_of(&self, unit: &Unit, gap: f32) -> Self {
		self.filter(|u| unit.in_range(u, gap))
	}
	pub fn in_range(&self, unit: &Unit, gap: f32) -> Self {
		self.filter(|u| u.in_range(unit, gap))
	}
	pub fn visible(&self) -> Self {
		self.filter(|u| u.is_visible())
	}
	pub fn sum<T, F>(&self, f: F) -> T
	where
		T: Sum,
		F: FnMut(&Unit) -> T,
	{
		self.iter().map(f).sum::<T>()
	}
	pub fn min<T, F>(&self, f: F) -> Option<&Unit>
	where
		T: Ord,
		F: for<'r> FnMut(&'r &Unit) -> T,
	{
		self.iter().min_by_key(f)
	}
	pub fn partial_min<T, F>(&self, mut f: F) -> Option<&Unit>
	where
		T: PartialOrd,
		F: for<'r> FnMut(&'r &Unit) -> T,
	{
		self.iter().min_by(|u1, u2| f(u1).partial_cmp(&f(u2)).unwrap())
	}
	pub fn min_value<T, F>(&self, f: F) -> Option<T>
	where
		T: Ord,
		F: FnMut(&Unit) -> T,
	{
		self.iter().map(f).min()
	}
	pub fn partial_min_value<T, F>(&self, f: F) -> Option<T>
	where
		T: PartialOrd,
		F: FnMut(&Unit) -> T,
	{
		self.iter().map(f).min_by(|a, b| a.partial_cmp(&b).unwrap())
	}
	pub fn max<T, F>(&self, f: F) -> Option<&Unit>
	where
		T: Ord,
		F: for<'r> FnMut(&'r &Unit) -> T,
	{
		self.iter().max_by_key(f)
	}
	pub fn partial_max<T, F>(&self, mut f: F) -> Option<&Unit>
	where
		T: PartialOrd,
		F: for<'r> FnMut(&'r &Unit) -> T,
	{
		self.iter().max_by(|u1, u2| f(u1).partial_cmp(&f(u2)).unwrap())
	}
	pub fn max_value<T, F>(&self, f: F) -> Option<T>
	where
		T: Ord,
		F: FnMut(&Unit) -> T,
	{
		self.iter().map(f).max()
	}
	pub fn partial_max_value<T, F>(&self, f: F) -> Option<T>
	where
		T: PartialOrd,
		F: FnMut(&Unit) -> T,
	{
		self.iter().map(f).max_by(|a, b| a.partial_cmp(&b).unwrap())
	}
	pub fn sort<T, F>(&self, f: F) -> Self
	where
		T: Ord,
		F: for<'r> FnMut(&'r &Unit) -> T,
	{
		self.iter().sorted_by_key(f).cloned().collect()
	}
	pub fn partial_sort<T, F>(&self, mut f: F) -> Self
	where
		T: PartialOrd,
		F: for<'r> FnMut(&'r &Unit) -> T,
	{
		self.iter()
			.sorted_by(|u1, u2| f(u1).partial_cmp(&f(u2)).unwrap())
			.cloned()
			.collect()
	}
}
impl FromIterator<Unit> for Units {
	#[inline]
	fn from_iter<I: IntoIterator<Item = Unit>>(iter: I) -> Self {
		Units {
			units: iter.into_iter().map(|u| (u.tag, u)).collect(),
		}
	}
}
impl IntoIterator for Units {
	type Item = (u64, Unit);
	type IntoIter = IntoIter<u64, Unit>;

	#[inline]
	fn into_iter(self) -> Self::IntoIter {
		self.units.into_iter()
	}
}
impl Index<u64> for Units {
	type Output = Unit;

	#[inline]
	fn index(&self, tag: u64) -> &Self::Output {
		&self.units[&tag]
	}
}
impl IndexMut<u64> for Units {
	#[inline]
	fn index_mut(&mut self, tag: u64) -> &mut Self::Output {
		self.units.get_mut(&tag).unwrap()
	}
}
impl Extend<Unit> for Units {
	#[inline]
	fn extend<T: IntoIterator<Item = Unit>>(&mut self, iter: T) {
		iter.into_iter().for_each(|u| {
			self.push(u);
		});
	}
}
