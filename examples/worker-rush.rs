#[macro_use]
extern crate clap;

use rand::prelude::{thread_rng, SliceRandom};
use rust_sc2::{
	bot, bot_impl_player, bot_new,
	geometry::Point2,
	player::{
		Difficulty,
		Players::{Computer, Human},
		Race,
	},
	run_game, run_ladder_game, Player, PlayerSettings,
};

#[bot]
struct WorkerRushAI {
	race: Race,
	start_location: Point2,
	enemy_start: Point2,
	mineral_forward: u64,
	mineral_back: u64,
}

impl WorkerRushAI {
	#[bot_new]
	fn new(race: Race, game_step: u32) -> Self {
		Self {
			game_step,
			race,
			start_location: Default::default(),
			enemy_start: Default::default(),
			mineral_forward: Default::default(),
			mineral_back: Default::default(),
		}
	}
}

#[bot_impl_player]
impl Player for WorkerRushAI {
	fn on_step(&mut self, iteration: usize) {
		if iteration == 0 {
			let townhall = self.grouped_units.townhalls[0].clone();
			self.command(townhall.train(UnitTypeId::Probe, false));

			self.start_location = townhall.position;
			self.enemy_start = self.game_info.start_locations[0];
			self.mineral_forward = self.grouped_units.mineral_field.closest_pos(self.enemy_start).tag;
			self.mineral_back = self
				.grouped_units
				.mineral_field
				.closest_pos(self.start_location)
				.tag;
		}

		let ground_attackers = self.grouped_units.enemy_units.filter(|u| {
			!u.is_flying.as_bool() && u.can_attack_ground() && u.distance_pos_squared(self.enemy_start) < 2025.0
		});
		if !ground_attackers.is_empty() {
			self.grouped_units.workers.clone().iter().for_each(|u| {
				let closest = ground_attackers.closest(&u);
				if u.shield > Some(0.0) {
					self.command(u.attack(Target::Tag(closest.tag), false));
				} else if u.in_range_of(&closest, 2.0) {
					self.command(u.gather(Target::Tag(self.mineral_back), false));
				} else {
					self.command(u.gather(Target::Tag(self.mineral_forward), false));
				}
			})
		} else {
			let ground_structures = self
				.grouped_units
				.enemy_structures
				.filter(|u| !u.is_flying.as_bool() && u.distance_pos_squared(self.enemy_start) < 2025.0);
			if !ground_structures.is_empty() {
				self.grouped_units.workers.clone().iter().for_each(|u| {
					self.command(u.attack(Target::Tag(ground_structures.closest(&u).tag), false));
				})
			} else {
				self.grouped_units.workers.clone().iter().for_each(|u| {
					self.command(u.gather(Target::Tag(self.mineral_forward), false));
				})
			}
		}
	}
	fn get_player_settings(&self) -> PlayerSettings {
		PlayerSettings::new(self.race, Some("RustyWorkers".to_string()))
	}
}

#[tokio::main]
async fn main() {
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
			(@arg realtime: --realtime "Enables realtime mode")
		)
	)
	.get_matches();

	let game_step = match app.value_of("game_step") {
		Some("0") => panic!("game_step must be X >= 1"),
		Some(step) => step.parse::<u32>().expect("Can't parse game_step"),
		None => unreachable!(),
	};

	let bot = Box::new(WorkerRushAI::new(Race::Protoss, game_step));

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
		)
		.await
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
		.await
		.unwrap();
	}
}
