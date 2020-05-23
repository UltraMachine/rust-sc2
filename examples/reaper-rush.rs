#[macro_use]
extern crate clap;

use rand::prelude::{thread_rng, SliceRandom};
use rust_sc2::prelude::*;
use std::{cmp::Ordering, collections::HashSet};

#[bot]
#[derive(Default)]
struct ReaperRushAI {
	reapers_retreat: HashSet<u64>,
	last_loop_distributed: u32,
}

impl ReaperRushAI {
	const DISTRIBUTION_DELAY: u32 = 8;

	fn new() -> Self {
		Default::default()
	}
	fn distribute_workers(&mut self) {
		if self.units.my.workers.is_empty() {
			return;
		}
		let mut idle_workers = self.units.my.workers.idle();

		// Check distribution delay if there aren't any idle workers
		let game_loop = self.state.observation.game_loop;
		let last_loop = &mut self.last_loop_distributed;
		if idle_workers.is_empty() && *last_loop + Self::DISTRIBUTION_DELAY > game_loop {
			return;
		}
		*last_loop = game_loop;

		// Distribute
		let mineral_fields = &self.units.mineral_fields;
		if mineral_fields.is_empty() {
			return;
		}
		let bases = self.units.my.townhalls.ready();
		if bases.is_empty() {
			return;
		}
		let gas_buildings = self
			.units
			.my
			.gas_buildings
			.filter(|g| g.is_ready() && g.vespene_contents.map_or(false, |vespene| vespene > 0));

		let mut deficit_minings = Units::new();
		let mut deficit_geysers = Units::new();

		bases.iter().for_each(
			|base| match base.assigned_harvesters.cmp(&base.ideal_harvesters) {
				Ordering::Equal => {}
				Ordering::Greater => {
					let local_minerals = self
						.units
						.mineral_fields
						.closer(11.0, base)
						.iter()
						.map(|m| m.tag)
						.collect::<Vec<u64>>();

					idle_workers.extend(
						self.units
							.my
							.workers
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
						self.units
							.my
							.workers
							.filter(|u| {
								u.target_tag().map_or(false, |target_tag| {
									target_tag == gas.tag
										|| (u.is_carrying_vespene()
											&& target_tag == bases.closest(gas).unwrap().tag)
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
			let minerals = mineral_fields.filter(|m| bases.iter().any(|base| base.is_closer(11.0, m)));
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
				let closest = deficit_geysers.closest(u).unwrap().tag;
				deficit_geysers.remove(closest);
				u.gather(closest, false);
			} else if !deficit_minings.is_empty() {
				let closest = deficit_minings.closest(u).unwrap().clone();
				deficit_minings.remove(closest.tag);
				u.gather(
					mineral_fields
						.closer(11.0, &closest)
						.max(|m| m.mineral_contents.unwrap_or(0))
						.unwrap()
						.tag,
					false,
				);
			} else if u.is_idle() {
				if let Some(minerals) = &minerals_near_base {
					u.gather(minerals.closest(u).unwrap().tag, false);
				}
			}
		});
	}

	fn get_builder(&self, pos: Point2, mineral_tags: &[u64]) -> Option<Unit> {
		let workers = self.units.my.workers.filter(|u| {
			!u.is_constructing()
				&& (!u.is_gathering() || u.target_tag().map_or(false, |tag| mineral_tags.contains(&tag)))
				&& !u.is_returning()
				&& !u.is_carrying_resource()
		});
		if workers.is_empty() {
			None
		} else {
			Some(workers.closest(pos).unwrap().clone())
		}
	}
	fn build(&mut self) {
		let mineral_tags = self
			.units
			.mineral_fields
			.iter()
			.map(|u| u.tag)
			.collect::<Vec<u64>>();
		let main_base = self.start_location.towards(self.game_info.map_center, 8.0);

		if self.current_units.get(&UnitTypeId::Refinery).unwrap_or(&0) < &2
			&& self.orders.get(&AbilityId::TerranBuildRefinery).unwrap_or(&0) == &0
			&& self.can_afford(UnitTypeId::Refinery, false)
		{
			let start_location = self.start_location;
			if let Some(geyser) = self.find_gas_placement(start_location) {
				if let Some(builder) = self.get_builder(geyser.position, &mineral_tags) {
					builder.build_gas(geyser.tag, false);
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
				self.find_placement(UnitTypeId::SupplyDepot, main_base, Default::default())
			{
				if let Some(builder) = self.get_builder(location, &mineral_tags) {
					builder.build(UnitTypeId::SupplyDepot, location, false);
					self.substract_resources(UnitTypeId::SupplyDepot);
					return;
				}
			}
		}

		if self.current_units.get(&UnitTypeId::Barracks).unwrap_or(&0)
			+ self.orders.get(&AbilityId::TerranBuildBarracks).unwrap_or(&0)
			< 4 && self.can_afford(UnitTypeId::Barracks, false)
		{
			if let Some(location) = self.find_placement(
				UnitTypeId::Barracks,
				main_base,
				PlacementOptions {
					step: 4,
					..Default::default()
				},
			) {
				if let Some(builder) = self.get_builder(location, &mineral_tags) {
					builder.build(UnitTypeId::Barracks, location, false);
					self.substract_resources(UnitTypeId::Barracks);
				}
			}
		}
	}

	fn train(&mut self) {
		if self.supply_workers < 22 && self.can_afford(UnitTypeId::SCV, true) {
			let townhalls = &self.units.my.townhalls;
			if !townhalls.is_empty() {
				let ccs = townhalls.filter(|u| u.is_ready() && u.is_almost_idle());
				if !ccs.is_empty() {
					ccs.first().unwrap().train(UnitTypeId::SCV, false);
					self.substract_resources(UnitTypeId::SCV);
				}
			}
		}

		if self.can_afford(UnitTypeId::Reaper, true) {
			let structures = &self.units.my.structures;
			if !structures.is_empty() {
				let barracks = structures
					.filter(|u| u.type_id == UnitTypeId::Barracks && u.is_ready() && u.is_almost_idle());
				if !barracks.is_empty() {
					barracks.first().unwrap().train(UnitTypeId::Reaper, false);
					self.substract_resources(UnitTypeId::Reaper);
				}
			}
		}
	}

	fn throw_mine(&mut self, reaper: &Unit, target: &Unit) -> bool {
		self.abilities_units.get(&reaper.tag).map_or(false, |abilities| {
			if abilities.contains(&AbilityId::KD8ChargeKD8Charge)
				&& reaper.is_closer(
					reaper.radius
						+ target.radius + self.game_data.abilities[&AbilityId::KD8ChargeKD8Charge]
						.cast_range
						.unwrap(),
					target,
				) {
				reaper.command(AbilityId::KD8ChargeKD8Charge, Target::Pos(target.position), false);
				true
			} else {
				false
			}
		})
	}
	fn execute_micro(&mut self) {
		// Lower ready depots
		self.units
			.my
			.structures
			.filter(|s| s.type_id == UnitTypeId::SupplyDepot && s.is_ready())
			.iter()
			.for_each(|s| (s.use_ability(AbilityId::MorphSupplyDepotLower, false)));

		// Reapers micro
		let reapers = self.units.my.units.of_type(UnitTypeId::Reaper);
		if reapers.is_empty() {
			return;
		}
		let targets = Some(
			self.units
				.enemy
				.all
				.filter(|u| !u.is_flying && u.can_attack_ground()),
		)
		.filter(|attackers| !attackers.is_empty())
		.or_else(|| Some(self.units.enemy.all.ground()).filter(|ground| !ground.is_empty()));

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
					if !self.throw_mine(u, &targets.closest(u).unwrap()) {
						if is_retreating || u.on_cooldown() {
							let close_enemies = targets
								.filter(|t| t.in_range(u, t.speed() + if is_retreating { 2.0 } else { 0.5 }));
							if !close_enemies.is_empty() {
								let retreat_position = {
									let closest = close_enemies.closest(u).unwrap().position;
									let pos = u.position.towards(closest, -u.speed());
									if self.is_pathable(pos) {
										pos
									} else {
										*u.position
											.neighbors8()
											.iter()
											.filter(|p| self.is_pathable(**p))
											.max_by(|p1, p2| {
												p1.distance_squared(closest)
													.partial_cmp(&p2.distance_squared(closest))
													.unwrap()
											})
											.unwrap_or(&self.start_location)
									}
								};
								u.move_to(Target::Pos(retreat_position), false);
							} else {
								let closest = targets.closest(u).unwrap();
								if !u.in_range(&closest, 0.0) {
									u.move_to(
										Target::Pos(if is_retreating {
											u.position
										} else {
											closest.position
										}),
										false,
									);
								}
							}
						} else {
							let close_targets = targets.in_range_of(u, 0.0);
							if !close_targets.is_empty() {
								u.attack(
									Target::Tag(close_targets.partial_min(|t| t.hits()).unwrap().tag),
									false,
								);
							} else {
								u.move_to(Target::Pos(targets.closest(u).unwrap().position), false);
							}
						}
					}
				}
				None => {
					u.move_to(
						Target::Pos(if is_retreating {
							u.position
						} else {
							self.enemy_start
						}),
						false,
					);
				}
			}
		});
	}
}

impl Player for ReaperRushAI {
	fn on_start(&mut self) -> SC2Result<()> {
		let townhall = self.units.my.townhalls.first().unwrap().clone();
		townhall.smart(Target::Pos(self.start_center), false);
		townhall.train(UnitTypeId::SCV, false);
		self.substract_resources(UnitTypeId::SCV);

		let minerals_near_base = self.units.mineral_fields.closer(11.0, &townhall);
		self.units.my.workers.clone().iter().for_each(|u| {
			u.gather(minerals_near_base.closest(u).unwrap().tag, false);
		});
		Ok(())
	}

	fn on_step(&mut self, _iteration: usize) -> SC2Result<()> {
		self.distribute_workers();
		self.build();
		self.train();
		self.execute_micro();
		Ok(())
	}

	fn get_player_settings(&self) -> PlayerSettings {
		PlayerSettings::new(Race::Terran, Some("RustyReapers"))
	}
}

fn main() -> SC2Result<()> {
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
			(@arg race: -r --race
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
			(@arg sc2_version: --("sc2-version")
				+takes_value
				"Sets sc2 version"
			)
			(@arg save_replay: --("save-replay")
				+takes_value
				"Sets path to save replay"
			)
			(@arg realtime: --realtime "Enables realtime mode")
		)
		(@subcommand human =>
			(about: "Runs game Human vs Bot")
			(@arg map: -m --map
				+takes_value
			)
			(@arg race: -r --race *
				+takes_value
				"Sets human race"
			)
			(@arg name: --name
				+takes_value
				"Sets human name"
			)
			(@arg sc2_version: --("sc2-version")
				+takes_value
				"Sets sc2 version"
			)
			(@arg save_replay: --("save-replay")
				+takes_value
				"Sets path to save replay"
			)
		)
	)
	.get_matches();

	let game_step = match app.value_of("game_step") {
		Some("0") => panic!("game_step must be X >= 1"),
		Some(step) => step.parse::<u32>().expect("Can't parse game_step"),
		None => unreachable!(),
	};

	let mut bot = ReaperRushAI::new();
	bot.game_step = game_step;

	if app.is_present("ladder_server") {
		run_ladder_game(
			&mut bot,
			app.value_of("ladder_server").unwrap_or("127.0.0.1"),
			app.value_of("host_port").expect("GamePort must be specified"),
			app.value_of("player_port")
				.expect("StartPort must be specified")
				.parse()
				.expect("Can't parse StartPort"),
			app.value_of("opponent_id"),
		)
	} else {
		let mut rng = thread_rng();

		match app.subcommand() {
			("local", Some(sub)) => run_vs_computer(
				&mut bot,
				Computer::new(
					sub.value_of("race").map_or(Race::Random, |race| {
						race.parse().expect("Can't parse computer race")
					}),
					sub.value_of("difficulty")
						.map_or(Difficulty::VeryEasy, |difficulty| {
							difficulty.parse().expect("Can't parse computer difficulty")
						}),
					sub.value_of("ai_build")
						.map(|ai_build| ai_build.parse().expect("Can't parse computer build")),
				),
				sub.value_of("map").unwrap_or_else(|| {
					[
						"AcropolisLE",
						"DiscoBloodbathLE",
						"EphemeronLE",
						"ThunderbirdLE",
						"TritonLE",
						"WintersGateLE",
						"WorldofSleepersLE",
					]
					.choose(&mut rng)
					.unwrap()
				}),
				LaunchOptions {
					sc2_version: sub.value_of("sc2_version"),
					realtime: sub.is_present("realtime"),
					save_replay_as: sub.value_of("save_replay"),
				},
			),
			("human", Some(sub)) => run_vs_human(
				&mut bot,
				PlayerSettings::new(
					sub.value_of("race")
						.unwrap()
						.parse()
						.expect("Can't parse human race"),
					sub.value_of("name"),
				),
				sub.value_of("map").unwrap_or_else(|| {
					[
						"AcropolisLE",
						"DiscoBloodbathLE",
						"EphemeronLE",
						"ThunderbirdLE",
						"TritonLE",
						"WintersGateLE",
						"WorldofSleepersLE",
					]
					.choose(&mut rng)
					.unwrap()
				}),
				LaunchOptions {
					sc2_version: sub.value_of("sc2_version"),
					realtime: true,
					save_replay_as: sub.value_of("save_replay"),
				},
			),
			_ => {
				println!("Game mode is not specified! Use -h, --help to print help information.");
				std::process::exit(0);
			}
		}
	}
}
