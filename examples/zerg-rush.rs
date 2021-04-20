#[macro_use]
extern crate clap;

use rand::prelude::*;
use rust_sc2::prelude::*;
use std::cmp::Ordering;

#[bot]
#[derive(Default)]
struct ZergRushAI {
	last_loop_distributed: u32,
}

impl Player for ZergRushAI {
	fn on_start(&mut self) -> SC2Result<()> {
		// Setting rallypoint for hatchery
		if let Some(townhall) = self.units.my.townhalls.first() {
			townhall.command(AbilityId::RallyWorkers, Target::Pos(self.start_center), false);
		}

		// Splitting workers to closest mineral crystals
		for u in &self.units.my.workers {
			if let Some(mineral) = self.units.mineral_fields.closest(u) {
				u.gather(mineral.tag(), false);
			}
		}

		// Ordering drone on initial 50 minerals
		if let Some(larva) = self.units.my.larvas.first() {
			larva.train(UnitTypeId::Drone, false);
		}
		self.subtract_resources(UnitTypeId::Drone, true);

		Ok(())
	}

	fn on_step(&mut self, _iteration: usize) -> SC2Result<()> {
		self.distribute_workers();
		self.upgrades();
		self.build();
		self.order_units();
		self.execute_micro();
		Ok(())
	}

	fn get_player_settings(&self) -> PlayerSettings {
		PlayerSettings::new(Race::Zerg, Some("RustyLings"))
	}
}

impl ZergRushAI {
	const DISTRIBUTION_DELAY: u32 = 8;

	fn distribute_workers(&mut self) {
		if self.units.my.workers.is_empty() {
			return;
		}
		let mut idle_workers = self.units.my.workers.idle();
		let bases = self.units.my.townhalls.ready();

		// Check distribution delay if there aren't any idle workers
		let game_loop = self.state.observation.game_loop();
		let last_loop = &mut self.last_loop_distributed;
		if idle_workers.is_empty() && *last_loop + Self::DISTRIBUTION_DELAY + bases.len() as u32 > game_loop {
			return;
		}
		*last_loop = game_loop;

		// Distribute
		let mineral_fields = &self.units.mineral_fields;
		if mineral_fields.is_empty() {
			return;
		}
		if bases.is_empty() {
			return;
		}

		let mut deficit_minings = Units::new();
		let mut deficit_geysers = Units::new();

		// Distributing mineral workers
		let mineral_tags = mineral_fields.iter().map(|m| m.tag()).collect::<Vec<u64>>();
		for base in &bases {
			match base.assigned_harvesters().cmp(&base.ideal_harvesters()) {
				Ordering::Less => (0..(base.ideal_harvesters().unwrap()
					- base.assigned_harvesters().unwrap()))
					.for_each(|_| {
						deficit_minings.push(base.clone());
					}),
				Ordering::Greater => {
					let local_minerals = mineral_fields
						.iter()
						.closer(11.0, base)
						.map(|m| m.tag())
						.collect::<Vec<u64>>();

					idle_workers.extend(
						self.units
							.my
							.workers
							.iter()
							.filter(|u| {
								u.target_tag().map_or(false, |target_tag| {
									local_minerals.contains(&target_tag)
										|| (u.is_carrying_minerals() && target_tag == base.tag())
								})
							})
							.take(
								(base.assigned_harvesters().unwrap() - base.ideal_harvesters().unwrap())
									as usize,
							)
							.cloned(),
					);
				}
				_ => {}
			}
		}

		// Distributing gas workers
		let speed_upgrade = UpgradeId::Zerglingmovementspeed;
		let has_enough_gas = self.can_afford_upgrade(speed_upgrade)
			|| self.has_upgrade(speed_upgrade)
			|| self.is_ordered_upgrade(speed_upgrade);
		let target_gas_workers = Some(if has_enough_gas { 0 } else { 3 });

		self.units.my.gas_buildings.iter().ready().for_each(|gas| {
			match gas.assigned_harvesters().cmp(&target_gas_workers) {
				Ordering::Less => {
					idle_workers.extend(self.units.my.workers.filter(|u| {
						u.target_tag()
							.map_or(false, |target_tag| mineral_tags.contains(&target_tag))
					}));
					(0..(gas.ideal_harvesters().unwrap() - gas.assigned_harvesters().unwrap())).for_each(
						|_| {
							deficit_geysers.push(gas.clone());
						},
					);
				}
				Ordering::Greater => {
					idle_workers.extend(
						self.units
							.my
							.workers
							.iter()
							.filter(|u| {
								u.target_tag().map_or(false, |target_tag| {
									target_tag == gas.tag()
										|| (u.is_carrying_vespene()
											&& target_tag == bases.closest(gas).unwrap().tag())
								})
							})
							.take(
								(gas.assigned_harvesters().unwrap() - gas.ideal_harvesters().unwrap())
									as usize,
							)
							.cloned(),
					);
				}
				_ => {}
			}
		});

		// Distributing idle workers
		let minerals_near_base = if idle_workers.len() > deficit_minings.len() + deficit_geysers.len() {
			let minerals = mineral_fields.filter(|m| bases.iter().any(|base| base.is_closer(11.0, *m)));
			if minerals.is_empty() {
				None
			} else {
				Some(minerals)
			}
		} else {
			None
		};

		for u in &idle_workers {
			if let Some(closest) = deficit_geysers.closest(u) {
				let tag = closest.tag();
				deficit_geysers.remove(tag);
				u.gather(tag, false);
			} else if let Some(closest) = deficit_minings.closest(u) {
				u.gather(
					mineral_fields
						.closer(11.0, closest)
						.max(|m| m.mineral_contents().unwrap_or(0))
						.unwrap()
						.tag(),
					false,
				);
				let tag = closest.tag();
				deficit_minings.remove(tag);
			} else if u.is_idle() {
				if let Some(mineral) = minerals_near_base.as_ref().and_then(|ms| ms.closest(u)) {
					u.gather(mineral.tag(), false);
				}
			}
		}
	}

