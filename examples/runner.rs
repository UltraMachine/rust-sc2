use rust_sc2::prelude::*;

#[bot]
#[derive(Default)]
struct EmptyBot;
impl Player for EmptyBot {
	fn get_player_settings(&self) -> PlayerSettings {
		PlayerSettings::new(Race::Random)
	}
}

// Example of how to use runner
fn main() -> SC2Result<()> {
	let mut bot = EmptyBot::default();

	// Bot vs Computer
	// 1. Initialize runner
	let mut runner = RunnerSingle::new(
		&mut bot,
		Computer::new(Race::Random, Difficulty::VeryEasy, None),
		"EverDreamLE",
		Some("4.10"), // Client version can be specified, otherwise will be used latest available version
	);

	// 2. Configure runner
	runner.set_map("EternalEmpireLE");
	runner.computer = Computer::new(Race::Protoss, Difficulty::VeryHard, Some(AIBuild::Air));
	runner.realtime = true; // Default: false
	runner.save_replay_as = Some("path/to/replay/MyReplay.SC2Replay"); // Default: None == don't save replay

	// 3. Launch SC2
	runner.launch()?;

	// 4. Run games
	// Run game once
	runner.run_game()?;

	const MAPS: &[&str] = &["EverDreamLE", "GoldenWallLE", "IceandChromeLE"];
	const RACES: &[Race] = &[Race::Zerg, Race::Terran, Race::Protoss];
	const DIFFICULTIES: &[Difficulty] = &[Difficulty::Easy, Difficulty::Medium, Difficulty::Hard];

	// Run multiply times
	for i in 0..3 {
		// Configuration can be changed between games
		runner.set_map(MAPS[i]);
		runner.computer.race = RACES[i];
		runner.computer.difficulty = DIFFICULTIES[i];

		runner.run_game()?;
	}

	// Better to close runner manually before launching other
	runner.close();

	let mut other = RunnerSingle::new(
		&mut bot,
		Computer::new(Race::Random, Difficulty::VeryEasy, None),
		"Flat32",
		None,
	);
	other.run_game()?;
	other.close();

	// Human vs Bot
	// 1. Initialize runner
	let mut runner = RunnerMulti::new(
		&mut bot,
		PlayerSettings::new(Race::Random).with_name("Name"),
		"PillarsofGoldLE",
		None,
	);

	// 2. Configure runner
	runner.set_map("PillarsofGoldLE");
	runner.human_settings = PlayerSettings::new(Race::Random).with_name("Name");
	runner.realtime = false;
	runner.save_replay_as = None;

	// 3. Launch SC2
	runner.launch()?;

	// 4. Run games
	runner.run_game()?;

	// Runners dropped here:
	// Any launched sc2 clients will be closed automatically

	Ok(())
}
