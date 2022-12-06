use rust_sc2::prelude::*;
use std::{cmp::Ordering, collections::HashSet};

mod ex_main;

#[bot]
#[derive(Default)]
struct ReaperRushAI {
	reapers_retreat: HashSet<u64>,
	last_loop_distributed: u32,
}

impl Player for ReaperRushAI {
	fn on_start(&mut self) -> SC2Result<()> {
		if let Some(townhall) = self.units.my.townhalls.first() {
			// Setting rallypoint for command center
			townhall.smart(Target::Pos(self.start_center), false);

			// Ordering scv on initial 50 minerals
			townhall.train(UnitTypeId::SCV, false);
			self.subtract_resources(UnitTypeId::SCV, true);
		}

		// Splitting workers to closest mineral crystals
		for u in &self.units.my.workers {
			if let Some(mineral) = self.units.mineral_fields.closest(u) {
				u.gather(mineral.tag(), false);
			}
		}

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
		PlayerSettings::new(Race::Terran).with_name("RustyReapers")
	}
}

impl ReaperRushAI {
	const DISTRIBUTION_DELAY: u32 = 8;

	fn distribute_workers(&mut self) {
		if self.units.my.workers.is_empty() {
			return;
		}
		let mut idle_workers = self.units.my.workers.idle();

		// Check distribution delay if there aren't any idle workers
		let game_loop = self.state.observation.game_loop();
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

		let mut deficit_minings = Units::new();
		let mut deficit_geysers = Units::new();

		// Distributing mineral workers
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
							.filter(|u| {
								u.target_tag().map_or(false, |target_tag| {
									local_minerals.contains(&target_tag)
										|| (u.is_carrying_minerals() && target_tag == base.tag())
								})
							})
							.iter()
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
		self.units
			.my
			.gas_buildings
			.iter()
			.ready()
			.filter(|g| g.vespene_contents().map_or(false, |vespene| vespene > 0))
			.for_each(
				|gas| match gas.assigned_harvesters().cmp(&gas.ideal_harvesters()) {
					Ordering::Less => (0..(gas.ideal_harvesters().unwrap()
						- gas.assigned_harvesters().unwrap()))
						.for_each(|_| {
							deficit_geysers.push(gas.clone());
						}),
					Ordering::Greater => {
						idle_workers.extend(
							self.units
								.my
								.workers
								.filter(|u| {
									u.target_tag().map_or(false, |target_tag| {
										target_tag == gas.tag()
											|| (u.is_carrying_vespene()
												&& target_tag == bases.closest(gas).unwrap().tag())
									})
								})
								.iter()
								.take(
									(gas.assigned_harvesters().unwrap() - gas.ideal_harvesters().unwrap())
										as usize,
								)
								.cloned(),
						);
					}
					_ => {}
				},
			);

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
		let main_base = self.start_location.towards(self.game_info.map_center, 8.0);

		if self.counter().count(UnitTypeId::Refinery) < 2
			&& self.counter().ordered().count(UnitTypeId::Refinery) == 0
			&& self.can_afford(UnitTypeId::Refinery, false)
		{
			let start_location = self.start_location;
			if let Some(geyser) = self.find_gas_placement(start_location) {
				if let Some(builder) = self.get_builder(geyser.position(), &mineral_tags) {
					builder.build_gas(geyser.tag(), false);
					self.subtract_resources(UnitTypeId::Refinery, false);
				}
			}
		}

		if self.supply_left < 3
			&& self.supply_cap < 200
			&& self.counter().ordered().count(UnitTypeId::SupplyDepot) == 0
			&& self.can_afford(UnitTypeId::SupplyDepot, false)
		{
			if let Some(location) =
				self.find_placement(UnitTypeId::SupplyDepot, main_base, Default::default())
			{
				if let Some(builder) = self.get_builder(location, &mineral_tags) {
					builder.build(UnitTypeId::SupplyDepot, location, false);
					self.subtract_resources(UnitTypeId::SupplyDepot, false);
					return;
				}
			}
		}

		if self.counter().all().count(UnitTypeId::Barracks) < 4
			&& self.can_afford(UnitTypeId::Barracks, false)
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
					self.subtract_resources(UnitTypeId::Barracks, false);
				}
			}
		}
	}

	fn train(&mut self) {
		if self.minerals < 50 || self.supply_left == 0 {
			return;
		}

		if self.supply_workers < 22 && self.can_afford(UnitTypeId::SCV, true) {
			if let Some(cc) = self
				.units
				.my
				.townhalls
				.iter()
				.find(|u| u.is_ready() && u.is_almost_idle())
			{
				cc.train(UnitTypeId::SCV, false);
				self.subtract_resources(UnitTypeId::SCV, true);
			}
		}

		if self.can_afford(UnitTypeId::Reaper, true) {
			if let Some(barracks) = self
				.units
				.my
				.structures
				.iter()
				.find(|u| u.type_id() == UnitTypeId::Barracks && u.is_ready() && u.is_almost_idle())
			{
				barracks.train(UnitTypeId::Reaper, false);
				self.subtract_resources(UnitTypeId::Reaper, true);
			}
		}
	}

	fn throw_mine(&self, reaper: &Unit, target: &Unit) -> bool {
		if reaper.has_ability(AbilityId::KD8ChargeKD8Charge)
			&& reaper.in_ability_cast_range(AbilityId::KD8ChargeKD8Charge, target, 0.0)
		{
			reaper.command(
				AbilityId::KD8ChargeKD8Charge,
				Target::Pos(target.position()),
				false,
			);
			true
		} else {
			false
		}
	}
	fn execute_micro(&mut self) {
		// Lower ready depots
		self.units
			.my
			.structures
			.iter()
			.of_type(UnitTypeId::SupplyDepot)
			.ready()
			.for_each(|s| s.use_ability(AbilityId::MorphSupplyDepotLower, false));

		// Reapers micro
		let reapers = self.units.my.units.of_type(UnitTypeId::Reaper);
		if reapers.is_empty() {
			return;
		}

		let targets = {
			let ground_targets = self.units.enemy.all.ground();
			let ground_attackers = ground_targets.filter(|e| e.can_attack_ground());
			if ground_attackers.is_empty() {
				ground_targets
			} else {
				ground_attackers
			}
		};

		for u in &reapers {
			let is_retreating = self.reapers_retreat.contains(&u.tag());
			if is_retreating {
				if u.health_percentage().unwrap() > 0.75 {
					self.reapers_retreat.remove(&u.tag());
				}
			} else if u.health_percentage().unwrap() < 0.5 {
				self.reapers_retreat.insert(u.tag());
			}

			match targets.closest(u) {
				Some(closest) => {
					if self.throw_mine(u, closest) {
						return;
					}
					if is_retreating || u.on_cooldown() {
						match targets
							.iter()
							.filter(|t| t.in_range(u, t.speed() + if is_retreating { 2.0 } else { 0.5 }))
							.closest(u)
						{
							Some(closest_attacker) => {
								let flee_position = {
									let pos = u.position().towards(closest_attacker.position(), -u.speed());
									if self.is_pathable(pos) {
										pos
									} else {
										*u.position()
											.neighbors8()
											.iter()
											.filter(|p| self.is_pathable(**p))
											.furthest(closest_attacker)
											.unwrap_or(&self.start_location)
									}
								};
								u.move_to(Target::Pos(flee_position), false);
							}
							None => {
								if !(is_retreating || u.in_range(closest, 0.0)) {
									u.move_to(Target::Pos(closest.position()), false);
								}
							}
						}
					} else {
						match targets.iter().in_range_of(u, 0.0).min_by_key(|t| t.hits()) {
							Some(target) => u.attack(Target::Tag(target.tag()), false),
							None => u.move_to(Target::Pos(closest.position()), false),
						}
					}
				}
				None => {
					let pos = if is_retreating {
						u.position()
					} else {
						self.enemy_start
					};
					u.move_to(Target::Pos(pos), false);
				}
			}
		}
	}
}

fn main() -> SC2Result<()> {
	ex_main::main(ReaperRushAI::default())
}
