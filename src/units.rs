use crate::{geometry::Point2, unit::Unit};
use itertools::Itertools;
use std::{collections::HashMap, iter::FromIterator, ops::Index};

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
	pub mineral_field: Units,
	pub vespene_geyser: Units,
	pub resources: Units,
	pub destructables: Units,
	pub watchtowers: Units,
	pub inhibitor_zones: Units,
	pub gas_buildings: Units,
	pub larva: Units,
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
	pub fn push(&mut self, u: Unit) {
		self.units.insert(u.tag, u);
	}

	#[inline]
	pub fn iter(&self) -> std::collections::hash_map::Values<u64, Unit> {
		self.units.values()
	}

	#[inline]
	pub fn iter_mut(&mut self) -> std::collections::hash_map::ValuesMut<u64, Unit> {
		self.units.values_mut()
	}

	#[inline]
	pub fn is_empty(&self) -> bool {
		self.units.is_empty()
	}

	// Units methods
	pub fn find_tag(&self, tag: u64) -> Option<Unit> {
		self.units.get(&tag).cloned()
	}
	pub fn find_tags<T: Iterator<Item = u64>>(&self, tags: T) -> Self {
		tags.filter_map(|tag| self.units.get(&tag).cloned()).collect()
	}
	pub fn closest(&self, other: &Unit) -> Unit {
		self.iter()
			.min_by(|u1, u2| u1.distance(other).partial_cmp(&u2.distance(other)).unwrap())
			.unwrap()
			.clone()
	}
	pub fn closest_pos(&self, other: Point2) -> Unit {
		self.iter()
			.min_by(|u1, u2| {
				u1.distance_pos(other)
					.partial_cmp(&u2.distance_pos(other))
					.unwrap()
			})
			.unwrap()
			.clone()
	}
	pub fn closer_pos(&self, distance: f32, pos: Point2) -> Units {
		self.filter(|u| u.distance_pos_squared(pos) < distance * distance)
	}
	pub fn closer(&self, distance: f32, unit: &Unit) -> Units {
		self.filter(|u| u.distance_squared(unit) < distance * distance)
	}
	pub fn further_pos(&self, distance: f32, pos: Point2) -> Units {
		self.filter(|u| u.distance_pos_squared(pos) > distance * distance)
	}
	pub fn further(&self, distance: f32, unit: &Unit) -> Units {
		self.filter(|u| u.distance_squared(unit) > distance * distance)
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
		self.filter(|u| !u.is_flying.as_bool())
	}
	pub fn flying(&self) -> Self {
		self.filter(|u| u.is_flying.as_bool())
	}
	pub fn idle(&self) -> Self {
		self.filter(|u| u.is_idle())
	}
	pub fn almost_idle(&self) -> Self {
		self.filter(|u| u.is_almost_idle())
	}
	pub fn in_range_of(&self, unit: &Unit, gap: f32) -> Self {
		self.filter(|u| unit.in_range(u, gap))
	}
	pub fn in_range(&self, unit: &Unit, gap: f32) -> Self {
		self.filter(|u| u.in_range(unit, gap))
	}
	pub fn min<B, F>(&self, f: F) -> Unit
	where
		B: Ord,
		F: for<'r> FnMut(&'r &Unit) -> B,
	{
		self.iter().min_by_key(f).unwrap().clone()
	}
	pub fn partial_min<B, F>(&self, mut f: F) -> Unit
	where
		B: PartialOrd,
		F: for<'r> FnMut(&'r &Unit) -> B,
	{
		self.iter()
			.min_by(|u1, u2| f(u1).partial_cmp(&f(u2)).unwrap())
			.unwrap()
			.clone()
	}
	pub fn max<B, F>(&self, f: F) -> Unit
	where
		B: Ord,
		F: for<'r> FnMut(&'r &Unit) -> B,
	{
		self.iter().max_by_key(f).unwrap().clone()
	}
	pub fn partial_max<B, F>(&self, mut f: F) -> Unit
	where
		B: PartialOrd,
		F: for<'r> FnMut(&'r &Unit) -> B,
	{
		self.iter()
			.max_by(|u1, u2| f(u1).partial_cmp(&f(u2)).unwrap())
			.unwrap()
			.clone()
	}
	pub fn sort<B, F>(&self, f: F) -> Self
	where
		B: Ord,
		F: for<'r> FnMut(&'r &Unit) -> B,
	{
		self.iter().sorted_by_key(f).cloned().collect()
	}
	pub fn partial_sort<B, F>(&self, mut f: F) -> Self
	where
		B: PartialOrd,
		F: for<'r> FnMut(&'r &Unit) -> B,
	{
		self.iter()
			.sorted_by(|u1, u2| f(u1).partial_cmp(&f(u2)).unwrap())
			.cloned()
			.collect()
	}
}
impl FromIterator<Unit> for Units {
	fn from_iter<I: IntoIterator<Item = Unit>>(iter: I) -> Self {
		Units {
			units: iter.into_iter().map(|u| (u.tag, u)).collect(),
		}
	}
}
impl IntoIterator for Units {
	type Item = (u64, Unit);
	type IntoIter = std::collections::hash_map::IntoIter<u64, Unit>;

	fn into_iter(self) -> Self::IntoIter {
		self.units.into_iter()
	}
}
impl Index<usize> for Units {
	type Output = Unit;

	fn index(&self, i: usize) -> &Self::Output {
		&self.units.values().nth(i).expect("Units index out of bounds")
	}
}
