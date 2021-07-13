#[macro_use]
extern crate clap;

use rand::prelude::*;
use rust_sc2::prelude::*;
use std::collections::{HashMap, HashSet};

#[bot]
#[derive(Default)]
struct LightningMcQueen {
	base_indices: HashMap<u64, usize>,    // (base tag, expansion index)
	assigned: HashMap<u64, HashSet<u64>>, // (mineral, workers)
	free_workers: HashSet<u64>,           // tags of workers which aren't assigned to any work
	harvesters: HashMap<u64, (u64, u64)>, // (worker, (target mineral, nearest townhall))
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

	fn on_step(&mut self, _iteration: usize) -> SC2Result<()> {
		self.assign_roles();
		self.execute_micro();
		Ok(())
	}
}

impl LightningMcQueen {
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
		let (gather_ability, return_ability) = match self.race {
			Race::Terran => (AbilityId::HarvestGatherSCV, AbilityId::HarvestReturnSCV),
			Race::Zerg => (AbilityId::HarvestGatherDrone, AbilityId::HarvestReturnDrone),
			Race::Protoss => (AbilityId::HarvestGatherProbe, AbilityId::HarvestReturnProbe),
			_ => unreachable!(),
		};
		let mut mineral_moving = HashSet::new();

		for u in &self.units.my.workers {
			if let Some((mineral_tag, base_tag)) = self.harvesters.get(&u.tag()) {
				let is_collides = || {
					let range = (u.radius() + u.distance_per_step()) * 2.0;
					!self.assigned[mineral_tag].iter().all(|&w| {
						w == u.tag()
							|| mineral_moving.contains(&w)
							|| u.is_further(range, &self.units.my.workers[w])
					})
				};

				match u.orders().first().map(|ord| (ord.ability, ord.target)) {
					// moving
					Some((AbilityId::MoveMove, Target::Pos(current_target))) => {
						let mineral = &self.units.mineral_fields[*mineral_tag];
						let range = mineral.radius() + u.distance_per_step();
						// moving towards mineral
						if current_target.is_closer(range, mineral) {
							// execute gather ability if close enough or colliding with other workers
							if u.is_closer(u.radius() + range, mineral) || is_collides() {
								u.smart(Target::Tag(mineral.tag()), false);
								mineral_moving.insert(u.tag());
							}
							// otherwise keep moving
							continue;
						} else {
							let base = &self.units.my.townhalls[*base_tag];
							let range = base.radius() + u.distance_per_step();
							// moving towards base
							if current_target.is_closer(range, base) {
								// execute return ability if close enough or colliding with other workers
								if u.is_closer(u.radius() + range, base) || is_collides() {
									u.smart(Target::Tag(base.tag()), false);
									mineral_moving.insert(u.tag());
								}
								// otherwise keep moving
								continue;
							}
						}
					}
					// gathering
					Some((ability, Target::Tag(t))) if ability == gather_ability && t == *mineral_tag => {
						let mineral = &self.units.mineral_fields[*mineral_tag];
						// execute move ability if far away from mineral and not colliding with other workers
						if u.is_further(u.radius() + mineral.radius() + u.distance_per_step(), mineral)
							&& !is_collides()
						{
							let base = &self.units.my.townhalls[*base_tag];
							u.move_to(
								Target::Pos(mineral.position().towards(base.position(), mineral.radius())),
								false,
							);
						// otherwise keep gathering
						} else {
							mineral_moving.insert(u.tag());
						}
						continue;
					}
					// returning
					Some((ability, Target::Tag(t))) if ability == return_ability && t == *base_tag => {
						let base = &self.units.my.townhalls[*base_tag];
						// execute move ability if far away from base and not colliding with other workers
						if u.is_further(u.radius() + base.radius() + u.distance_per_step(), base)
							&& !is_collides()
						{
							u.move_to(
								Target::Pos(base.position().towards(u.position(), base.radius())),
								false,
							);
						// otherwise keep returning
						} else {
							mineral_moving.insert(u.tag());
						}
						continue;
					}
					_ => {}
				}

				// execute default ability if worker is doing something it shouldn't do
				if u.is_carrying_resource() {
					u.return_resource(false);
				} else {
					u.gather(*mineral_tag, false);
				}
				mineral_moving.insert(u.tag());
			}
		}
	}
}

fn main() -> SC2Result<()> {
	let app = clap_app!(Sonic =>
		(version: crate_version!())
		(author: crate_authors!())
		(@arg ladder_server: --LadderServer +takes_value)
		(@arg opponent_id: --OpponentId +takes_value)
		(@arg host_port: --GamePort +takes_value)
		(@arg player_port: --StartPort +takes_value)
		(@arg realtime: --RealTime)
		(@arg race: -r --race
			+takes_value
			"Sets race for bot"
		)
		(@arg game_step: -s --step
			+takes_value
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
			(@arg ai_build: -b --("ai-build")
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
			(@arg name: -n --name
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

	let mut bot = LightningMcQueen::default();

	if let Some(step) = app.value_of("game_step") {
		if step == "0" {
			panic!("game_step must be X >= 1");
		}
		bot.set_game_step(step.parse::<u32>().expect("Can't parse game_step"));
	}
	if let Some(race) = app
		.value_of("race")
		.map(|race| race.parse().expect("Can't parse bot race"))
	{
		bot.race = race;
	}

	const LADDER_MAPS: &[&str] = &[
		"Deathaura506",
		"EternalEmpire506",
		"EverDream506",
		"GoldenWall506",
		"IceandChrome506",
		"PillarsofGold506",
		"Submarine506",
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
			PlayerSettings {
				race: sub
					.value_of("race")
					.unwrap()
					.parse()
					.expect("Can't parse human race"),
				name: sub.value_of("name"),
				..Default::default()
			},
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
