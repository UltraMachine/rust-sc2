//! [`Bot`] struct and it's helpers.
#![warn(missing_docs)]

use crate::{
	action::{Action, ActionResult, Commander, Target},
	api::API,
	client::SC2Result,
	constants::{RaceValues, INHIBITOR_IDS, RACE_VALUES, TECH_ALIAS, UNIT_ALIAS},
	debug::{DebugCommand, Debugger},
	distance::*,
	game_data::{Cost, GameData},
	game_info::GameInfo,
	game_state::{Alliance, GameState},
	geometry::Point2,
	ids::{AbilityId, UnitTypeId, UpgradeId},
	player::Race,
	ramp::{Ramp, Ramps},
	unit::{DataForUnit, SharedUnitData, Unit},
	units::AllUnits,
	utils::{dbscan, range_query},
	FromProto, IntoProto,
};
use num_traits::ToPrimitive;
use rand::prelude::{thread_rng, SliceRandom};
use rustc_hash::{FxHashMap, FxHashSet};
use sc2_proto::{
	query::{RequestQueryBuildingPlacement, RequestQueryPathing},
	sc2api::Request,
};
use std::{panic, process::Child};

#[cfg(feature = "rayon")]
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
#[cfg(not(feature = "rayon"))]
use std::{
	cell::{Ref, RefCell, RefMut},
	rc::Rc,
};

#[cfg(feature = "rayon")]
pub(crate) type Rs<T> = Arc<T>;
#[cfg(not(feature = "rayon"))]
pub(crate) type Rs<T> = Rc<T>;

#[cfg(feature = "rayon")]
pub(crate) type Rw<T> = Arc<RwLock<T>>;
#[cfg(not(feature = "rayon"))]
pub(crate) type Rw<T> = Rc<RefCell<T>>;

#[cfg(feature = "rayon")]
pub(crate) type Reader<'a, T> = RwLockReadGuard<'a, T>;
#[cfg(not(feature = "rayon"))]
pub(crate) type Reader<'a, T> = Ref<'a, T>;

#[cfg(feature = "rayon")]
pub(crate) type Writer<'a, T> = RwLockWriteGuard<'a, T>;
#[cfg(not(feature = "rayon"))]
pub(crate) type Writer<'a, T> = RefMut<'a, T>;

pub(crate) trait Locked<T> {
	fn lock_read(&self) -> Reader<T>;
	fn lock_write(&self) -> Writer<T>;
}
impl<T> Locked<T> for Rw<T> {
	fn lock_read(&self) -> Reader<T> {
		#[cfg(feature = "rayon")]
		{
			self.read().unwrap()
		}
		#[cfg(not(feature = "rayon"))]
		{
			self.borrow()
		}
	}
	fn lock_write(&self) -> Writer<T> {
		#[cfg(feature = "rayon")]
		{
			self.write().unwrap()
		}
		#[cfg(not(feature = "rayon"))]
		{
			self.borrow_mut()
		}
	}
}

/// Additional options for [`find_placement`](Bot::find_placement).
#[derive(Clone, Copy)]
pub struct PlacementOptions {
	/// Maximum distance of checked points from given position. [Default: `15`]
	pub max_distance: isize,
	/// Step between each checked position.  [Default: `2`]
	pub step: isize,
	/// Return random found point if `true`, or closest to given position. [Default: `false`]
	pub random: bool,
	/// Filter positions where addon can fit. [Default: `false`]
	pub addon: bool,
}
impl Default for PlacementOptions {
	fn default() -> Self {
		Self {
			max_distance: 15,
			step: 2,
			random: false,
			addon: false,
		}
	}
}

/**
Options used to configure which units are counted.
Constructed with [`counter`] method.

# Examples
Count all ready marines:
```rust
let count = self.counter().count(UnitTypeId::Marine);
```

Count all supplies in progress:
```rust
let count = self.counter().ordered().count(UnitTypeId::SupplyDepot);
```

Count all ready and ordered nexuses:
```rust
let count = self.counter().all().count(UnitTypeId::Nexus);
```

Count all ready zerglings, taking burrowed ones into accont:
```rust
let count = self.counter().alias().count(UnitTypeId::Zergling);
```

Count all terran bases and alias (orbital, planetary fortress), including ccs in progress:
```rust
let count = self.counter().all().tech().count(UnitTypeId::CommandCenter);
```

[`counter`]: Bot::counter
*/
#[derive(Clone, Copy)]
pub struct CountOptions<'a> {
	bot: &'a Bot,
	/// State of counted units.
	/// Can be:
	/// - `Complete` - only complete units
	/// - `Ordered` - only units in progress
	/// - `All` - both compete and ordered units
	///
	/// [Default: `Complete`]
	pub completion: Completion,
	/// Alias of counted units.
	/// Can be:
	/// - `None` - don't count alias
	/// - `Unit` - count unit-alias, used when unit has 2 forms
	/// - `Tech` - count tech-alias, used when unit has more than 2 forms (usually structures)
	///
	/// [Default: `None`]
	pub alias: UnitAlias,
}
impl<'a> CountOptions<'a> {
	pub(crate) fn new(bot: &'a Bot) -> Self {
		Self {
			bot,
			completion: Default::default(),
			alias: Default::default(),
		}
	}
	/// Sets [`completion`](Self::completion) to `Ordered`.
	pub fn ordered(&mut self) -> &mut Self {
		self.completion = Completion::Ordered;
		self
	}
	/// Sets [`completion`](Self::completion) to `All`.
	pub fn all(&mut self) -> &mut Self {
		self.completion = Completion::All;
		self
	}
	/// Sets [`alias`](Self::alias) to `Unit`.
	pub fn alias(&mut self) -> &mut Self {
		self.alias = UnitAlias::Unit;
		self
	}
	/// Sets [`alias`](Self::alias) to `Tech`.
	pub fn tech(&mut self) -> &mut Self {
		self.alias = UnitAlias::Tech;
		self
	}
	/// Counts units of given type and returns the result.
	pub fn count(&self, unit_id: UnitTypeId) -> usize {
		let bot = self.bot;
		let count: Box<dyn Fn(UnitTypeId) -> usize> = match self.completion {
			Completion::Complete => Box::new(|id| bot.current_units.get(&id).copied().unwrap_or(0)),
			Completion::Ordered => Box::new(|id| {
				bot.game_data.units[&id]
					.ability
					.and_then(|ability| bot.orders.get(&ability).copied())
					.unwrap_or(0)
			}),
			Completion::All => Box::new(|id| {
				bot.current_units.get(&id).copied().unwrap_or(0)
					+ bot.game_data.units[&id]
						.ability
						.and_then(|ability| bot.orders.get(&ability).copied())
						.unwrap_or(0)
			}),
		};
		match self.alias {
			UnitAlias::None => count(unit_id),
			UnitAlias::Unit => count(unit_id) + UNIT_ALIAS.get(&unit_id).copied().map(count).unwrap_or(0),
			UnitAlias::Tech => {
				count(unit_id)
					+ TECH_ALIAS
						.get(&unit_id)
						.map_or(0, |alias| alias.iter().copied().map(count).sum::<usize>())
			}
		}
	}
}

