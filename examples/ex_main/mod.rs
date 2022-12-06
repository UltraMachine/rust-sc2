use clap::{Parser, Subcommand};
use rust_sc2::bot::Bot;
use rust_sc2::prelude::*;
use std::ops::{Deref, DerefMut};

#[derive(Parser)]
#[clap(version, author)]
struct Args {
	#[clap(long = "LadderServer")]
	ladder_server: Option<String>,
	#[clap(long = "OpponentId")]
	opponent_id: Option<String>,
	#[clap(long = "GamePort")]
	host_port: Option<i32>,
	#[clap(long = "StartPort")]
	player_port: Option<i32>,
	#[clap(long = "RealTime")]
	realtime: bool,

	/// Set game step for bot
	#[clap(short = 's', long = "step", default_value = "2")]
	game_step: u32,

	#[clap(subcommand)]
	command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
	/// Run local game vs Computer
	Local {
		/// Set path to map, relative to "Starcraft II/Maps" directory.
		/// See `https://github.com/UltraMachine/rust-sc2/blob/master/src/paths.rs#L38-L67`
		#[clap(short, long)]
		map: String,
		/// Set opponent race
		#[clap(short, long, default_value = "Random")]
		race: Race,
		/// Set opponent diffuculty
		#[clap(short, long)]
		difficulty: Option<Difficulty>,
		/// Set opponent build
		#[clap(short = 'b', long)]
		ai_build: Option<AIBuild>,
		/// Set sc2 version
		#[clap(long)]
		sc2_version: Option<String>,
		/// Set path to save replay
		#[clap(long)]
		save_replay: Option<String>,
		/// Enable realtime mode
		#[clap(long)]
		realtime: bool,
	},
	/// Run game Human vs Bot
	Human {
		/// Set path to map, relative to "Starcraft II/Maps" directory.
		/// See `https://github.com/UltraMachine/rust-sc2/blob/master/src/paths.rs#L38-L67`
		#[clap(short, long)]
		map: String,
		/// Set human race
		#[clap(short, long, default_value = "Random")]
		race: Race,
		/// Set human name
		#[clap(short, long)]
		name: Option<String>,
		/// Set sc2 version
		#[clap(long)]
		sc2_version: Option<String>,
		/// Set path to save replay
		#[clap(long)]
		save_replay: Option<String>,
	},
}

pub(crate) fn main(mut bot: impl Player + DerefMut<Target = Bot> + Deref<Target = Bot>) -> SC2Result<()> {
	let args = Args::parse();

	if args.game_step == 0 {
		panic!("game_step must be X >= 1")
	}
	bot.set_game_step(args.game_step);

	match args.command {
		Some(Command::Local {
			map,
			race,
			difficulty,
			ai_build,
			sc2_version,
			save_replay,
			realtime,
		}) => run_vs_computer(
			&mut bot,
			Computer::new(race, difficulty.unwrap_or(Difficulty::VeryEasy), ai_build),
			&map,
			LaunchOptions {
				sc2_version: sc2_version.as_deref(),
				realtime,
				save_replay_as: save_replay.as_deref(),
			},
		),
		Some(Command::Human {
			map,
			race,
			name,
			sc2_version,
			save_replay,
		}) => run_vs_human(
			&mut bot,
			PlayerSettings {
				race,
				name: name.as_deref(),
				..Default::default()
			},
			&map,
			LaunchOptions {
				sc2_version: sc2_version.as_deref(),
				realtime: true,
				save_replay_as: save_replay.as_deref(),
			},
		),
		None => run_ladder_game(
			&mut bot,
			args.ladder_server.as_deref().unwrap_or("127.0.0.1"),
			args.host_port.expect("GamePort must be specified"),
			args.player_port.expect("StartPort must be specified"),
			args.opponent_id.as_deref(),
		),
	}
}