	fn order_units(&mut self) {
		// Can't order units without resources
		if self.minerals < 50 {
			return;
		}

		// Order one queen per each base
		let queen = UnitTypeId::Queen;
		if self.counter().all().count(queen) < self.units.my.townhalls.len() && self.can_afford(queen, true) {
			if let Some(townhall) = self.units.my.townhalls.first() {
				townhall.train(queen, false);
				self.subtract_resources(queen, true);
			}
		}

		// Can't order units without larva
		if self.units.my.larvas.is_empty() {
			return;
		}

		let over = UnitTypeId::Overlord;
		if self.supply_left < 3
			&& self.supply_cap < 200
			&& self.counter().ordered().count(over) == 0
			&& self.can_afford(over, false)
		{
			if let Some(larva) = self.units.my.larvas.pop() {
				larva.train(over, false);
				self.subtract_resources(over, false);
			}
		}

		let drone = UnitTypeId::Drone;
		if (self.supply_workers as usize) < 96.min(self.counter().all().count(UnitTypeId::Hatchery) * 16)
			&& self.can_afford(drone, true)
		{
			if let Some(larva) = self.units.my.larvas.pop() {
				larva.train(drone, false);
				self.subtract_resources(drone, true);
			}
		}

		let zergling = UnitTypeId::Zergling;
		if self.can_afford(zergling, true) {
			if let Some(larva) = self.units.my.larvas.pop() {
				larva.train(zergling, false);
				self.subtract_resources(zergling, true);
			}
		}
	}

	fn get_builder(&self, pos: Point2, mineral_tags: &[u64]) -> Option<&Unit> {
		self.units
			.my
			.workers
			.iter()
			.filter(|u| {
				!(u.is_constructing()
					|| u.is_returning() || u.is_carrying_resource()
					|| (u.is_gathering() && u.target_tag().map_or(true, |tag| !mineral_tags.contains(&tag))))
			})
			.closest(pos)
	}
	fn build(&mut self) {
		if self.minerals < 75 {
			return;
		}

		let mineral_tags = self
			.units
			.mineral_fields
			.iter()
			.map(|u| u.tag())
			.collect::<Vec<u64>>();

		let pool = UnitTypeId::SpawningPool;
		if self.counter().all().count(pool) == 0 && self.can_afford(pool, false) {
			let place = self.start_location.towards(self.game_info.map_center, 6.0);
			if let Some(location) = self.find_placement(pool, place, Default::default()) {
				if let Some(builder) = self.get_builder(location, &mineral_tags) {
					builder.build(pool, location, false);
					self.subtract_resources(pool, false);
				}
			}
		}

		let extractor = UnitTypeId::Extractor;
		if self.counter().all().count(extractor) == 0 && self.can_afford(extractor, false) {
			let start = self.start_location;
			if let Some(geyser) = self.find_gas_placement(start) {
				if let Some(builder) = self.get_builder(geyser.position(), &mineral_tags) {
					builder.build_gas(geyser.tag(), false);
					self.subtract_resources(extractor, false);
				}
			}
		}

		let hatchery = UnitTypeId::Hatchery;
		if self.can_afford(hatchery, false) {
			if let Some(exp) = self.get_expansion() {
				if let Some(builder) = self.get_builder(exp.loc, &mineral_tags) {
					builder.build(hatchery, exp.loc, false);
					self.subtract_resources(hatchery, false);
				}
			}
		}
	}

