#[macro_use]
extern crate clap;

use rand::prelude::{thread_rng, SliceRandom};
use rust_sc2::{
	bot, bot_impl_player, bot_new,
	geometry::Point2,
	player::{
		Difficulty,
		Players::{Computer, Human},
	},
	run_game, run_ladder_game, Player, PlayerSettings,
};
use std::cmp::Ordering;

#[bot]
struct ReaperRushAI {
	reapers_retreat: HashSet<u64>,
	last_loop_distributed: u32,
}

impl ReaperRushAI {
	const DISTRIBUTION_DELAY: u32 = 16;

	#[bot_new]
	fn new(game_step: u32) -> Self {
		Self {
			game_step,
			reapers_retreat: HashSet::new(),
			last_loop_distributed: 0,
		}
	}
	fn distribute_workers(&mut self) {
		let workers = &self.grouped_units.workers;
		if workers.is_empty() {
			return;
		}
		let mut idle_workers = workers.idle();

		// Check distribution delay if there aren't any idle workers
		let game_loop = self.state.observation.game_loop;
		let last_loop = &mut self.last_loop_distributed;
		if idle_workers.is_empty() && *last_loop + Self::DISTRIBUTION_DELAY > game_loop {
			return;
		}
		*last_loop = game_loop;

		// Distribute
		let mineral_fields = &self.grouped_units.mineral_fields;
		if mineral_fields.is_empty() {
			return;
		}
		let bases = self.grouped_units.townhalls.ready();
		if bases.is_empty() {
			return;
		}
		let gas_buildings = self
			.grouped_units
			.gas_buildings
			.filter(|g| g.is_ready() && g.vespene_contents.map_or(false, |vespene| vespene > 0));

		let mut deficit_minings = Units::new();
		let mut deficit_geysers = Units::new();

		bases.iter().for_each(
			|base| match base.assigned_harvesters.cmp(&base.ideal_harvesters) {
				Ordering::Equal => {}
				Ordering::Greater => {
					let local_minerals = self
						.grouped_units
						.mineral_fields
						.closer(11.0, base)
						.iter()
						.map(|m| m.tag)
						.collect::<Vec<u64>>();

					idle_workers.extend(
						workers
							.filter(|u| {
								u.target_tag().map_or(false, |target_tag| {
									local_minerals.contains(&target_tag)
										|| (u.is_carrying_minerals() && target_tag == base.tag)
								})
							})
							.iter()
							.take(
								(base.assigned_harvesters.unwrap() - base.ideal_harvesters.unwrap()) as usize,
							)
							.cloned(),
					);
				}
				Ordering::Less => (0..(base.ideal_harvesters.unwrap() - base.assigned_harvesters.unwrap()))
					.for_each(|_| {
						deficit_minings.push(base.clone());
					}),
			},
		);
		gas_buildings
			.iter()
			.for_each(|gas| match gas.assigned_harvesters.cmp(&gas.ideal_harvesters) {
				Ordering::Equal => {}
				Ordering::Greater => {
					idle_workers.extend(
						workers
							.filter(|u| {
								u.target_tag().map_or(false, |target_tag| {
									target_tag == gas.tag
										|| (u.is_carrying_vespene() && target_tag == bases.closest(gas).tag)
								})
							})
							.iter()
							.take((gas.assigned_harvesters.unwrap() - gas.ideal_harvesters.unwrap()) as usize)
							.cloned(),
					);
				}
				Ordering::Less => (0..(gas.ideal_harvesters.unwrap() - gas.assigned_harvesters.unwrap()))
					.for_each(|_| {
						deficit_geysers.push(gas.clone());
					}),
			});

		let minerals_near_base = if idle_workers.len() > deficit_minings.len() + deficit_geysers.len() {
			let minerals =
				mineral_fields.filter(|m| bases.iter().any(|base| base.distance_squared(m) < 121.0));
			if minerals.is_empty() {
				None
			} else {
				Some(minerals)
			}
		} else {
			None
		};

		let mineral_fields = mineral_fields.clone();
		idle_workers.iter().for_each(|u| {
			if !deficit_geysers.is_empty() {
				let closest = deficit_geysers.closest(u).tag;
				deficit_geysers.remove(closest);
				self.command(u.gather(closest, false));
			} else if !deficit_minings.is_empty() {
				let closest = deficit_minings.closest(u);
				deficit_minings.remove(closest.tag);
				self.command(
					u.gather(
						mineral_fields
							.closer(11.0, &closest)
							.max(|m| m.mineral_contents.unwrap_or(0))
							.tag,
						false,
					),
				);
			} else if u.is_idle() {
				if let Some(minerals) = &minerals_near_base {
					self.command(u.gather(minerals.closest(u).tag, false));
				}
			}
		});
	}