/// Alias of counted units, used in [`CountOptions`].
#[derive(Clone, Copy)]
pub enum UnitAlias {
	/// Don't count alias.
	None,
	/// Count unit-alias, used when unit has 2 forms.
	Unit,
	/// Count tech-alias, used when unit has more than 2 forms (usually structures).
	Tech,
}
impl Default for UnitAlias {
	fn default() -> Self {
		Self::None
	}
}

/// State of counted units, used in [`CountOptions`].
#[derive(Clone, Copy)]
pub enum Completion {
	/// Only complete units.
	Complete,
	/// Only units in progress.
	Ordered,
	/// Both compete and ordered units.
	All,
}
impl Default for Completion {
	fn default() -> Self {
		Self::Complete
	}
}

/// Main bot struct.
/// Structs with `#[bot]` attribute will get all it's fields and methods
/// through `Deref` and `DerefMut` traits.
pub struct Bot {
	pub(crate) process: Option<Child>,
	pub(crate) api: Option<API>,
	/// Sets number of frames between every [`on_step`](crate::Player::on_step). Must be bigger than `0`.
	pub game_step: u32,
	#[doc(hidden)]
	pub disable_fog: bool,
	/// Actual race of your bot.
	pub race: Race,
	/// Requested race of your opponent.
	pub enemy_race: Race,
	/// Your in-game id.
	pub player_id: u32,
	/// Opponent in-game id.
	pub enemy_player_id: u32,
	/// Opponent id on ladder, filled by `--OpponentId`.
	pub opponent_id: String,
	actions: Vec<Action>,
	commander: Rw<Commander>,
	/// Debug API
	pub debug: Debugger,
	/// Information about map.
	pub game_info: GameInfo,
	/// Constant information about abilities, unit types, upgrades, buffs and effects.
	pub game_data: Rs<GameData>,
	/// Information about current state, updated each step.
	pub state: GameState,
	/// Values, which depend on bot's race
	pub race_values: Rs<RaceValues>,
	data_for_unit: SharedUnitData,
	/// Structured collection of units.
	pub units: AllUnits,
	pub(crate) abilities_units: Rs<FxHashMap<u64, FxHashSet<AbilityId>>>,
	orders: FxHashMap<AbilityId, usize>,
	current_units: FxHashMap<UnitTypeId, usize>,
	/// In-game time in seconds.
	pub time: f32,
	/// Amount of minerals bot has.
	pub minerals: u32,
	/// Amount of gas bot has.
	pub vespene: u32,
	/// Amount of supply used by army.
	pub supply_army: u32,
	/// Amount of supply used by workers.
	pub supply_workers: u32,
	/// The supply limit.
	pub supply_cap: u32,
	/// Total supply used.
	pub supply_used: u32,
	/// Amount of free supply.
	pub supply_left: u32,
	/// Bot's starting location.
	pub start_location: Point2,
	/// Opponent's starting location.
	pub enemy_start: Point2,
	/// Bot's resource center on start location.
	pub start_center: Point2,
	/// Opponents's resource center on start location.
	pub enemy_start_center: Point2,
	techlab_tags: Rw<FxHashSet<u64>>,
	reactor_tags: Rw<FxHashSet<u64>>,
	/// All expansions stored in (location, resource center) pairs.
	pub expansions: Vec<(Point2, Point2)>,
	max_cooldowns: Rw<FxHashMap<UnitTypeId, f32>>,
	last_units_health: Rs<FxHashMap<u64, u32>>,
	/// Obstacles on map which block vision of ground units, but still pathable.
	pub vision_blockers: Vec<Point2>,
	/// Ramps on map.
	pub ramps: Ramps,
	enemy_upgrades: Rw<FxHashSet<UpgradeId>>,
	pub(crate) owned_tags: FxHashSet<u64>,
	pub(crate) under_construction: FxHashSet<u64>,
}

