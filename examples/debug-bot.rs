#[macro_use]
extern crate clap;

use rand::prelude::*;
use rust_sc2::{geometry::Point3, prelude::*};

#[bot]
#[derive(Default)]
struct DebugAI;

impl Player for DebugAI {
	fn on_step(&mut self, _iteration: usize) -> SC2Result<()> {
		// Debug expansion locations
		for exp in self.expansions.clone() {
			let (loc, center) = (exp.loc, exp.center);
			let z = self.get_z_height(loc) + 1.5;
			self.debug.draw_sphere(loc.to3(z), 0.6, Some((255, 128, 255)));
			let z = self.get_z_height(center) + 1.5;
			self.debug.draw_sphere(center.to3(z), 0.5, Some((255, 128, 64)));
		}

		// Debug unit types
		self.units
			.all
			.iter()
			.map(|u| (format!("{:?}", u.type_id()), u.position3d()))
			.collect::<Vec<(String, Point3)>>()
			.into_iter()
			.for_each(|(s, pos)| self.debug.draw_text_world(&s, pos, Some((255, 128, 128)), None));
		Ok(())
	}

	fn get_player_settings(&self) -> PlayerSettings {
		PlayerSettings::new(self.race, None)
	}
}

fn main() -> SC2Result<()> {
	let app = clap_app!(DebugBot =>
		(version: crate_version!())
		(author: crate_authors!())
		(@arg ladder_server: --LadderServer +takes_value)
		(@arg opponent_id: --OpponentId +takes_value)
		(@arg host_port: --GamePort +takes_value)
		(@arg player_port: --StartPort +takes_value)
		(@arg race: -r --race
			+takes_value
			"Sets race for debug bot"
		)
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

	let mut bot = DebugAI::default();
	bot.set_game_step(game_step);
	if let Some(race) = app
		.value_of("race")
		.map(|race| race.parse().expect("Can't parse bot race"))
	{
		bot.race = race;
	}

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
