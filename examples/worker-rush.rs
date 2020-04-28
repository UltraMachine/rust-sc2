#[macro_use]
extern crate clap;

use rand::prelude::{thread_rng, SliceRandom};
use rust_sc2::{
	action::Target,
	bot, bot_new,
	ids::UnitTypeId,
	player::{Computer, Difficulty, Race},
	run_ladder_game, run_vs_computer, run_vs_human, Player, PlayerSettings, SC2Result, WS,
};

#[bot]
struct WorkerRushAI {
	mineral_forward: u64,
	mineral_back: u64,
}

impl WorkerRushAI {
	#[bot_new]
	fn new() -> Self {
		Self {
			mineral_forward: Default::default(),
			mineral_back: Default::default(),
		}
	}
}

impl Player for WorkerRushAI {
	fn on_start(&mut self, _ws: &mut WS) -> SC2Result<()> {
		self.grouped_units
			.townhalls
			.first()
			.train(UnitTypeId::Probe, false);

		self.mineral_forward = self
			.grouped_units
			.mineral_fields
			.closest_pos(self.enemy_start)
			.tag;
		self.mineral_back = self
			.grouped_units
			.mineral_fields
			.closest_pos(self.start_location)
			.tag;
		Ok(())
	}

	fn on_step(&mut self, _ws: &mut WS, _iteration: usize) -> SC2Result<()> {
		let ground_attackers = self.grouped_units.enemy_units.filter(|u| {
			!u.is_flying && u.can_attack_ground() && u.distance_pos_squared(self.enemy_start) < 2025.0
		});
		if !ground_attackers.is_empty() {
			self.grouped_units.workers.clone().iter().for_each(|u| {
				let closest = ground_attackers.closest(&u);
				if u.shield > Some(0.0) {
					u.attack(Target::Tag(closest.tag), false);
				} else if u.in_range_of(&closest, 2.0) {
					u.gather(self.mineral_back, false);
				} else {
					u.gather(self.mineral_forward, false);
				}
			})
		} else {
			let ground_structures = self
				.grouped_units
				.enemy_structures
				.filter(|u| !u.is_flying && u.distance_pos_squared(self.enemy_start) < 2025.0);
			if !ground_structures.is_empty() {
				self.grouped_units.workers.clone().iter().for_each(|u| {
					u.attack(Target::Tag(ground_structures.closest(&u).tag), false);
				})
			} else {
				self.grouped_units.workers.clone().iter().for_each(|u| {
					u.gather(self.mineral_forward, false);
				})
			}
		}
		Ok(())
	}

	fn get_player_settings(&self) -> PlayerSettings {
		PlayerSettings::new(Race::Protoss, Some("RustyWorkers".to_string()))
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
		)
	)
	.get_matches();

	let game_step = match app.value_of("game_step") {
		Some("0") => panic!("game_step must be X >= 1"),
		Some(step) => step.parse::<u32>().expect("Can't parse game_step"),
		None => unreachable!(),
	};

	let mut bot = WorkerRushAI::new();
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
				None,
				sub.is_present("realtime"),
			),
			("human", Some(sub)) => run_vs_human(
				&mut bot,
				PlayerSettings::new(
					sub.value_of("race")
						.unwrap()
						.parse()
						.expect("Can't parse human race"),
					sub.value_of("name").map(|name| name.to_string()),
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
				None,
			),
			_ => {
				println!("Game mode is not specified! Use -h, --help to print help information.");
				std::process::exit(0);
			}
		}
	}
}