impl Bot {
	/// Interface for interacting with SC2 API through Request/Response.
	#[inline]
	pub fn api(&mut self) -> &mut API {
		self.api.as_mut().expect("API is not initialized")
	}
	/// Constructs new [`CountOptions`], used to count units fast and easy.
	pub fn counter(&self) -> CountOptions {
		CountOptions::new(self)
	}
	pub(crate) fn get_data_for_unit(&self) -> SharedUnitData {
		Rs::clone(&self.data_for_unit)
	}
	pub(crate) fn get_actions(&mut self) -> &[Action] {
		let actions = &mut self.actions;

		let mut commander = self.commander.lock_write();

		if !commander.commands.is_empty() {
			actions.extend(
				commander
					.commands
					.drain()
					.map(|((ability, target, queue), units)| {
						Action::UnitCommand(ability, target, units, queue)
					}),
			);
		}
		if !commander.autocast.is_empty() {
			actions.extend(
				commander
					.autocast
					.drain()
					.map(|(ability, units)| Action::ToggleAutocast(ability, units)),
			);
		}

		actions
	}
	pub(crate) fn clear_actions(&mut self) {
		self.actions.clear();
	}
	pub(crate) fn get_debug_commands(&mut self) -> &[DebugCommand] {
		self.debug.get_commands()
	}
	pub(crate) fn clear_debug_commands(&mut self) {
		self.debug.clear_commands();
	}
	/// Returns full cost of building given unit type, without any corrections.
	pub fn get_unit_api_cost(&self, unit: UnitTypeId) -> Cost {
		self.game_data
			.units
			.get(&unit)
			.map_or_else(Default::default, |data| data.cost())
	}
	/// Returns correct cost of building given unit type.
	pub fn get_unit_cost(&self, unit: UnitTypeId) -> Cost {
		let mut cost = self
			.game_data
			.units
			.get(&unit)
			.map_or_else(Default::default, |data| data.cost());
		match unit {
			UnitTypeId::OrbitalCommand | UnitTypeId::PlanetaryFortress => {
				cost.minerals = 150;
			}
			UnitTypeId::Reactor => {
				cost.minerals = 50;
				cost.vespene = 50;
			}
			UnitTypeId::TechLab => {
				cost.minerals = 50;
				cost.vespene = 25;
			}
			UnitTypeId::Zergling => {
				cost.minerals *= 2;
			}
			_ => {}
		}
		cost
	}
	/// Checks if bot has enough resources and supply to build given unit type.
	pub fn can_afford(&self, unit: UnitTypeId, check_supply: bool) -> bool {
		let cost = self.get_unit_cost(unit);
		if self.minerals < cost.minerals || self.vespene < cost.vespene {
			return false;
		}
		if check_supply && (self.supply_left as f32) < cost.supply {
			return false;
		}
		true
	}
	/// Checks cost of making given upgrade.
	pub fn get_upgrade_cost(&self, upgrade: UpgradeId) -> Cost {
		self.game_data
			.upgrades
			.get(&upgrade)
			.map_or_else(Default::default, |data| data.cost())
	}
	/// Checks if bot has enough resources to make given upgrade.
	pub fn can_afford_upgrade(&self, upgrade: UpgradeId) -> bool {
		let cost = self.get_upgrade_cost(upgrade);
		self.minerals >= cost.minerals && self.vespene >= cost.vespene
	}
	/*
	fn can_afford_ability(&self, ability: AbilityId) -> bool {
		unimplemented!()
	}
	*/
	/// Subtracts cost of given unit type from [`minerals`], [`vespene`], [`supply_left`] and adds to [`supply_used`].
	///
	/// [`minerals`]: Self::minerals
	/// [`vespene`]: Self::vespene
	/// [`supply_left`]: Self::supply_left
	/// [`supply_used`]: Self::supply_used
	pub fn subtract_resources(&mut self, unit: UnitTypeId, subtract_supply: bool) {
		let cost = self.get_unit_cost(unit);
		self.minerals = self.minerals.saturating_sub(cost.minerals);
		self.vespene = self.vespene.saturating_sub(cost.vespene);
		if subtract_supply {
			let supply_cost = cost.supply as u32;
			self.supply_used += supply_cost;
			self.supply_left = self.supply_left.saturating_sub(supply_cost);
		}
	}
	/// Subtracts cost of given upgrade from [`minerals`] and [`vespene`].
	///
	/// [`minerals`]: Self::minerals
	/// [`vespene`]: Self::vespene
	pub fn subtract_upgrade_cost(&mut self, upgrade: UpgradeId) {
		let cost = self.get_upgrade_cost(upgrade);
		self.minerals = self.minerals.saturating_sub(cost.minerals);
		self.vespene = self.vespene.saturating_sub(cost.vespene);
	}
	/// Checks if given upgrade is complete.
	pub fn has_upgrade(&self, upgrade: UpgradeId) -> bool {
		self.state.observation.raw.upgrades.lock_read().contains(&upgrade)
	}
	/// Checks if predicted opponent's upgrades contains given upgrade.
	pub fn enemy_has_upgrade(&self, upgrade: UpgradeId) -> bool {
		self.enemy_upgrades.lock_read().contains(&upgrade)
	}
	/// Returns mutable set of predicted opponent's upgrades.
	pub fn enemy_upgrades(&self) -> Writer<FxHashSet<UpgradeId>> {
		self.enemy_upgrades.lock_write()
	}
	/// Checks if upgrade is in progress.
	pub fn is_ordered_upgrade(&self, upgrade: UpgradeId) -> bool {
		let ability = self.game_data.upgrades[&upgrade].ability;
		self.orders
			.get(&ability)
			.copied()
			.map_or(false, |count| count > 0)
	}
	/// Returns progress of making given upgrade.
	/// - `1` - complete
	/// - `0` - not even ordered
	/// - `0..1` - in progress
	pub fn upgrade_progress(&self, upgrade: UpgradeId) -> f32 {
		if self.has_upgrade(upgrade) {
			return 1.0;
		}
		if !self.is_ordered_upgrade(upgrade) {
			return 0.0;
		}

		let ability = self.game_data.upgrades[&upgrade].ability;
		self.units
			.my
			.structures
			.iter()
			.filter(|s| s.is_ready())
			.find_map(|s| {
				s.orders
					.iter()
					.find(|order| order.ability == ability)
					.map(|order| order.progress)
			})
			.unwrap_or(0.0)
	}
	/// Sends message to in-game chat.
	pub fn chat(&mut self, message: &str) {
		self.actions.push(Action::Chat(message.to_string(), false));
	}
	/// Returns actual terrain height on given position in 3D space.
	pub fn get_z_height<P: Into<(usize, usize)>>(&self, pos: P) -> f32 {
		self.game_info.terrain_height[pos.into()] as f32 * 32.0 / 255.0 - 16.0
	}
	/// Returns terrain height on given position.
	pub fn get_height<P: Into<(usize, usize)>>(&self, pos: P) -> u8 {
		self.game_info.terrain_height[pos.into()]
	}
	/// Checks if it's possible to build on given position.
	pub fn is_placeable<P: Into<(usize, usize)>>(&self, pos: P) -> bool {
		self.game_info.placement_grid[pos.into()].is_empty()
	}
	/// Checks if it's possible for ground units to walk through given position.
	pub fn is_pathable<P: Into<(usize, usize)>>(&self, pos: P) -> bool {
		self.game_info.pathing_grid[pos.into()].is_empty()
	}
	/// Checks if given position is hidden (weren't explored before).
	pub fn is_hidden<P: Into<(usize, usize)>>(&self, pos: P) -> bool {
		self.state.observation.raw.visibility[pos.into()].is_hidden()
	}
	/// Checks if given position is in fog of war (was explored before).
	pub fn is_fogged<P: Into<(usize, usize)>>(&self, pos: P) -> bool {
		self.state.observation.raw.visibility[pos.into()].is_fogged()
	}
	/// Checks if given position is visible now.
	pub fn is_visible<P: Into<(usize, usize)>>(&self, pos: P) -> bool {
		self.state.observation.raw.visibility[pos.into()].is_visible()
	}
	/// Checks if given position is fully hidden
	/// (terrain isn't visible, only darkness; only in campain and custom maps).
	pub fn is_full_hidden<P: Into<(usize, usize)>>(&self, pos: P) -> bool {
		self.state.observation.raw.visibility[pos.into()].is_full_hidden()
	}
	/// Checks if given position is not hidden (was explored before).
	pub fn is_explored<P: Into<(usize, usize)>>(&self, pos: P) -> bool {
		self.state.observation.raw.visibility[pos.into()].is_explored()
	}
	/// Checks if given position has zerg's creep.
	pub fn has_creep<P: Into<(usize, usize)>>(&self, pos: P) -> bool {
		self.state.observation.raw.creep[pos.into()].is_empty()
	}
	pub(crate) fn update_data_for_unit(&mut self) {
		self.data_for_unit = Rs::new(DataForUnit {
			commander: Rs::clone(&self.commander),
			game_data: Rs::clone(&self.game_data),
			techlab_tags: Rs::clone(&self.techlab_tags),
			reactor_tags: Rs::clone(&self.reactor_tags),
			race_values: Rs::clone(&self.race_values),
			max_cooldowns: Rs::clone(&self.max_cooldowns),
			last_units_health: Rs::clone(&self.last_units_health),
			abilities_units: Rs::clone(&self.abilities_units),
			enemy_upgrades: Rs::clone(&self.enemy_upgrades),
			upgrades: Rs::clone(&self.state.observation.raw.upgrades),
			creep: Rs::clone(&self.state.observation.raw.creep),
			visibility: Rs::clone(&self.state.observation.raw.visibility),
			game_step: self.game_step,
		});
	}
	pub(crate) fn prepare_start(&mut self) {
		self.race = self.game_info.players[&self.player_id].race_actual.unwrap();
		if self.game_info.players.len() == 2 {
			let enemy_player_id = 3 - self.player_id;
			self.enemy_race = self.game_info.players[&enemy_player_id].race_requested;
			self.enemy_player_id = enemy_player_id;
		}
		self.race_values = Rs::new(RACE_VALUES[&self.race].clone());
		self.update_units();

		self.update_data_for_unit();

		if let Some(townhall) = self.units.my.townhalls.first() {
			self.start_location = townhall.position;
		}
		self.enemy_start = self.game_info.start_locations[0];

		let resources = self.units.resources.closer(11.0, self.start_location);
		self.start_center =
			(resources.sum(|r| r.position) + self.start_location) / (resources.len() + 1) as f32;

		let resources = self.units.resources.closer(11.0, self.enemy_start);
		self.enemy_start_center =
			(resources.sum(|r| r.position) + self.enemy_start) / (resources.len() + 1) as f32;

		// Calculating expansion locations

		const RESOURCE_SPREAD: f32 = 72.25; // 8.5

		let all_resources = self
			.units
			.resources
			.filter(|r| r.type_id != UnitTypeId::MineralField450);

		let positions = all_resources
			.iter()
			.map(|r| (r.position, r.tag))
			.collect::<Vec<(Point2, u64)>>();

		let resource_groups = dbscan(
			&positions,
			range_query(
				&positions,
				|(p1, _), (p2, _)| p1.distance_squared(*p2),
				RESOURCE_SPREAD,
			),
			4,
		)
		.0;

		const OFFSET: isize = 7;
		let offsets = iproduct!((-OFFSET..=OFFSET), (-OFFSET..=OFFSET))
			.filter(|(x, y)| x * x + y * y <= 64)
			.collect::<Vec<(isize, isize)>>();

		self.expansions = resource_groups
			.iter()
			.map(|group| {
				let resources = all_resources.find_tags(group.iter().map(|(_, tag)| tag));
				let center = resources.center().unwrap().floor() + 0.5;

				if center.is_closer(4.0, self.start_center) {
					(self.start_location, self.start_center)
				} else if center.is_closer(4.0, self.enemy_start_center) {
					(self.enemy_start, self.enemy_start_center)
				} else {
					let location = offsets
						.iter()
						.filter_map(|(x, y)| {
							let pos = center.offset(*x as f32, *y as f32);
							if self.is_placeable((pos.x as usize, pos.y as usize)) {
								let mut distance_sum = 0_f32;
								let far_enough = |r: &Unit| {
									let dist = pos.distance_squared(r);
									distance_sum += dist;
									dist > if r.is_geyser() { 49.0 } else { 36.0 }
								};
								if resources.iter().all(far_enough) {
									return Some((pos, distance_sum));
								}
							}
							None
						})
						.min_by(|(_, d1), (_, d2)| d1.partial_cmp(d2).unwrap())
						.expect("Can't detect right position for expansion")
						.0;

					(
						location,
						(resources.sum(|r| r.position) + location) / (resources.len() + 1) as f32,
					)
				}
			})
			.collect();

		// Calclulating ramp locations
		let mut ramp_points = FxHashSet::default();

		let area = self.game_info.playable_area;
		iproduct!(area.x0..area.x1, area.y0..area.y1).for_each(|pos| {
			if !self.is_pathable(pos) || self.is_placeable(pos) {
				return;
			}

			let h = self.get_height(pos);
			let (x, y) = pos;

			let neighbors = [
				(x + 1, y),
				(x - 1, y),
				(x, y + 1),
				(x, y - 1),
				(x + 1, y + 1),
				(x - 1, y - 1),
				(x + 1, y - 1),
				(x - 1, y + 1),
			];

			if neighbors.iter().all(|p| self.get_height(*p) == h) {
				self.vision_blockers.push(Point2::new(x as f32, y as f32));
			} else {
				ramp_points.insert(pos);
			}
		});

		let ramps = dbscan(
			&ramp_points,
			|&(x, y)| {
				[
					(x + 1, y),
					(x - 1, y),
					(x, y + 1),
					(x, y - 1),
					(x + 1, y + 1),
					(x - 1, y - 1),
					(x + 1, y - 1),
					(x - 1, y + 1),
				]
				.iter()
				.filter(|n| ramp_points.contains(n))
				.copied()
				.collect()
			},
			1,
		)
		.0
		.into_iter()
		.filter(|ps| ps.len() >= 8)
		.map(|ps| Ramp::new(ps, &self.game_info.terrain_height, self.start_location))
		.collect::<Vec<Ramp>>();

		let get_closest_ramp = |loc: Point2| {
			let (loc_x, loc_y) = ((loc.x + 0.5) as usize, (loc.y + 0.5) as usize);
			let cmp = |r: &&Ramp| {
				let (x, y) = r.top_center().unwrap();
				let dx = loc_x.checked_sub(x).unwrap_or_else(|| x - loc_x);
				let dy = loc_y.checked_sub(y).unwrap_or_else(|| y - loc_y);
				dx * dx + dy * dy
			};
			ramps
				.iter()
				.filter(|r| {
					let upper_len = r.upper().len();
					upper_len == 2 || upper_len == 5
				})
				.min_by_key(cmp)
				.or_else(|| {
					ramps
						.iter()
						.filter(|r| {
							let upper_len = r.upper().len();
							upper_len == 4 || upper_len == 9
						})
						.min_by_key(cmp)
				})
				.cloned()
		};

		if let Some(ramp) = get_closest_ramp(self.start_location) {
			self.ramps.my = ramp;
		}
		if let Some(ramp) = get_closest_ramp(self.enemy_start) {
			self.ramps.enemy = ramp;
		}

		self.ramps.all = ramps;
	}
	pub(crate) fn prepare_step(&mut self) {
		self.last_units_health = Rs::new(
			self.units
				.all
				.iter()
				.filter_map(|u| Some((u.tag, u.hits()?)))
				.collect(),
		);
		self.update_units();
		let observation = &self.state.observation;
		self.time = (observation.game_loop as f32) / 22.4;
		let common = &observation.common;
		self.minerals = common.minerals;
		self.vespene = common.vespene;
		self.supply_army = common.food_army;
		self.supply_workers = common.food_workers;
		self.supply_cap = common.food_cap;
		self.supply_used = common.food_used;
		self.supply_left = self.supply_cap - self.supply_used;

		self.update_data_for_unit();

		// Counting units and orders
		let mut current_units = FxHashMap::default();
		let mut orders = FxHashMap::default();
		self.units.my.all.iter().for_each(|u| {
			u.orders.iter().for_each(|order| {
				if !order.ability.is_constructing() {
					*orders.entry(order.ability).or_default() += 1
				}
			});

			if u.is_ready() && !u.is_placeholder() {
				*current_units.entry(u.type_id).or_default() += 1;
			} else if let Some(data) = self.game_data.units.get(&u.type_id) {
				if let Some(ability) = data.ability {
					*orders.entry(ability).or_default() += 1;
				}
			}
		});
		self.current_units = current_units;
		self.orders = orders;
	}
	fn update_units(&mut self) {
		self.units.clear();

		let mut techlab_tags = self.techlab_tags.lock_write();
		let mut reactor_tags = self.reactor_tags.lock_write();
		let mut max_cooldowns = self.max_cooldowns.lock_write();

		techlab_tags.clear();
		reactor_tags.clear();

		let units = &mut self.units;
		self.state.observation.raw.units.iter().for_each(|u| {
			macro_rules! add_to {
				($group:expr) => {{
					$group.push(u.clone());
					}};
			}

			let u_type = u.type_id;
			match u.alliance {
				Alliance::Neutral => match u_type {
					UnitTypeId::XelNagaTower => add_to!(units.watchtowers),

					UnitTypeId::RichMineralField
					| UnitTypeId::RichMineralField750
					| UnitTypeId::MineralField
					| UnitTypeId::MineralField450
					| UnitTypeId::MineralField750
					| UnitTypeId::LabMineralField
					| UnitTypeId::LabMineralField750
					| UnitTypeId::PurifierRichMineralField
					| UnitTypeId::PurifierRichMineralField750
					| UnitTypeId::PurifierMineralField
					| UnitTypeId::PurifierMineralField750
					| UnitTypeId::BattleStationMineralField
					| UnitTypeId::BattleStationMineralField750
					| UnitTypeId::MineralFieldOpaque
					| UnitTypeId::MineralFieldOpaque900 => {
						add_to!(units.resources);
						add_to!(units.mineral_fields);
					}
					UnitTypeId::VespeneGeyser
					| UnitTypeId::SpacePlatformGeyser
					| UnitTypeId::RichVespeneGeyser
					| UnitTypeId::ProtossVespeneGeyser
					| UnitTypeId::PurifierVespeneGeyser
					| UnitTypeId::ShakurasVespeneGeyser => {
						add_to!(units.resources);
						add_to!(units.vespene_geysers);
					}
					id if INHIBITOR_IDS.contains(&id) => add_to!(units.inhibitor_zones),

					_ => add_to!(units.destructables),
				},
				Alliance::Own | Alliance::Enemy => {
					let units = if u.is_mine() {
						&mut units.my
					} else {
						&mut units.enemy
					};
					add_to!(units.all);
					if u.is_mine() {
						if let Some(cooldown) = u.weapon_cooldown {
							max_cooldowns
								.entry(u_type)
								.and_modify(|max| {
									if cooldown > *max {
										*max = cooldown;
									}
								})
								.or_insert(cooldown);
						}
					}
					if u.is_structure() {
						if u.is_placeholder() {
							add_to!(units.placeholders);
						} else {
							add_to!(units.structures);
							match u_type {
								UnitTypeId::CommandCenter
								| UnitTypeId::OrbitalCommand
								| UnitTypeId::PlanetaryFortress
								| UnitTypeId::CommandCenterFlying
								| UnitTypeId::OrbitalCommandFlying
								| UnitTypeId::Hatchery
								| UnitTypeId::Lair
								| UnitTypeId::Hive
								| UnitTypeId::Nexus => add_to!(units.townhalls),

								UnitTypeId::Refinery
								| UnitTypeId::RefineryRich
								| UnitTypeId::Assimilator
								| UnitTypeId::AssimilatorRich
								| UnitTypeId::Extractor
								| UnitTypeId::ExtractorRich => add_to!(units.gas_buildings),

								UnitTypeId::TechLab
								| UnitTypeId::BarracksTechLab
								| UnitTypeId::FactoryTechLab
								| UnitTypeId::StarportTechLab
									if u.is_mine() =>
								{
									techlab_tags.insert(u.tag);
								}

								UnitTypeId::Reactor
								| UnitTypeId::BarracksReactor
								| UnitTypeId::FactoryReactor
								| UnitTypeId::StarportReactor
									if u.is_mine() =>
								{
									reactor_tags.insert(u.tag);
								}

								_ => {}
							}
						}
					} else {
						add_to!(units.units);
						match u_type {
							UnitTypeId::SCV | UnitTypeId::Probe | UnitTypeId::Drone => add_to!(units.workers),
							UnitTypeId::Larva => add_to!(units.larvas),
							_ => {}
						}
					}
				}
				_ => {}
			}
		});
		units.all = self.state.observation.raw.units.clone();
	}

