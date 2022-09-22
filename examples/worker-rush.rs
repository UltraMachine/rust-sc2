use rand::prelude::{thread_rng, SliceRandom};
use rust_sc2::prelude::*;

mod ex_main;

#[bot]
#[derive(Default)]
struct WorkerRushAI {
	mineral_forward: u64,
	mineral_back: u64,
}

impl Player for WorkerRushAI {
	fn on_start(&mut self) -> SC2Result<()> {
		if let Some(townhall) = self.units.my.townhalls.first() {
			townhall.train(UnitTypeId::Probe, false);
		}

		if let Some(closest) = self.units.mineral_fields.closest(self.enemy_start) {
			self.mineral_forward = closest.tag();
		}
		if let Some(closest) = self.units.mineral_fields.closest(self.start_location) {
			self.mineral_back = closest.tag();
		}

		Ok(())
	}

	fn on_step(&mut self, _iteration: usize) -> SC2Result<()> {
		let ground_attackers = self
			.units
			.enemy
			.units
			.filter(|u| !u.is_flying() && u.can_attack_ground() && u.is_closer(45.0, self.enemy_start));
		if !ground_attackers.is_empty() {
			for u in &self.units.my.workers {
				let closest = ground_attackers.closest(u).unwrap();
				if u.shield() > Some(5) {
					if !u.on_cooldown() {
						u.attack(Target::Tag(closest.tag()), false);
					} else {
						u.gather(self.mineral_back, false);
					}
				} else if u.in_range_of(closest, 2.0) {
					u.gather(self.mineral_back, false);
				} else {
					u.gather(self.mineral_forward, false);
				}
			}
		} else {
			let ground_structures = self
				.units
				.enemy
				.structures
				.filter(|u| !u.is_flying() && u.is_closer(45.0, self.enemy_start));
			if !ground_structures.is_empty() {
				for u in &self.units.my.workers {
					u.attack(Target::Tag(ground_structures.closest(u).unwrap().tag()), false);
				}
			} else {
				for u in &self.units.my.workers {
					u.gather(self.mineral_forward, false);
				}
			}
		}
		Ok(())
	}

	fn get_player_settings(&self) -> PlayerSettings {
		PlayerSettings::new(Race::Protoss).with_name("RustyWorkers")
	}
}

fn main() -> SC2Result<()> {
	ex_main::main(WorkerRushAI::default())
}
