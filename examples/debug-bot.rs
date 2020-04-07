#[macro_use]
extern crate clap;

use rand::prelude::{thread_rng, SliceRandom};
use rust_sc2::{
	bot, bot_impl_player, bot_new,
	geometry::Point2,
	player::{
		Difficulty,
		Players::{Computer, Human},
	},
	run_game, run_ladder_game, Player, PlayerSettings,
};

#[bot]
struct DebugAI {
	debug_z: f32,
}

impl DebugAI {
	#[bot_new]
	fn new(game_step: u32) -> Self {
		Self {
			game_step,
			debug_z: 0.0,
		}
	}
}

#[bot_impl_player]
impl Player for DebugAI {
	fn on_start(&mut self, _ws: &mut WS) {
		self.debug_z = self.grouped_units.townhalls[0].position3d.z;
	}

	fn on_step(&mut self, _ws: &mut WS, _iteration: usize) {
		// Debug expansion locations
		self.expansions.clone().iter().for_each(|(loc, center)| {
			self.debug
				.draw_sphere(loc.to3(self.debug_z), 0.6, Some((255, 128, 255)));
			self.debug
				.draw_sphere(center.to3(self.debug_z), 0.5, Some((255, 128, 64)));
		});

		// Debug unit types
		self.state.observation.raw.units.clone().iter().for_each(|u| {
			self.debug.draw_text_world(
				format!("{:?}", u.type_id),
				u.position3d,
				Some((255, 128, 128)),
				None,
			)
		});
	}

	fn get_player_settings(&self) -> PlayerSettings {
		PlayerSettings::new(Race::Random, None)
	}
}

fn main() {
	let app = clap_app!(DebugBot =>
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

	let bot = Box::new(DebugAI::new(game_step));

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