	/// Simple wrapper around [`query_placement`](Self::query_placement).
	/// Checks if it's possible to build given building on given position.
	pub fn can_place(&mut self, building: UnitTypeId, pos: Point2) -> bool {
		self.query_placement(
			vec![(self.game_data.units[&building].ability.unwrap(), pos, None)],
			false,
		)
		.unwrap()[0] == ActionResult::Success
	}
	/// Simple wrapper around [`query_placement`](Self::query_placement).
	/// Multi-version of [`can_place`](Self::can_place).
	pub fn can_place_some(&mut self, places: Vec<(UnitTypeId, Point2)>) -> Vec<bool> {
		self.query_placement(
			places
				.into_iter()
				.map(|(building, pos)| (self.game_data.units[&building].ability.unwrap(), pos, None))
				.collect(),
			false,
		)
		.unwrap()
		.into_iter()
		.map(|r| r == ActionResult::Success)
		.collect()
	}

	/// Nice wrapper around [`query_placement`](Self::query_placement).
	/// Returns correct position where it is possible to build given `building`,
	/// or `None` if position is not found or `building` can't be built by a worker.
	pub fn find_placement(
		&mut self,
		building: UnitTypeId,
		near: Point2,
		options: PlacementOptions,
	) -> Option<Point2> {
		if let Some(data) = self.game_data.units.get(&building) {
			if let Some(ability) = data.ability {
				let addon = options.addon;
				if self
					.query_placement(
						if addon {
							vec![
								(ability, near, None),
								(AbilityId::TerranBuildSupplyDepot, near.offset(2.5, -0.5), None),
							]
						} else {
							vec![(ability, near, None)]
						},
						false,
					)
					.unwrap()
					.iter()
					.all(|r| matches!(r, ActionResult::Success))
				{
					return Some(near);
				}

				let placement_step = options.step;
				for distance in (placement_step..options.max_distance).step_by(placement_step as usize) {
					let positions = (-distance..=distance)
						.step_by(placement_step as usize)
						.flat_map(|offset| {
							vec![
								near.offset(offset as f32, (-distance) as f32),
								near.offset(offset as f32, distance as f32),
								near.offset((-distance) as f32, offset as f32),
								near.offset(distance as f32, offset as f32),
							]
						})
						.collect::<Vec<Point2>>();
					let results = self
						.query_placement(positions.iter().map(|pos| (ability, *pos, None)).collect(), false)
						.unwrap();

					let mut valid_positions = positions
						.iter()
						.zip(results.iter())
						.filter_map(|(pos, res)| {
							if matches!(res, ActionResult::Success) {
								Some(*pos)
							} else {
								None
							}
						})
						.collect::<Vec<Point2>>();

					if addon {
						let results = self
							.query_placement(
								valid_positions
									.iter()
									.map(|pos| {
										(AbilityId::TerranBuildSupplyDepot, pos.offset(2.5, -0.5), None)
									})
									.collect(),
								false,
							)
							.unwrap();
						valid_positions = valid_positions
							.iter()
							.zip(results.iter())
							.filter_map(|(pos, res)| {
								if *res == ActionResult::Success {
									Some(*pos)
								} else {
									None
								}
							})
							.collect::<Vec<Point2>>();
					}

					if !valid_positions.is_empty() {
						return if options.random {
							valid_positions.choose(&mut thread_rng()).copied()
						} else {
							valid_positions.iter().closest(near).copied()
						};
					}
				}
			}
		}
		None
	}
	/// Another wrapper around [`query_placement`](Self::query_placement),
	/// used to find free geyser near given base.
	///
	/// Returns `Unit` of geyser or `None` if there're no free geysers around given base.
	pub fn find_gas_placement(&mut self, base: Point2) -> Option<Unit> {
		let ability = self.game_data.units[&self.race_values.gas].ability.unwrap();

		let geysers = self.units.vespene_geysers.closer(11.0, base);
		let results = self
			.query_placement(
				geysers.iter().map(|u| (ability, u.position, None)).collect(),
				false,
			)
			.unwrap();

		geysers
			.iter()
			.zip(results.iter())
			.find(|(_, res)| **res == ActionResult::Success)
			.map(|(geyser, _)| geyser.clone())
	}

