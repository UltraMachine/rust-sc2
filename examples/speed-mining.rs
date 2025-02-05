use rust_sc2::prelude::*;
use std::collections::{HashMap, HashSet};

mod ex_main;

#[bot]
#[derive(Default)]
struct LightningMcQueen {
	base_indices: HashMap<u64, usize>,    // (base tag, expansion index)
	assigned: HashMap<u64, HashSet<u64>>, // (mineral, workers)
	free_workers: HashSet<u64>,           // tags of workers which aren't assigned to any work
	harvesters: HashMap<u64, (u64, u64)>, // (worker, (target mineral, nearest townhall))
	targets: HashMap<u64, Point2>,        // (mineral, target move location)
}

impl Player for LightningMcQueen {
	fn get_player_settings(&self) -> PlayerSettings {
		PlayerSettings::new(self.race).raw_crop_to_playable_area(true)
	}

	fn on_event(&mut self, event: Event) -> SC2Result<()> {
		match event {
			Event::UnitCreated(tag) => {
				if let Some(u) = self.units.my.units.get(tag) {
					if u.type_id() == self.race_values.worker {
						self.free_workers.insert(tag);
					}
				}
			}
			Event::ConstructionComplete(tag) => {
				if let Some(u) = self.units.my.structures.get(tag) {
					if u.type_id() == self.race_values.start_townhall {
						if let Some(idx) = self
							.expansions
							.iter()
							.enumerate()
							.find(|(_, exp)| exp.base == Some(tag))
							.map(|(idx, _)| idx)
						{
							self.base_indices.insert(tag, idx);
						}
					}
				}
			}
			Event::UnitDestroyed(tag, alliance) => {
				let remove_mineral = |bot: &mut LightningMcQueen, tag| {
					if let Some(ws) = bot.assigned.remove(&tag) {
						for w in ws {
							bot.harvesters.remove(&w);
							bot.free_workers.insert(w);
						}
					}
				};

				match alliance {
					Some(Alliance::Own) => {
						// townhall destroyed
						if let Some(idx) = self.base_indices.remove(&tag) {
							let exp = &self.expansions[idx];
							for m in exp.minerals.clone() {
								remove_mineral(self, m);
							}
						// harvester died
						} else if let Some((m, _)) = self.harvesters.remove(&tag) {
							self.assigned.entry(m).and_modify(|ws| {
								ws.remove(&tag);
							});
						// free worker died
						} else {
							self.free_workers.remove(&tag);
						}
					}
					// mineral mined out
					Some(Alliance::Neutral) => remove_mineral(self, tag),
					_ => {}
				}
			}
			_ => {}
		}
		Ok(())
	}

	fn on_start(&mut self) -> SC2Result<()> {
		self.assign_mineral_targets();

		Ok(())
	}

	fn on_step(&mut self, _iteration: usize) -> SC2Result<()> {
		self.assign_roles();
		self.execute_micro();

		// visualise the mineral target points
		for (mmineral_tag, t) in self.targets.clone() {
			if let Some(m) = self.units.mineral_fields.get(mmineral_tag).map(|m| m.position()) {
				let start = t.to3(self.get_z_height(t) + 0.5);
				let end = m.to3(self.get_z_height(m) + 0.5);

				self.debug.draw_line(start, end, Some((255, 255, 60)));
				self.debug.draw_sphere(start, 0.5, Some((255, 255, 60)));
			}
		}

		// print out total minerals gathered by the 5 minute mark
		if self.state.observation.game_loop() == 6720 {
			println!(
				"mined {} minerals by {}:{:02}",
				self.minerals,
				self.time as usize / 60,
				self.time as usize % 60
			);
		}

		Ok(())
	}
}

impl LightningMcQueen {
	const MINERAL_RADIUS: f32 = 1.35;

	fn assign_mineral_targets(&mut self) {
		for (&b, &i) in &self.base_indices {
			let base = self.units.my.townhalls[b].position();

			for m in self.expansions[i].minerals.clone() {
				let mineral = self.units.mineral_fields[m].position();

				// default target point is straight towards the townhall
				let mut target = mineral.towards(base, Self::MINERAL_RADIUS);

				// find the position of all other mineral patches within 1.5 radius of this one
				let nearby_minerals = self
					.units
					.mineral_fields
					.closer(Self::MINERAL_RADIUS * 1.5, mineral)
					.filter(|p| p.tag() != m)
					.iter()
					.map(|p| p.position())
					.collect::<Vec<_>>();

				// create an offset vector that pushes the target away from each nearby minerals
				let mut offset = Point2::new(0.0, 0.0);
				for &patch in &nearby_minerals {
					let push = patch.towards(target, 1.0) - patch;
					offset += push / mineral.distance(patch);
				}

				// add our offset, and then normalise the resulting point back onto the radius
				target = mineral.towards(target + offset, Self::MINERAL_RADIUS);

				self.targets.insert(m, target);
			}
		}
	}

	fn assign_roles(&mut self) {
		let mut to_harvest = vec![];
		// iterator of (mineral tag, nearest base tag)
		let mut harvest_targets = self.base_indices.iter().flat_map(|(b, i)| {
			self.expansions[*i]
				.minerals
				.iter()
				.map(|m| (m, 2 - self.assigned.get(m).map_or(0, |ws| ws.len())))
				.flat_map(move |(m, c)| vec![(*m, *b); c])
		});

		for w in &self.free_workers {
			if let Some(t) = harvest_targets.next() {
				to_harvest.push((*w, t));
			} else {
				break;
			}
		}

		for (w, t) in to_harvest {
			self.free_workers.remove(&w);
			self.harvesters.insert(w, t);
			self.assigned.entry(t.0).or_default().insert(w);
		}
	}

	fn execute_micro(&mut self) {
		for u in &self.units.my.workers.clone() {
			if let Some((mineral_tag, base_tag)) = self.harvesters.get(&u.tag()) {
				// only need to change orders if we don't already have 2 commands queued
				if u.orders().len() < 2 {
					// we're on our way back from a mineral field
					if u.is_carrying_resource() {
						let base = &self.units.my.townhalls[*base_tag];
						let target: Point2 = base.position().towards(u.position(), base.radius() * 1.08);
						let distance = u.position().distance_squared(target);
						// let the built-in unit behaviour handle the first ~half of the trip
						if distance > 0.5625 && distance < 4.0 {
							u.move_to(Target::Pos(target), false);
							u.smart(Target::Tag(*base_tag), true);
						}
						// deal with the rare case where collisions cause the worker to just park itself
						else if !u.is_returning() {
							u.smart(Target::Tag(*base_tag), false);
						}
					}
					// we're on our way to a mineral field
					else {
						let target: Point2 = self.targets[mineral_tag];
						let distance = u.position().distance_squared(target);
						// again we want to mineral walk as much of the way as possible, before using the queue trick
						if distance > 0.5625 && distance < 4.0 {
							u.move_to(Target::Pos(target), false);
							u.smart(Target::Tag(*mineral_tag), true);
						}
						// either sc2 accidentally deposited the minerals early, or it switched mineral fields on us
						else if !u.is_gathering() || u.target_tag().map_or(false, |t| t != *mineral_tag) {
							u.gather(*mineral_tag, false);
						}
					}
				}
			}
		}
	}
}

fn main() -> SC2Result<()> {
	ex_main::main(LightningMcQueen::default())
}