	fn get_builder(&self, pos: Point2, mineral_tags: &[u64]) -> Option<Unit> {
		let workers = self.grouped_units.workers.filter(|u| {
			!u.is_constructing()
				&& (!u.is_gathering() || u.target_tag().map_or(false, |tag| mineral_tags.contains(&tag)))
				&& !u.is_returning()
				&& !u.is_carrying_resource()
		});
		if workers.is_empty() {
			None
		} else {
			Some(workers.closest_pos(pos))
		}
	}
	fn build(&mut self, ws: &mut WS) {
		let mineral_tags = self
			.grouped_units
			.mineral_fields
			.iter()
			.map(|u| u.tag)
			.collect::<Vec<u64>>();
		let main_base = self.start_location.towards(self.game_info.map_center, 8.0);

		if self.current_units.get(&UnitTypeId::Refinery).unwrap_or(&0) < &2
			&& self.orders.get(&AbilityId::TerranBuildRefinery).unwrap_or(&0) == &0
			&& self.can_afford(UnitTypeId::Refinery, false)
		{
			if let Some(geyser) = self.find_gas_placement(ws, self.start_location) {
				if let Some(builder) = self.get_builder(geyser.position, &mineral_tags) {
					self.command(builder.build_gas(geyser.tag, false));
					self.substract_resources(UnitTypeId::Refinery);
				}
			}
		}

		if self.supply_left < 3
			&& self.supply_cap < 200
			&& self.orders.get(&AbilityId::TerranBuildSupplyDepot).unwrap_or(&0) == &0
			&& self.can_afford(UnitTypeId::SupplyDepot, false)
		{
			if let Some(location) =
				self.find_placement(ws, UnitTypeId::SupplyDepot, main_base, 15, 2, false, false)
			{
				if let Some(builder) = self.get_builder(location, &mineral_tags) {
					self.command(builder.build(UnitTypeId::SupplyDepot, location, false));
					self.substract_resources(UnitTypeId::SupplyDepot);
					return;
				}
			}
		}

		if self.current_units.get(&UnitTypeId::Barracks).unwrap_or(&0)
			+ self.orders.get(&AbilityId::TerranBuildBarracks).unwrap_or(&0)
			< 4 && self.can_afford(UnitTypeId::Barracks, false)
		{
			if let Some(location) =
				self.find_placement(ws, UnitTypeId::Barracks, main_base, 15, 4, false, false)
			{
				if let Some(builder) = self.get_builder(location, &mineral_tags) {
					self.command(builder.build(UnitTypeId::Barracks, location, false));
					self.substract_resources(UnitTypeId::Barracks);
				}
			}
		}
	}

	fn train(&mut self) {
		// Maximum 20 workers instead of 22 beacause API can't see 2 workers inside refineries, so bot trains 2 extra workers.
		if self.current_units.get(&UnitTypeId::SCV).unwrap_or(&0)
			+ self.orders.get(&AbilityId::CommandCenterTrainSCV).unwrap_or(&0)
			< 20 && self.can_afford(UnitTypeId::SCV, true)
		{
			let townhalls = &self.grouped_units.townhalls;
			if !townhalls.is_empty() {
				let ccs = townhalls.filter(|u| u.is_ready() && u.is_almost_idle());
				if !ccs.is_empty() {
					self.command(ccs[0].train(UnitTypeId::SCV, false));
					self.substract_resources(UnitTypeId::SCV);
				}
			}
		}

		if self.can_afford(UnitTypeId::Reaper, true) {
			let structures = &self.grouped_units.structures;
			if !structures.is_empty() {
				let barracks = structures
					.filter(|u| u.type_id == UnitTypeId::Barracks && u.is_ready() && u.is_almost_idle());
				if !barracks.is_empty() {
					self.command(barracks[0].train(UnitTypeId::Reaper, false));
					self.substract_resources(UnitTypeId::Reaper);
				}
			}
		}
	}