	/// Returns next possible location from [`expansions`](Self::expansions) closest to bot's start location
	/// or `None` if there aren't any free locations.
	pub fn get_expansion(&mut self) -> Option<(Point2, Point2)> {
		let expansions = self
			.expansions
			.iter()
			.filter(|(loc, _)| {
				self.units.my.townhalls.iter().all(|t| t.is_further(7.0, *loc))
					&& self.units.my.placeholders.iter().all(|p| p.is_further(7.0, *loc))
			})
			.copied()
			.collect::<Vec<(Point2, Point2)>>();
		let paths = self
			.query_pathing(
				expansions
					.iter()
					.map(|(loc, _)| (Target::Pos(self.start_location), *loc))
					.collect(),
			)
			.unwrap();

		expansions
			.iter()
			.zip(paths.iter())
			.filter_map(|(loc, path)| path.map(|path| (loc, path)))
			.min_by(|(_, path1), (_, path2)| path1.partial_cmp(&path2).unwrap())
			.map(|(loc, _path)| *loc)
	}
	/// Returns next possible location from [`expansions`](Self::expansions) closest to opponent's start location
	/// or `None` if there aren't any free locations.
	pub fn get_enemy_expansion(&mut self) -> Option<(Point2, Point2)> {
		let expansions = self
			.expansions
			.iter()
			.filter(|(loc, _)| self.units.enemy.townhalls.iter().all(|t| t.is_further(7.0, *loc)))
			.copied()
			.collect::<Vec<(Point2, Point2)>>();
		let paths = self
			.query_pathing(
				expansions
					.iter()
					.map(|(loc, _)| (Target::Pos(self.enemy_start), *loc))
					.collect(),
			)
			.unwrap();

		expansions
			.iter()
			.zip(paths.iter())
			.filter_map(|(loc, path)| path.map(|path| (loc, path)))
			.min_by(|(_, path1), (_, path2)| path1.partial_cmp(&path2).unwrap())
			.map(|(loc, _path)| *loc)
	}
	/// Returns all [`expansions`](Self::expansions) taken by bot.
	pub fn owned_expansions(&self) -> Vec<(Point2, Point2)> {
		self.expansions
			.iter()
			.filter(|(loc, _)| self.units.my.townhalls.iter().any(|t| t.is_closer(15.0, *loc)))
			.copied()
			.collect()
	}
	/// Returns all [`expansions`](Self::expansions) taken by opponent.
	pub fn enemy_expansions(&self) -> Vec<(Point2, Point2)> {
		self.expansions
			.iter()
			.filter(|(loc, _)| self.units.enemy.townhalls.iter().any(|t| t.is_closer(15.0, *loc)))
			.copied()
			.collect()
	}
	/// Returns all avaliable [`expansions`](Self::expansions).
	pub fn free_expansions(&self) -> Vec<(Point2, Point2)> {
		self.expansions
			.iter()
			.filter(|(loc, _)| {
				self.units.my.townhalls.iter().all(|t| t.is_further(15.0, *loc))
					&& self
						.units
						.enemy
						.townhalls
						.iter()
						.all(|t| t.is_further(15.0, *loc))
			})
			.copied()
			.collect()
	}
	/// Sends pathing requests to API.
	///
	/// Takes `Vec` of (start, goal), where `start` is position or unit tag and `goal` is position.
	///
	/// Returns `Vec` ordered by input values,
	/// where element is distance of path from start to goal or `None` if there's no path.
	pub fn query_pathing(&mut self, paths: Vec<(Target, Point2)>) -> SC2Result<Vec<Option<f32>>> {
		let mut req = Request::new();
		let req_pathing = req.mut_query().mut_pathing();

		paths.iter().for_each(|(start, goal)| {
			let mut pathing = RequestQueryPathing::new();
			match start {
				Target::Tag(tag) => pathing.set_unit_tag(*tag),
				Target::Pos(pos) => pathing.set_start_pos(pos.into_proto()),
				Target::None => panic!("start pos is not specified in query pathing request"),
			}
			pathing.set_end_pos(goal.into_proto());
			req_pathing.push(pathing);
		});

		let res = self.api().send(req)?;
		Ok(res
			.get_query()
			.get_pathing()
			.iter()
			.map(|result| result.distance)
			.collect())
	}
	/// Sends placement requests to API.
	/// Takes creep, psionic matrix, and other stuff into account.
	///
	/// Returned results will be successful when:
	/// - given ability can be used by worker
	/// - `check_resources` is `false` or bot has enough resources to use given ability
	/// - worker tag is `None` or worker can reach given position
	/// - given place is free of obstacles
	///
	/// Takes `Vec` of (build ability, position, tag of worker or `None`).
	///
	/// Returns `Vec` of [`ActionResult`] ordered by input values.
	pub fn query_placement(
		&mut self,
		places: Vec<(AbilityId, Point2, Option<u64>)>,
		check_resources: bool,
	) -> SC2Result<Vec<ActionResult>> {
		let mut req = Request::new();
		let req_query = req.mut_query();
		req_query.set_ignore_resource_requirements(!check_resources);
		let req_placement = req_query.mut_placements();

		places.iter().for_each(|(ability, pos, builder)| {
			let mut placement = RequestQueryBuildingPlacement::new();
			placement.set_ability_id(ability.to_i32().unwrap());
			placement.set_target_pos(pos.into_proto());
			if let Some(tag) = builder {
				placement.set_placing_unit_tag(*tag);
			}
			req_placement.push(placement);
		});

		let res = self.api().send(req)?;
		Ok(res
			.get_query()
			.get_placements()
			.iter()
			.map(|result| ActionResult::from_proto(result.get_result()))
			.collect())
	}

