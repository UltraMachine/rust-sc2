#[macro_use]
extern crate clap;

use rand::prelude::{thread_rng, SliceRandom};
use rust_sc2::{
	bot, bot_impl_player, bot_new,
	geometry::Point2,
	ids::BuffId,
	player::{
		Difficulty,
		Players::{Computer, Human},
	},
	run_game, run_ladder_game, Player, PlayerSettings,
};
use std::cmp::{min, Ordering};

#[bot]
struct ZergRushAI {
	last_loop_distributed: u32,
}

impl ZergRushAI {
	const DISTRIBUTION_DELAY: u32 = 16;

	#[bot_new]
	fn new(game_step: u32) -> Self {
		Self {
			game_step,
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
		let gas_buildings = self.grouped_units.gas_buildings.ready();

		let mut deficit_minings = Units::new();
		let mut deficit_geysers = Units::new();

		let mineral_tags = mineral_fields.iter().map(|m| m.tag).collect::<Vec<u64>>();

		let speed_upgrade = UpgradeId::Zerglingmovementspeed;
		let has_enough_gas = self.can_afford_upgrade(speed_upgrade)
			|| self.has_upgrade(speed_upgrade)
			|| self
				.orders
				.get(&self.game_data.upgrades[&speed_upgrade].ability)
				.unwrap_or(&0) == &1;

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

		if has_enough_gas {
			gas_buildings.iter().for_each(|gas| {
				if let Ordering::Greater = gas.assigned_harvesters.cmp(&Some(0)) {
					idle_workers.extend(
						workers
							.filter(|u| {
								u.target_tag().map_or(false, |target_tag| {
									target_tag == gas.tag
										|| (u.is_carrying_vespene() && target_tag == bases.closest(gas).tag)
								})
							})
							.iter()
							.cloned(),
					);
				}
			});
		} else {
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
											|| (u.is_carrying_vespene()
												&& target_tag == bases.closest(gas).tag)
									})
								})
								.iter()
								.take(
									(gas.assigned_harvesters.unwrap() - gas.ideal_harvesters.unwrap())
										as usize,
								)
								.cloned(),
						);
					}
					Ordering::Less => {
						idle_workers.extend(
							workers
								.filter(|u| {
									u.target_tag()
										.map_or(false, |target_tag| mineral_tags.contains(&target_tag))
								})
								.iter()
								.cloned(),
						);
						(0..(gas.ideal_harvesters.unwrap() - gas.assigned_harvesters.unwrap())).for_each(
							|_| {
								deficit_geysers.push(gas.clone());
							},
						);
					}
				});
		}

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
				let closest = deficit_minings.closest(u).clone();
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

	fn order_units(&mut self) {
		if self.grouped_units.larvas.is_empty() {
			return;
		}

		let over = UnitTypeId::Overlord;
		if self.supply_left < 3
			&& self.supply_cap < 200
			&& self
				.orders
				.get(&self.game_data.units[&over].ability.unwrap())
				.unwrap_or(&0) == &0
			&& self.can_afford(over, false)
		{
			if let Some(larva) = self.grouped_units.larvas.pop() {
				self.command(larva.train(over, false));
				self.substract_resources(over);
			}
		}

		let drone = UnitTypeId::Drone;
		if self.current_units.get(&drone).unwrap_or(&0)
			+ self
				.orders
				.get(&self.game_data.units[&drone].ability.unwrap())
				.unwrap_or(&0)
			< min(
				96,
				self.current_units
					.get(&UnitTypeId::Hatchery)
					.map_or(0, |n| n * 16),
			) && self.can_afford(drone, true)
		{
			if let Some(larva) = self.grouped_units.larvas.pop() {
				self.command(larva.train(drone, false));
				self.substract_resources(drone);
			}
		}

		let queen = UnitTypeId::Queen;
		if self.current_units.get(&queen).unwrap_or(&0)
			+ self
				.orders
				.get(&self.game_data.units[&queen].ability.unwrap())
				.unwrap_or(&0)
			< self.grouped_units.townhalls.len()
			&& self.can_afford(queen, true)
		{
			let townhalls = self.grouped_units.townhalls.clone();
			if !townhalls.is_empty() {
				self.command(townhalls.first().train(queen, false));
				self.substract_resources(queen);
			}
		}

		let zergling = UnitTypeId::Zergling;
		if self.can_afford(zergling, true) {
			if let Some(larva) = self.grouped_units.larvas.pop() {
				self.command(larva.train(zergling, false));
				self.substract_resources(zergling);
			}
		}
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
			Some(workers.closest_pos(pos).clone())
		}
	}
	fn build(&mut self, ws: &mut WS) {
		let mineral_tags = self
			.grouped_units
			.mineral_fields
			.iter()
			.map(|u| u.tag)
			.collect::<Vec<u64>>();

		let pool = UnitTypeId::SpawningPool;
		if self.current_units.get(&pool).unwrap_or(&0)
			+ self
				.orders
				.get(&self.game_data.units[&pool].ability.unwrap())
				.unwrap_or(&0)
			== 0 && self.can_afford(pool, false)
		{
			if let Some(location) = self.find_placement(
				ws,
				pool,
				self.start_location.towards(self.game_info.map_center, 6.0),
				15,
				1,
				false,
				false,
			) {
				if let Some(builder) = self.get_builder(location, &mineral_tags) {
					self.command(builder.build(pool, location, false));
					self.substract_resources(pool);
				}
			}
		}

		let extractor = UnitTypeId::Extractor;
		if self.current_units.get(&extractor).unwrap_or(&0)
			+ self
				.orders
				.get(&self.game_data.units[&extractor].ability.unwrap())
				.unwrap_or(&0)
			== 0 && self.can_afford(extractor, false)
		{
			if let Some(geyser) = self.find_gas_placement(ws, self.start_location) {
				if let Some(builder) = self.get_builder(geyser.position, &mineral_tags) {
					self.command(builder.build_gas(geyser.tag, false));
					self.substract_resources(extractor);
				}
			}
		}

		let hatchery = UnitTypeId::Hatchery;
		if self.can_afford(hatchery, false) {
			if let Some((location, _resource_center)) = self.get_expansion(ws) {
				if let Some(builder) = self.get_builder(location, &mineral_tags) {
					self.command(builder.build(hatchery, location, false));
					self.substract_resources(hatchery);
				}
			}
		}
	}

	fn upgrades(&mut self) {
		let speed_upgrade = UpgradeId::Zerglingmovementspeed;
		if !self.has_upgrade(speed_upgrade)
			&& self
				.orders
				.get(&self.game_data.upgrades[&speed_upgrade].ability)
				.unwrap_or(&0) == &0
			&& self.can_afford_upgrade(speed_upgrade)
		{
			let pool = self.grouped_units.structures.of_type(UnitTypeId::SpawningPool);
			if !pool.is_empty() {
				self.command(pool.first().research(speed_upgrade, false));
				self.substract_upgrade_cost(speed_upgrade);
			}
		}
	}

	fn execute_micro(&mut self) {
		// Injecting Larva
		let hatcheries = self.grouped_units.townhalls.clone();
		if !hatcheries.is_empty() {
			let not_injected = hatcheries.filter(|h| {
				!h.has_buff(BuffId::QueenSpawnLarvaTimer)
					|| h.buff_duration_remain.unwrap() * 20 > h.buff_duration_max.unwrap()
			});
			if !not_injected.is_empty() {
				let mut queens = self.grouped_units.units.filter(|u| {
					u.type_id == UnitTypeId::Queen
						&& !u.is_using(AbilityId::EffectInjectLarva)
						&& self.abilities_units.get(&u.tag).map_or(false, |abilities| {
							abilities.contains(&AbilityId::EffectInjectLarva)
						})
				});
				hatcheries.iter().for_each(|h| {
					if !queens.is_empty() {
						let queen = queens.closest(h).clone();
						queens.remove(queen.tag);
						self.command(queen.command(AbilityId::EffectInjectLarva, Target::Tag(h.tag), false));
					}
				});
			}
		}

		let zerglings = self.grouped_units.units.of_type(UnitTypeId::Zergling);
		if zerglings.is_empty() {
			return;
		}

		// Check if speed upgrade is >90% ready
		let speed_upgrade = UpgradeId::Zerglingmovementspeed;

		let speed_upgrade_is_almost_ready = self.has_upgrade(speed_upgrade)
			|| self.grouped_units.structures.iter().any(|s| {
				s.type_id == UnitTypeId::SpawningPool && !s.is_idle() && {
					let order = &s.orders[0];
					order.ability == self.game_data.upgrades[&speed_upgrade].ability
						&& (order.progress - 0.9).abs() < std::f32::EPSILON
				}
			});

		// Attacking with zerglings or defending our locations
		let targets = {
			let enemies = if speed_upgrade_is_almost_ready {
				self.grouped_units.enemies.clone()
			} else {
				self.grouped_units
					.enemies
					.filter(|e| hatcheries.iter().any(|h| h.distance_squared(e) < 625.0))
			};
			if enemies.is_empty() {
				None
			} else {
				let attackers = enemies.filter(|u| !u.is_flying && u.can_attack_ground());
				if attackers.is_empty() {
					let ground = enemies.ground();
					if ground.is_empty() {
						None
					} else {
						Some(ground)
					}
				} else {
					Some(attackers)
				}
			}
		};
		match targets {
			Some(targets) => zerglings.iter().for_each(|u| {
				let target = {
					let close_targets = targets.in_range_of(u, 0.0);
					if !close_targets.is_empty() {
						close_targets.partial_min(|t| t.hits()).tag
					} else {
						targets.closest(u).tag
					}
				};
				self.command(u.attack(Target::Tag(target), false));
			}),
			None => {
				let target = if speed_upgrade_is_almost_ready {
					self.enemy_start
				} else {
					self.start_location.towards(self.start_resource_center, -8.0)
				};
				zerglings.iter().for_each(|u| {
					self.command(u.move_to(Target::Pos(target), false));
				})
			}
		}
	}
}