	fn is_pathable(&self, pos: Point2) -> bool {
		self.game_info.pathing_grid[pos].is_empty()
	}
	fn throw_mine(&mut self, reaper: &Unit, target: &Unit) -> bool {
		if let Some(abilities) = self.abilities_units.get(&reaper.tag) {
			if abilities.contains(&AbilityId::KD8ChargeKD8Charge)
				&& reaper.distance_squared(target)
					<= (reaper.radius
						+ target.radius + self.game_data.abilities[&AbilityId::KD8ChargeKD8Charge]
						.cast_range
						.unwrap())
					.powi(2)
			{
				self.command(reaper.command(
					AbilityId::KD8ChargeKD8Charge,
					Target::Pos(target.position),
					false,
				));
				return true;
			}
		}
		false
	}
	fn execute_micro(&mut self) {
		// Lower ready depots
		self.grouped_units
			.structures
			.filter(|s| s.type_id == UnitTypeId::SupplyDepot && s.is_ready())
			.iter()
			.for_each(|s| self.command(s.use_ability(AbilityId::MorphSupplyDepotLower, false)));

		// Reapers micro
		let reapers = self.grouped_units.units.of_type(UnitTypeId::Reaper);
		if reapers.is_empty() {
			return;
		}
		let targets = {
			let attackers = self
				.grouped_units
				.enemies
				.filter(|u| !u.is_flying && u.can_attack_ground());
			if attackers.is_empty() {
				let ground = self.grouped_units.enemies.ground();
				if ground.is_empty() {
					None
				} else {
					Some(ground)
				}
			} else {
				Some(attackers)
			}
		};

		reapers.iter().for_each(|u| {
			let is_retreating = self.reapers_retreat.contains(&u.tag);
			if is_retreating {
				// health > 75%
				if u.health.unwrap() > u.health_max.unwrap() * 0.75 {
					self.reapers_retreat.remove(&u.tag);
				}
			// health < 50%
			} else if u.health.unwrap() * 2.0 < u.health_max.unwrap() {
				self.reapers_retreat.insert(u.tag);
			}

			match &targets {
				Some(targets) => {
					if !self.throw_mine(u, &targets.closest(u)) {
						if is_retreating || u.on_cooldown() {
							let close_enemies = targets.in_range(u, {
								if is_retreating {
									2.0
								} else {
									0.5
								}
							});
							if !close_enemies.is_empty() {
								let retreat_position = {
									let pos =
										u.position.towards(close_enemies.closest(u).position, -u.speed());
									if self.is_pathable(pos) {
										pos
									} else {
										self.start_location
									}
								};
								self.command(u.move_to(Target::Pos(retreat_position), false));
							} else {
								let closest = targets.closest(u);
								if !u.in_range(&closest, 0.0) {
									self.command(u.move_to(
										Target::Pos(if is_retreating {
											u.position
										} else {
											closest.position
										}),
										false,
									));
								}
							}
						} else {
							let close_targets = targets.in_range_of(u, 0.0);
							if !close_targets.is_empty() {
								self.command(
									u.attack(Target::Tag(close_targets.partial_min(|t| t.hits()).tag), false),
								);
							} else {
								self.command(u.move_to(Target::Pos(targets.closest(u).position), false));
							}
						}
					}
				}
				None => {
					self.command(u.move_to(
						Target::Pos(if is_retreating {
							u.position
						} else {
							self.enemy_start
						}),
						false,
					));
				}
			}
		});
	}
}

#[bot_impl_player]
impl Player for ReaperRushAI {
	fn on_start(&mut self, _ws: &mut WS) {
		let townhall = self.grouped_units.townhalls[0].clone();
		self.command(townhall.smart(Target::Pos(self.start_resource_center), false));
		self.command(townhall.train(UnitTypeId::SCV, false));
		self.substract_resources(UnitTypeId::SCV);

		let minerals_near_base = self.grouped_units.mineral_fields.closer(11.0, &townhall);
		self.grouped_units.workers.clone().iter().for_each(|u| {
			self.command(u.gather(minerals_near_base.closest(&u).tag, false));
		});
	}

	fn on_step(&mut self, ws: &mut WS, _iteration: usize) {
		self.distribute_workers();
		self.build(ws);
		self.train();
		self.execute_micro();
	}

	fn get_player_settings(&self) -> PlayerSettings {
		PlayerSettings::new(Race::Terran, Some("RustyReapers".to_string()))
	}
}