	/// Leaves current game, which is counted as Defeat for bot.
	///
	/// Note: [`on_end`] will not be called, if needed use [`debug.end_game`] instead.
	///
	/// [`on_end`]: crate::Player::on_end
	/// [`debug.end_game`]: Debugger::end_game
	pub fn leave(&mut self) -> SC2Result<()> {
		let mut req = Request::new();
		req.mut_leave_game();
		self.api().send_request(req)
	}

	pub(crate) fn close_client(&mut self) {
		if let Some(api) = &mut self.api {
			let mut req = Request::new();
			req.mut_leave_game();
			if let Err(e) = api.send_request(req) {
				error!("Request LeaveGame failed: {}", e);
			}

			let mut req = Request::new();
			req.mut_quit();
			if let Err(e) = api.send_request(req) {
				error!("Request QuitGame failed: {}", e);
			}
		}

		if let Some(process) = &mut self.process {
			if let Err(e) = process.kill() {
				error!("Can't kill SC2 process: {}", e);
			}
		}
	}
}

impl Default for Bot {
	fn default() -> Self {
		Self {
			game_step: 1,
			disable_fog: false,
			race: Race::Random,
			enemy_race: Race::Random,
			process: None,
			api: None,
			player_id: Default::default(),
			enemy_player_id: Default::default(),
			opponent_id: Default::default(),
			actions: Default::default(),
			commander: Default::default(),
			debug: Default::default(),
			game_info: Default::default(),
			game_data: Default::default(),
			state: Default::default(),
			race_values: Default::default(),
			data_for_unit: Default::default(),
			units: Default::default(),
			abilities_units: Default::default(),
			orders: Default::default(),
			current_units: Default::default(),
			time: Default::default(),
			minerals: Default::default(),
			vespene: Default::default(),
			supply_army: Default::default(),
			supply_workers: Default::default(),
			supply_cap: Default::default(),
			supply_used: Default::default(),
			supply_left: Default::default(),
			start_location: Default::default(),
			enemy_start: Default::default(),
			start_center: Default::default(),
			enemy_start_center: Default::default(),
			techlab_tags: Default::default(),
			reactor_tags: Default::default(),
			expansions: Default::default(),
			max_cooldowns: Default::default(),
			last_units_health: Default::default(),
			vision_blockers: Default::default(),
			ramps: Default::default(),
			enemy_upgrades: Default::default(),
			owned_tags: Default::default(),
			under_construction: Default::default(),
		}
	}
}

impl Drop for Bot {
	fn drop(&mut self) {
		self.close_client();
	}
}
