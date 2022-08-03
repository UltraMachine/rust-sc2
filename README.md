<!-- markdown-toc start - Don't edit this section. Run M-x markdown-toc-refresh-toc -->
**Table of Contents**

- [rust-sc2](#rust-sc2)
- [Getting started](#getting-started)
	- [Rust](#rust)
	- [StarCraft II](#starcraft-ii)
		- [Windows and macOS](#windows-and-macos)
		- [Linux](#linux)
			- [Lutris and Wine](#lutris-and-wine)
			- [Headless (no graphics)](#headless-no-graphics)
- [Bug Workarounds (as of 7/31/22)](#bug-workarounds-as-of-73122)
- [Example](#example)
	- [Running Example](#running-example)
		- [Lutris](#lutris)
		- [Headless](#headless)
	- [Optional features](#optional-features)
	- [Making bot step by step](#making-bot-step-by-step)

<!-- markdown-toc end -->


# rust-sc2
[![crates.io](https://img.shields.io/crates/v/rust-sc2.svg)](https://crates.io/crates/rust-sc2)
[![Documentation](https://docs.rs/rust-sc2/badge.svg)](https://docs.rs/rust-sc2)

Rust implementation of StarCraft II API

The library aims to be simple and easy to use, being very fast and functional at the same time. However, it provides both high and low level abstractions. This lib is inspired by [python-sc2](https://github.com/BurnySc2/python-sc2) lib, so people might find it easy to switch to rust-sc2. It was originally created because other rust libs were old, not functional and low level.

Feel free to ask questions in `#rust` channel of these Discord servers:
- [Starcraft 2 AI](https://discord.gg/Emm5Ztz)
- [AI Arena](https://discord.gg/yDBzbtC)


# Getting started
## Rust
[Install Rust](https://www.rust-lang.org/tools/install) >= 1.42.0

Create your project

`cargo add <name_of_project>`

Warning: Compilation is broken in rustc 1.45.0 - 1.46.0, you'll get following error:
```
thread 'rustc' has overflowed its stack
error: could not compile `rust-sc2`.
```

Add to dependencies in Cargo.toml:
```toml
[dependencies]
rust-sc2 = "1.1.0"
```
Or if you want developer version directly from github:
```toml
[dependencies]
rust-sc2 = { git = "https://github.com/UltraMachine/rust-sc2" }
```

Or if you want to use a local version:
```toml
[dependencies]
rust-sc2 = { path = "/path/to/rust-sc2" }
```

## StarCraft II

### Installation
#### Windows and macOS

Install SC2 through [Battle.net](https://www.blizzard.com/en-us/apps/battle.net/desktop).

#### Linux
##### Headfull (Lutris and Wine)

1. Install Lutris from your package manager
2. [Install Battle.net dependencies](https://github.com/lutris/docs/blob/master/Battle.Net.md). (Wine and Vulkan drivers)
3. [Install SC2 through Lutris](https://lutris.net/games/starcraft-ii/)

##### Headless (no graphics)

1. Download most recent [Linux Package](https://github.com/Blizzard/s2client-proto#linux-packages) (Maps will come with the zip)
2. Unzip to ~/StarCraftII (you'll need the End User License Agreement Password above the Linux Packages link)


# Bug Workarounds For Linux (as of 7/31/22)

These solutions will (hopefully) help you run the bot example...

1. The current solution of expanding `~` doesn't work. [I guess it's not that easy](https://stackoverflow.com/questions/54267608/expand-tilde-in-rust-path-idiomatically). So explicitly use the absolute path: `export SC2PATH=/abs/path/to/StarCraftII`.
2. `rust-sc2` doesn't recurse down `Maps` child directories, so you will need to copy whatever `.SC2Map` from the season's to the parent `Maps` directory. (Make sure you update your map in the bot example below)


# Example
The simplest competetive bot in less than 30 lines. Copy this into your `/path/to/project/main.rs`
```rust
use rust_sc2::prelude::*;

#[bot]
#[derive(Default)]
struct WorkerRush;
impl Player for WorkerRush {
	fn get_player_settings(&self) -> PlayerSettings {
		PlayerSettings::new(Race::Protoss)
	}
	fn on_start(&mut self) -> SC2Result<()> {
		for worker in &self.units.my.workers {
			worker.attack(Target::Pos(self.enemy_start), false);
		}
		Ok(())
	}
}

fn main() -> SC2Result<()> {
	let mut bot = WorkerRush::default();
	run_vs_computer(
		&mut bot,
		Computer::new(Race::Random, Difficulty::Medium, None),
		"EternalEmpireLE",
		Default::default(),
	)
}
```
Note: The linux client doesn't have the map `EternalEmpireLE` so you'll need to reference another map from the LadderXXXXSeasonX directories.

## Running Example
### Headfull
As of 8/2/22 this works on Arch Linux, but you need to revert the changes in [this commit](https://github.com/UltraMachine/rust-sc2/commit/321a5e7c768d89f523d8447c4f3f9f161c6dd461) (current HEAD fails to find `icuu52.dll`)

1. `export SC2PATH="/home/<user>/Games/starcraft-ii/drive_c/Program Files (x86)/StarCraft II"`
2. Make sure you have this snippet in your project's **Cargo.toml**:
```toml
[features]
wine_sc2 = ["rust-sc2/wine_sc2"]

```
3. `cargo run --features wine_sc2`

### Headless
1. `export SC2PATH=/abs/path/to/StarCraftII`
2. `cargo run`

For more advanced examples see [`examples`](https://github.com/UltraMachine/rust-sc2/tree/master/examples) folder.

## Optional features
- `"rayon"` - enables parallelism and makes all types threadsafe
- `"serde"` - adds implementation of `Serialize`, `Deserialize` to ids, Race, GameResult, ...
- `"wine_sc2"` - allows you to run headful SC2 through Lutris and Wine

## Making bot step by step
First of all, import rust-sc2 lib:
```rust
use rust_sc2::prelude::*;
```
Create your bot's struct (Can be Unit or C-like):
```rust
#[bot]
struct MyBot;
```
```rust
#[bot]
struct MyBot {
	/* fields here */
}
```
Then implement `Player` trait for your bot:
```rust
// You mustn't call any of these methods by hands, they're for API only
impl Player for MyBot {
	// Must be implemented
	fn get_player_settings(&self) -> PlayerSettings {
		// Race can be Terran, Zerg, Protoss or Random
		PlayerSettings::new(Race::Random)
	}

	// Methods below aren't necessary to implement (Empty by default)

	// Called once on first step
	fn on_start(&mut self) -> SC2Result<()> {
		/* your awesome code here */
	}

	// Called on every game step
	fn on_step(&mut self, iteration: usize) -> SC2Result<()> {
		/* your awesome code here */
	}

	// Called once on last step
	// "result" says if your bot won or lost game
	fn on_end(&self, result: GameResult) -> SC2Result<()> {
		/* your awesome code here */
	}

	// Called on different events, see more in `examples/events.rs`
	fn on_event(&mut self, event: Event) -> SC2Result<()> {
		/* your awesome code here */
	}
}
```
Also you might want to add method to construct it:
```rust
impl MyBot {
	// It's necessary to have #[bot_new] here
	#[bot_new]
	fn new() -> Self {
		Self {
			/* initializing fields */
		}
	}
}
```
If your bot implements `Default` you can simply call `MyBot::default()`, but if you want more control over initializer:
```rust
impl MyBot {
	// You don't need #[bot_new] here, because of "..Default::default()"
	fn new() -> Self {
		Self {
			/* initializing fields */
			..Default::default()
		}
	}
}
```
The rest is to run it:
```rust
fn main() -> SC2Result<()> {
	let mut bot = MyBot::new();
	run_vs_computer(
		&mut bot,
		Computer::new(
			Race::Random,
			Difficulty::VeryEasy,
			None,              // AI Build (random here)
		),
		"EternalEmpireLE", // Map name
		LaunchOptions::default(),
	)
}
```
