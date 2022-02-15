#[macro_use]
extern crate clap;

use rand::prelude::{thread_rng, SliceRandom};
use rust_sc2::prelude::*;

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
	let app = clap_app!(RustyWorkers =>
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
		Some("0") => panic!("game_step must be X >= 1"),
		Some(step) => step.parse::<u32>().expect("Can't parse game_step"),
		None => unreachable!(),
	};

	let mut bot = WorkerRushAI::default();
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
			app.value_of("host_port")
				.expect("GamePort must be specified")
				.parse()
				.expect("Can't parse GamePort"),
			app.value_of("player_port")
				.expect("StartPort must be specified")
				.parse()
				.expect("Can't parse StartPort"),
			app.value_of("opponent_id"),
		),
	}
}