fn main() {
	let app = clap_app!(RustyReapers =>
		(version: crate_version!())
		(author: crate_authors!())
		(@arg ladder_server: --LadderServer +takes_value)
		(@arg opponent_id: --OpponentId +takes_value)
		(@arg host_port: --GamePort +takes_value)
		(@arg player_port: --StartPort +takes_value)
		(@arg game_step: -s --step
			+takes_value
			default_value("2")
			"Sets game step for bot"
		)
		(@subcommand local =>
			(about: "Runs local game vs Computer")
			(@arg map: -m --map
				+takes_value
			)
			(@arg race: --race
				+takes_value
				"Sets opponent race"
			)
			(@arg difficulty: -d --difficulty
				+takes_value
				"Sets opponent diffuculty"
			)
			(@arg ai_build: --("ai-build")
				+takes_value
				"Sets opponent build"
			)
			(@arg realtime: --realtime "Enables realtime mode")
		)
		(@subcommand human =>
			(about: "Runs game Human vs Bot")
			(@arg map: -m --map
				+takes_value
			)
			(@arg race: --race *
				+takes_value
				"Sets human race"
			)
			(@arg name: --name
				+takes_value
				"Sets human name"
			)
			(@arg realtime: --realtime "Enables realtime mode")
		)
	)
	.get_matches();

	let game_step = match app.value_of("game_step") {
		Some("0") => panic!("game_step must be X >= 1"),
		Some(step) => step.parse::<u32>().expect("Can't parse game_step"),
		None => unreachable!(),
	};

	let bot = Box::new(ReaperRushAI::new(game_step));

	if app.is_present("ladder_server") {
		run_ladder_game(
			bot,
			app.value_of("ladder_server").unwrap_or("127.0.0.1").to_string(),
			app.value_of("host_port")
				.expect("GamePort must be specified")
				.to_string(),
			app.value_of("player_port")
				.expect("StartPort must be specified")
				.parse()
				.expect("Can't parse StartPort"),
			app.value_of("opponent_id"),
		)
		.unwrap();
	} else {
		let mut rng = thread_rng();

		let map;
		let realtime;
		let players: Vec<Box<dyn Player>>;

		match app.subcommand() {
			("local", Some(sub)) => {
				map = match sub.value_of("map") {
					Some(map) => Some(map.to_string()),
					None => None,
				};
				realtime = sub.is_present("realtime");
				players = vec![
					bot,
					Box::new(Computer(
						match sub.value_of("race") {
							Some(race) => race.parse().expect("Can't parse computer race"),
							None => Race::Random,
						},
						match sub.value_of("difficulty") {
							Some(difficulty) => difficulty.parse().expect("Can't parse computer difficulty"),
							None => Difficulty::VeryEasy,
						},
						match sub.value_of("ai_build") {
							Some(ai_build) => Some(ai_build.parse().expect("Can't parse computer build")),
							None => None,
						},
					)),
				];
			}
			("human", Some(sub)) => {
				map = match sub.value_of("map") {
					Some(map) => Some(map.to_string()),
					None => None,
				};
				realtime = sub.is_present("realtime");
				players = vec![
					Box::new(Human(
						match sub.value_of("race") {
							Some(race) => race.parse().expect("Can't parse human race"),
							None => unreachable!("Human race must be set"),
						},
						match sub.value_of("name") {
							Some(name) => Some(name.to_string()),
							None => None,
						},
					)),
					bot,
				];
			}
			_ => {
				println!("Game mode is not specified! Use -h, --help to print help information.");
				std::process::exit(0);
			}
		}

		// Maps:
		// - Ladder_2019_Season_3:
		//   - AcropolisLE, DiscoBloodbathLE, EphemeronLE, ThunderbirdLE, TritonLE, WintersGateLE, WorldofSleepersLE
		// - Melee: Empty128, Flat32, Flat48, Flat64, Flat96, Flat128, Simple64, Simple96, Simple128.

		run_game(
			map.unwrap_or_else(|| {
				(*[
					"AcropolisLE",
					"DiscoBloodbathLE",
					"EphemeronLE",
					"ThunderbirdLE",
					"TritonLE",
					"WintersGateLE",
					"WorldofSleepersLE",
				]
				.choose(&mut rng)
				.unwrap())
				.to_string()
			}),
			players,
			realtime,
			None,
		)
		.unwrap();
	}
}
