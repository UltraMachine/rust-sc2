use crate::{
	action::{Action, ActionResult, Commander, Target},
	constants::{RaceValues, RACE_VALUES},
	debug::{DebugCommand, Debugger},
	game_data::{Cost, GameData},
	game_info::GameInfo,
	game_state::{Alliance, GameState},
	geometry::Point2,
	ids::{AbilityId, UnitTypeId, UpgradeId},
	iproduct,
	player::Race,
	query::QueryMaster,
	unit::{DataForUnit, Unit},
	units::{GroupedUnits, Units},
	Itertools, WS,
};
use rand::prelude::{thread_rng, SliceRandom};
use std::{
	cell::RefCell,
	collections::{HashMap, HashSet},
	rc::Rc,
};

#[derive(Clone)]
pub struct Bot {
	pub game_step: u32,
	pub race: Race,
	pub enemy_race: Race,
	pub player_id: u32,
	pub opponent_id: String,
	pub actions: Vec<Action>,
	pub commander: Rc<RefCell<Commander>>,
	pub debug: Debugger,
	pub query: QueryMaster,
	pub game_info: GameInfo,
	pub game_data: Rc<GameData>,
	pub state: GameState,
	pub race_values: Rc<RaceValues>,
	pub data_for_unit: Rc<DataForUnit>,
	pub grouped_units: GroupedUnits,
	pub abilities_units: HashMap<u64, Vec<AbilityId>>,
	pub orders: HashMap<AbilityId, usize>,
	pub current_units: HashMap<UnitTypeId, usize>,
	pub time: f32,
	pub minerals: u32,
	pub vespene: u32,
	pub supply_army: u32,
	pub supply_workers: u32,
	pub supply_cap: u32,
	pub supply_used: u32,
	pub supply_left: u32,
	pub start_location: Point2,
	pub enemy_start: Point2,
	pub start_center: Point2,
	pub enemy_start_center: Point2,
	pub techlab_tags: Rc<Vec<u64>>,
	pub reactor_tags: Rc<Vec<u64>>,
	pub expansions: Vec<(Point2, Point2)>,
	pub max_cooldowns: Rc<RefCell<HashMap<UnitTypeId, f32>>>,
}

