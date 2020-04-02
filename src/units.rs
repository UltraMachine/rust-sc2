use crate::{geometry::Point2, ids::UnitTypeId, unit::Unit};
use itertools::Itertools;
use std::{
	collections::{
		hash_map::{IntoIter, Iter, IterMut, Values, ValuesMut},
		HashMap,
	},
	iter::FromIterator,
	ops::Index,
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
	pub fn is_empty(&self) -> bool {
		self.units.is_empty()
	}

	#[inline]
	pub fn len(&self) -> usize {
		self.units.len()
	}

	// Units methods
	pub fn find_tag(&self, tag: u64) -> Option<Unit> {
		self.units.get(&tag).cloned()
	}
	pub fn find_tags<T: Iterator<Item = u64>>(&self, tags: T) -> Self {
		tags.filter_map(|tag| self.units.get(&tag).cloned()).collect()
	}
	pub fn of_type(&self, u_type: UnitTypeId) -> Self {
		self.filter(|u| u.type_id == u_type)
	}
	pub fn of_types<T: Iterator<Item = UnitTypeId>>(&self, mut types: T) -> Self {
		self.filter(|u| types.any(|u_type| u.type_id == u_type))
	}
	pub fn center(&self) -> Point2 {
		self.iter().map(|u| u.position).sum::<Point2>() / (self.len() as f32)
	}
	// Get closest | furthest
	pub fn closest(&self, other: &Unit) -> Unit {
		self.partial_min(|u| u.distance_squared(other))
	}
	pub fn closest_pos(&self, other: Point2) -> Unit {
		self.partial_min(|u| u.distance_pos_squared(other))
	}
	pub fn furthest(&self, other: &Unit) -> Unit {
		self.partial_max(|u| u.distance_squared(other))
	}
	pub fn furthest_pos(&self, other: Point2) -> Unit {
		self.partial_max(|u| u.distance_pos_squared(other))
	}
	// Get closest | furthest distance
	pub fn closest_distance(&self, other: &Unit) -> f32 {
		self.partial_min_value(|u| u.distance_squared(other)).sqrt()
	}
	pub fn closest_distance_pos(&self, other: Point2) -> f32 {
		self.partial_min_value(|u| u.distance_pos_squared(other)).sqrt()
	}
	pub fn furthest_distance(&self, other: &Unit) -> f32 {
		self.partial_max_value(|u| u.distance_squared(other)).sqrt()
	}
	pub fn furthest_distance_pos(&self, other: Point2) -> f32 {
		self.partial_max_value(|u| u.distance_pos_squared(other)).sqrt()
	}
	// Squared
	pub fn closest_distance_squared(&self, other: &Unit) -> f32 {
		self.partial_min_value(|u| u.distance_squared(other))
	}
	pub fn closest_distance_pos_squared(&self, other: Point2) -> f32 {
		self.partial_min_value(|u| u.distance_pos_squared(other))
	}
	pub fn furthest_distance_squared(&self, other: &Unit) -> f32 {
		self.partial_max_value(|u| u.distance_squared(other))
	}
	pub fn furthest_distance_pos_squared(&self, other: Point2) -> f32 {
		self.partial_max_value(|u| u.distance_pos_squared(other))
	}
	// Filter closer | further than distance
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
	pub fn visible(&self) -> Self {
		self.filter(|u| u.is_visible())
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
	pub fn min_value<B, F>(&self, mut f: F) -> B
	where
		B: Ord,
		F: for<'r> FnMut(&'r &Unit) -> B,
	{
		self.iter().map(|u| f(&u)).min().unwrap()
	}
	pub fn partial_min_value<B, F>(&self, mut f: F) -> B
	where
		B: PartialOrd,
		F: for<'r> FnMut(&'r &Unit) -> B,
	{
		self.iter()
			.map(|u| f(&u))
			.min_by(|b1, b2| b1.partial_cmp(&b2).unwrap())
			.unwrap()
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
	pub fn max_value<B, F>(&self, mut f: F) -> B
	where
		B: Ord,
		F: for<'r> FnMut(&'r &Unit) -> B,
	{
		self.iter().map(|u| f(&u)).max().unwrap()
	}
	pub fn partial_max_value<B, F>(&self, mut f: F) -> B
	where
		B: PartialOrd,
		F: for<'r> FnMut(&'r &Unit) -> B,
	{
		self.iter()
			.map(|u| f(&u))
			.max_by(|b1, b2| b1.partial_cmp(&b2).unwrap())
			.unwrap()
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
	type IntoIter = IntoIter<u64, Unit>;

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