	fn upgrades(&mut self) {
		let speed_upgrade = UpgradeId::Zerglingmovementspeed;
		if !(self.has_upgrade(speed_upgrade) || self.is_ordered_upgrade(speed_upgrade))
			&& self.can_afford_upgrade(speed_upgrade)
		{
			if let Some(pool) = self
				.units
				.my
				.structures
				.iter()
				.find(|s| s.type_id() == UnitTypeId::SpawningPool)
			{
				pool.research(speed_upgrade, false);
				self.subtract_upgrade_cost(speed_upgrade);
			}
		}
	}

	fn execute_micro(&self) {
		// Injecting Larva
		let mut queens = self.units.my.units.filter(|u| {
			u.type_id() == UnitTypeId::Queen
				&& !u.is_using(AbilityId::EffectInjectLarva)
				&& u.has_ability(AbilityId::EffectInjectLarva)
		});
		if !queens.is_empty() {
			self.units
				.my
				.townhalls
				.iter()
				.filter(|h| {
					!h.has_buff(BuffId::QueenSpawnLarvaTimer)
						|| h.buff_duration_remain().unwrap() * 20 > h.buff_duration_max().unwrap()
				})
				.for_each(|h| {
					if let Some(queen) = queens.closest(h) {
						queen.command(AbilityId::EffectInjectLarva, Target::Tag(h.tag()), false);
						let tag = queen.tag();
						queens.remove(tag);
					}
				});
		}

		let zerglings = self.units.my.units.of_type(UnitTypeId::Zergling);
		if zerglings.is_empty() {
			return;
		}

		// Check if speed upgrade is >80% ready
		let speed_upgrade = UpgradeId::Zerglingmovementspeed;
		let speed_upgrade_is_almost_ready =
			self.has_upgrade(speed_upgrade) || self.upgrade_progress(speed_upgrade) >= 0.8;

		// Attacking with zerglings or defending our locations
		let targets = if speed_upgrade_is_almost_ready {
			self.units.enemy.all.ground()
		} else {
			self.units
				.enemy
				.all
				.filter(|e| !e.is_flying() && self.units.my.townhalls.iter().any(|h| h.is_closer(25.0, *e)))
		};
		if !targets.is_empty() {
			for u in &zerglings {
				if let Some(target) = targets
					.iter()
					.in_range_of(u, 0.0)
					.min_by_key(|t| t.hits())
					.or_else(|| targets.closest(u))
				{
					u.attack(Target::Pos(target.position()), false);
				}
			}
		} else {
			let target = if speed_upgrade_is_almost_ready {
				self.enemy_start
			} else {
				self.start_location.towards(self.start_center, -8.0)
			};
			for u in &zerglings {
				u.move_to(Target::Pos(target), false);
			}
		}
	}
}

fn main() -> SC2Result<()> {
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
		Some("0") => panic!("game_step must be integer bigger than 0"),
		Some(step) => step.parse::<u32>().expect("Can't parse game_step"),
		None => unreachable!(),
	};

	let mut bot = ZergRushAI::default();
	bot.set_game_step(game_step);

	const LADDER_MAPS: &[&str] = &[
		"DeathauraLE",
		"EternalEmpireLE",
		"EverDreamLE",
		"GoldenWallLE",
		"IceandChromeLE",
		"PillarsofGoldLE",
		"SubmarineLE",
	];
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
			sub.value_of("map")
				.unwrap_or_else(|| LADDER_MAPS.choose(&mut rng).unwrap()),
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
			sub.value_of("map")
				.unwrap_or_else(|| LADDER_MAPS.choose(&mut rng).unwrap()),
			LaunchOptions {
				sc2_version: sub.value_of("sc2_version"),
				realtime: true,
				save_replay_as: sub.value_of("save_replay"),
			},
		),
		_ => run_ladder_game(
			&mut bot,
			app.value_of("ladder_server").unwrap_or("127.0.0.1"),
			app.value_of("host_port").expect("GamePort must be specified"),
			app.value_of("player_port")
				.expect("StartPort must be specified")
				.parse()
				.expect("Can't parse StartPort"),
			app.value_of("opponent_id"),
		),
	}
}