impl Bot {
	pub fn new() -> Self {
		Self {
			game_step: 1,
			race: Race::Random,
			enemy_race: Race::Random,
			player_id: Default::default(),
			opponent_id: Default::default(),
			actions: Default::default(),
			commander: Default::default(),
			debug: Default::default(),
			query: Default::default(),
			game_info: Default::default(),
			game_data: Default::default(),
			state: Default::default(),
			race_values: Default::default(),
			data_for_unit: Default::default(),
			grouped_units: Default::default(),
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
		}
	}
	pub fn get_data_for_unit(&self) -> Rc<DataForUnit> {
		Rc::clone(&self.data_for_unit)
	}
	pub fn get_actions(&self) -> Vec<Action> {
		let commands = &self.commander.borrow().commands;
		if !commands.is_empty() {
			let mut actions = self.actions.clone();
			commands.iter().for_each(|((ability, target, queue), units)| {
				actions.push(Action::UnitCommand(*ability, *target, units.clone(), *queue));
			});
			actions
		} else {
			self.actions.clone()
		}
	}
	pub fn clear_actions(&mut self) {
		self.actions.clear();
		self.commander.borrow_mut().commands.clear();
	}
	pub fn get_debug_commands(&self) -> Vec<DebugCommand> {
		self.debug.get_commands()
	}
	pub fn clear_debug_commands(&mut self) {
		self.debug.clear_commands();
	}
	pub fn substract_resources(&mut self, unit: UnitTypeId) {
		let cost = self.game_data.units[&unit].cost();
		self.minerals = self.minerals.saturating_sub(cost.minerals);
		self.vespene = self.vespene.saturating_sub(cost.vespene);
		let supply_cost = cost.supply as u32;
		self.supply_used += supply_cost;
		self.supply_left = self.supply_left.saturating_sub(supply_cost);
	}
	pub fn substract_upgrade_cost(&mut self, upgrade: UpgradeId) {
		let cost = self.game_data.upgrades[&upgrade].cost();
		self.minerals = self.minerals.saturating_sub(cost.minerals);
		self.vespene = self.vespene.saturating_sub(cost.vespene);
	}
	pub fn has_upgrade(&self, upgrade: UpgradeId) -> bool {
		self.state.observation.raw.upgrades.contains(&upgrade)
	}
	pub fn chat_send(&mut self, message: String, team_only: bool) {
		self.actions.push(Action::Chat(message, team_only));
	}
	pub fn init_data_for_unit(&mut self) {
		self.data_for_unit = Rc::new(DataForUnit {
			commander: Rc::clone(&self.commander),
			game_data: Rc::clone(&self.game_data),
			techlab_tags: Rc::clone(&self.techlab_tags),
			reactor_tags: Rc::clone(&self.reactor_tags),
			race_values: Rc::clone(&self.race_values),
			max_cooldowns: Rc::clone(&self.max_cooldowns),
		});
	}
	#[allow(clippy::block_in_if_condition_stmt)]
	pub fn prepare_start(&mut self, ws: &mut WS) {
		self.race = self.game_info.players[&self.player_id].race_actual.unwrap();
		if self.game_info.players.len() == 2 {
			self.enemy_race = self.game_info.players[&(3 - self.player_id)].race_requested;
		}
		self.race_values = Rc::new(RACE_VALUES[&self.race].clone());

		self.group_units();

		self.start_location = self.grouped_units.townhalls.first().unwrap().position;
		self.enemy_start = self.game_info.start_locations[0];

		self.start_center = self
			.grouped_units
			.resources
			.closer_pos(11.0, self.start_location)
			.center();
		self.enemy_start_center = self
			.grouped_units
			.resources
			.closer_pos(11.0, self.enemy_start)
			.center();

		// Calculating expansion locations
		let all_resources = self
			.grouped_units
			.resources
			.filter(|r| r.type_id != UnitTypeId::MineralField450);

		let mut resource_groups: Vec<HashSet<u64>> = Vec::new();
		all_resources.iter().for_each(|r| {
			if !resource_groups.iter_mut().any(|res| {
				if res
					.iter()
					.any(|other_r| all_resources.find_tag(*other_r).unwrap().distance_squared(r) < 72.25)
				{
					res.insert(r.tag);
					true
				} else {
					false
				}
			}) {
				let mut set = HashSet::new();
				set.insert(r.tag);
				resource_groups.push(set);
			}
		});

		loop {
			let mut unioned = resource_groups
				.clone()
				.iter()
				.combinations(2)
				.filter_map(|res| {
					let res1 = res[0];
					let res2 = res[1];
					if !res1.is_disjoint(&res2)
						|| iproduct!(res1, res2)
							.any(|(r1, r2)| all_resources[*r1].distance_squared(&all_resources[*r2]) < 72.25)
					{
						if let Some(i) = resource_groups.iter().position(|r| r == res1) {
							resource_groups.remove(i);
						}
						if let Some(i) = resource_groups.iter().position(|r| r == res2) {
							resource_groups.remove(i);
						}
						Some(res1.union(&res2).copied().collect())
					} else {
						None
					}
				})
				.collect::<Vec<HashSet<u64>>>();
			if unioned.is_empty() {
				break;
			}
			resource_groups.append(&mut unioned);
		}

		self.expansions = resource_groups
			.iter()
			.filter_map(|res| {
				let center = all_resources.find_tags(res.iter().copied()).center();
				if center.distance_squared(self.start_location) < 72.25 {
					Some((self.start_location, center))
				} else {
					self.find_placement(ws, self.race_values.start_townhall, center, 8, 1, false, false)
						.map(|place| (place, center))
				}
			})
			.collect();
	}
	pub fn prepare_step(&mut self) {
		self.group_units();
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

		// Counting units and orders
		self.current_units.clear();
		self.orders.clear();
		self.grouped_units.owned.clone().iter().for_each(|u| {
			u.orders
				.iter()
				.for_each(|order| *self.orders.entry(order.ability).or_default() += 1);
			if !u.is_ready() {
				if u.race() != Race::Terran || !u.is_structure() {
					if let Some(data) = self.game_data.units.get(&u.type_id) {
						if let Some(ability) = data.ability {
							*self.orders.entry(ability).or_default() += 1;
						}
					}
				}
			} else {
				*self.current_units.entry(u.type_id).or_default() += 1;
			}
		});
	}
	fn group_units(&mut self) {
		let mut owned = Units::new();
		let mut units = Units::new();
		let mut structures = Units::new();
		let mut townhalls = Units::new();
		let mut workers = Units::new();
		let mut enemies = Units::new();
		let mut enemy_units = Units::new();
		let mut enemy_structures = Units::new();
		let mut enemy_townhalls = Units::new();
		let mut enemy_workers = Units::new();
		let mut mineral_fields = Units::new();
		let mut vespene_geysers = Units::new();
		let mut resources = Units::new();
		let mut destructables = Units::new();
		let mut watchtowers = Units::new();
		let mut inhibitor_zones = Units::new();
		let mut gas_buildings = Units::new();
		let mut larvas = Units::new();
		let mut placeholders = Units::new();
		let mut techlab_tags = Vec::new();
		let mut reactor_tags = Vec::new();
		let mut max_cooldowns = self.max_cooldowns.borrow_mut();

		self.state.observation.raw.units.iter().for_each(|u| {
			let u_type = u.type_id;
			match u.alliance {
				Alliance::Neutral => match u_type {
					UnitTypeId::XelNagaTower => {
						watchtowers.push(u.clone());
					}
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
						resources.push(u.clone());
						mineral_fields.push(u.clone());
					}
					UnitTypeId::VespeneGeyser
					| UnitTypeId::SpacePlatformGeyser
					| UnitTypeId::RichVespeneGeyser
					| UnitTypeId::ProtossVespeneGeyser
					| UnitTypeId::PurifierVespeneGeyser
					| UnitTypeId::ShakurasVespeneGeyser => {
						resources.push(u.clone());
						vespene_geysers.push(u.clone());
					}
					UnitTypeId::InhibitorZoneSmall
					| UnitTypeId::InhibitorZoneMedium
					| UnitTypeId::InhibitorZoneLarge => {
						inhibitor_zones.push(u.clone());
					}
					_ => {
						destructables.push(u.clone());
					}
				},
				Alliance::Own => {
					owned.push(u.clone());
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
					if u.is_structure() {
						if u.is_placeholder() {
							placeholders.push(u.clone());
						} else {
							structures.push(u.clone());
							match u_type {
								UnitTypeId::CommandCenter
								| UnitTypeId::OrbitalCommand
								| UnitTypeId::PlanetaryFortress
								| UnitTypeId::CommandCenterFlying
								| UnitTypeId::OrbitalCommandFlying
								| UnitTypeId::Hatchery
								| UnitTypeId::Lair
								| UnitTypeId::Hive
								| UnitTypeId::Nexus => {
									townhalls.push(u.clone());
								}
								UnitTypeId::Refinery
								| UnitTypeId::RefineryRich
								| UnitTypeId::Assimilator
								| UnitTypeId::AssimilatorRich
								| UnitTypeId::Extractor
								| UnitTypeId::ExtractorRich => {
									gas_buildings.push(u.clone());
								}
								UnitTypeId::TechLab
								| UnitTypeId::BarracksTechLab
								| UnitTypeId::FactoryTechLab
								| UnitTypeId::StarportTechLab => techlab_tags.push(u.tag),

								UnitTypeId::Reactor
								| UnitTypeId::BarracksReactor
								| UnitTypeId::FactoryReactor
								| UnitTypeId::StarportReactor => reactor_tags.push(u.tag),
								_ => {}
							}
						}
					} else {
						units.push(u.clone());
						match u_type {
							UnitTypeId::SCV | UnitTypeId::Probe | UnitTypeId::Drone => {
								workers.push(u.clone());
							}
							UnitTypeId::Larva => {
								larvas.push(u.clone());
							}
							_ => {}
						}
					}
				}
				Alliance::Enemy => {
					enemies.push(u.clone());
					if u.is_structure() {
						enemy_structures.push(u.clone());
						if [
							UnitTypeId::CommandCenter,
							UnitTypeId::OrbitalCommand,
							UnitTypeId::PlanetaryFortress,
							UnitTypeId::CommandCenterFlying,
							UnitTypeId::OrbitalCommandFlying,
							UnitTypeId::Hatchery,
							UnitTypeId::Lair,
							UnitTypeId::Hive,
							UnitTypeId::Nexus,
						]
						.contains(&u_type)
						{
							enemy_townhalls.push(u.clone());
						}
					} else {
						enemy_units.push(u.clone());
						if [UnitTypeId::SCV, UnitTypeId::Probe, UnitTypeId::Drone].contains(&u_type) {
							enemy_workers.push(u.clone());
						}
					}
				}
				_ => {}
			}
		});