#[bot_impl_player]
impl Player for ZergRushAI {
	fn on_start(&mut self, _ws: &mut WS) {
		let townhall = self.grouped_units.townhalls.first().clone();

		self.command(townhall.command(
			AbilityId::RallyWorkers,
			Target::Pos(self.start_resource_center),
			false,
		));
		self.command(self.grouped_units.larvas.first().train(UnitTypeId::Drone, false));
		self.substract_resources(UnitTypeId::Drone);

		let minerals_near_base = self.grouped_units.mineral_fields.closer(11.0, &townhall);
		self.grouped_units.workers.clone().iter().for_each(|u| {
			self.command(u.gather(minerals_near_base.closest(&u).tag, false));
		});
	}

	fn on_step(&mut self, ws: &mut WS, _iteration: usize) {
		self.distribute_workers();
		self.upgrades();
		self.build(ws);
		self.order_units();
		self.execute_micro();
	}

	fn get_player_settings(&self) -> PlayerSettings {
		PlayerSettings::new(Race::Zerg, Some("RustyLings".to_string()))
	}
}

fn main() {
	let app = clap_app!(RustyLings =>
		(version: crate_version!())
		(author: crate_authors!())
		(@arg ladder_server: --LadderServer +takes_value)
		(@arg opponent_id: --OpponentId +takes_value)
		(@arg host_port: --GamePort +takes_value)
		(@arg player_port: --StartPort +takes_value)
		(@arg game_step: -s --step
			+takes_value
			default_value("1")
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

	let bot = Box::new(ZergRushAI::new(game_step));

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
