use rust_sc2::prelude::*;

#[bot]
#[derive(Default)]
struct EmptyBot;
impl Player for EmptyBot {
	fn get_player_settings(&self) -> PlayerSettings {
		PlayerSettings::new(Race::Random, None)
	}
}

// Example of how to use runner
fn main() -> SC2Result<()> {
	let mut bot = EmptyBot::default();

	// Bot vs Computer

	// Launch single SC2 client
	// Client version can be specified, otherwise will be used latest available version
	let mut runner = RunnerSingle::new(&mut bot, Some("4.10"))?;

	// Configure runner
	// Default: Computer(Race::Random, Difficulty::VeryEasy, AIBuild::Random)
	runner.computer = Computer::new(Race::Protoss, Difficulty::VeryHard, Some(AIBuild::Air));
	runner.map = "EternalEmpireLE"; // Must be specified
	runner.realtime = true; // Default: false
	runner.save_replay_as = Some("path/to/replay/MyReplay.SC2Replay"); // Default: None == don't save replay

	// Run game once
	runner.run_game()?;

	const MAPS: &[&str] = &["EverDreamLE", "GoldenWallLE", "IceandChromeLE"];
	const RACES: &[Race] = &[Race::Zerg, Race::Terran, Race::Protoss];
	const DIFFICULTIES: &[Difficulty] = &[Difficulty::Easy, Difficulty::Medium, Difficulty::Hard];

	// Run multiply times
	for i in 0..3 {
		// Configuration can be changed between games
		runner.map = MAPS[i];
		runner.computer.race = RACES[i];
		runner.computer.difficulty = DIFFICULTIES[i];

		runner.run_game()?;
	}

	// Better to close runner manually before launching other
	runner.close();

	let mut other = RunnerSingle::new(&mut bot, None)?;
	other.map = "Flat32";
	other.run_game()?;
	other.close();

	// Human vs Bot

	// Launch 2 SC2 clients
	let mut multi_runner = RunnerMulti::new(&mut bot, None)?;
	// Can be also made from single runner
	// let mut multi_runner = runner.into_multi()?;

	// Configured as single runner
	multi_runner.map = "PillarsofGoldLE";
	multi_runner.realtime = false; // Unnecessary line - This is default value
	multi_runner.save_replay_as = None; // Unnecessary line - This is default value

	// There is human's config instead of computer
	multi_runner.human_settings = PlayerSettings::new(Race::Random, Some("Name"));

	// Can be also reduced to single runner
	let _single_runner = multi_runner.into_single();

	// Runners dropped here:
	// Any launched sc2 clients will be closed automatically

	Ok(())
}