		self.grouped_units = GroupedUnits {
			owned,
			units,
			structures,
			townhalls,
			workers,
			enemies,
			enemy_units,
			enemy_structures,
			enemy_townhalls,
			enemy_workers,
			mineral_fields,
			vespene_geysers,
			resources,
			destructables,
			watchtowers,
			inhibitor_zones,
			gas_buildings,
			larvas,
			placeholders,
		};
		self.techlab_tags = Rc::new(techlab_tags);
		self.reactor_tags = Rc::new(reactor_tags);

		self.data_for_unit = Rc::new(DataForUnit {
			commander: Rc::clone(&self.commander),
			game_data: Rc::clone(&self.game_data),
			techlab_tags: Rc::clone(&self.techlab_tags),
			reactor_tags: Rc::clone(&self.reactor_tags),
			race_values: Rc::clone(&self.race_values),
			max_cooldowns: Rc::clone(&self.max_cooldowns),
		});
	}
	pub fn get_unit_api_cost(&self, unit: UnitTypeId) -> Cost {
		self.game_data
			.units
			.get(&unit)
			.map_or_else(Default::default, |data| data.cost())
	}
	pub fn get_unit_cost(&self, unit: UnitTypeId) -> Cost {
		self.game_data
			.units
			.get(&unit)
			.map_or_else(Default::default, |data| {
				let mut cost = data.cost();
				match unit {
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
			})
	}
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
	pub fn get_upgrade_cost(&self, upgrade: UpgradeId) -> Cost {
		self.game_data
			.upgrades
			.get(&upgrade)
			.map_or_else(Default::default, |data| data.cost())
	}
	pub fn can_afford_upgrade(&self, upgrade: UpgradeId) -> bool {
		let cost = self.get_upgrade_cost(upgrade);
		self.minerals >= cost.minerals && self.vespene >= cost.vespene
	}
	/*
	fn can_afford_ability(&self, ability: AbilityId) -> bool {
		unimplemented!()
	}
	*/
	pub fn can_place(&self, ws: &mut WS, building: UnitTypeId, pos: Point2) -> bool {
		self.query
			.placement(
				ws,
				vec![(self.game_data.units[&building].ability.unwrap(), pos, None)],
				false,
			)
			.unwrap()[0] == ActionResult::Success
	}
	#[allow(clippy::too_many_arguments)]
	pub fn find_placement(
		&self,
		ws: &mut WS,
		building: UnitTypeId,
		near: Point2,
		max_distance: isize,
		placement_step: isize,
		random: bool,
		addon: bool,
	) -> Option<Point2> {
		if let Some(data) = self.game_data.units.get(&building) {
			if let Some(ability) = data.ability {
				if self
					.query
					.placement(
						ws,
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
					.unwrap()[0] == ActionResult::Success
				{
					return Some(near);
				}

				for distance in (placement_step..max_distance).step_by(placement_step as usize) {
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
						.query
						.placement(
							ws,
							positions.iter().map(|pos| (ability, *pos, None)).collect(),
							false,
						)
						.unwrap();

					let mut valid_positions = positions
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

					if addon {
						let results = self
							.query
							.placement(
								ws,
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
						return if random {
							let mut rng = thread_rng();
							valid_positions.choose(&mut rng).copied()
						} else {
							let f = |pos: Point2| near.distance_squared(pos);
							valid_positions
								.iter()
								.min_by(|pos1, pos2| f(**pos1).partial_cmp(&f(**pos2)).unwrap())
								.copied()
						};
					}
				}
			}
		}
		None
	}
	pub fn find_gas_placement(&self, ws: &mut WS, base: Point2) -> Option<Unit> {
		let ability = self.game_data.units[&self.race_values.gas_building]
			.ability
			.unwrap();

		let geysers = self.grouped_units.vespene_geysers.closer_pos(11.0, base);
		let results = self
			.query
			.placement(
				ws,
				geysers.iter().map(|u| (ability, u.position, None)).collect(),
				false,
			)
			.unwrap();

		let valid_geysers = geysers
			.iter()
			.zip(results.iter())
			.filter_map(|(geyser, res)| {
				if *res == ActionResult::Success {
					Some(geyser)
				} else {
					None
				}
			})
			.collect::<Vec<&Unit>>();

		if valid_geysers.is_empty() {
			None
		} else {
			Some(valid_geysers[0].clone())
		}
	}
	pub fn get_expansion(&self, ws: &mut WS) -> Option<(Point2, Point2)> {
		let expansions = self
			.expansions
			.iter()
			.filter(|(loc, _)| {
				self.grouped_units
					.townhalls
					.iter()
					.all(|t| t.is_further_pos(15.0, *loc))
			})
			.copied()
			.collect::<Vec<(Point2, Point2)>>();
		let paths = self
			.query
			.pathing(
				ws,
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
	pub fn get_enemy_expansion(&self, ws: &mut WS) -> Option<(Point2, Point2)> {
		let expansions = self
			.expansions
			.iter()
			.filter(|(loc, _)| {
				self.grouped_units
					.enemy_townhalls
					.iter()
					.all(|t| t.is_further_pos(15.0, *loc))
			})
			.copied()
			.collect::<Vec<(Point2, Point2)>>();
		let paths = self
			.query
			.pathing(
				ws,
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
	pub fn owned_expansions(&self) -> Vec<(Point2, Point2)> {
		self.expansions
			.iter()
			.filter(|(loc, _)| {
				self.grouped_units
					.townhalls
					.iter()
					.any(|t| t.is_closer_pos(15.0, *loc))
			})
			.copied()
			.collect()
	}
	pub fn enemy_expansions(&self) -> Vec<(Point2, Point2)> {
		self.expansions
			.iter()
			.filter(|(loc, _)| {
				self.grouped_units
					.enemy_townhalls
					.iter()
					.any(|t| t.is_closer_pos(15.0, *loc))
			})
			.copied()
			.collect()
	}
}

impl Default for Bot {
	fn default() -> Self {
		Self::new()
	}
}
