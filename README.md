# rust-sc2
[![crates.io](https://img.shields.io/crates/v/rust-sc2.svg)](https://crates.io/crates/rust-sc2)
[![Documentation](https://docs.rs/rust-sc2/badge.svg)](https://docs.rs/rust-sc2)

Rust implementation of StarCraft II API

The library aims to be simple and easy to use, being very fast and functional at the same time. However, it provides both high and low level abstractions. This lib is inspired by [python-sc2](https://github.com/BurnySc2/python-sc2) lib, so people might find it easy to switch to rust-sc2. It was originally created because other rust libs were old, not functional and low level.

Feel free to ask questions in `#rust` channel of these Discord servers:
- [Starcraft 2 AI](https://discord.gg/Emm5Ztz)
- [AI Arena](https://discord.gg/yDBzbtC)

# Getting started
Install Rust >= 1.42.0

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

The simplest competetive bot in less than 30 lines:
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

For more advanced examples see [`examples`](https://github.com/UltraMachine/rust-sc2/tree/master/examples) folder.

## Optional features
- `"rayon"` - enables parallelism and makes all types threadsafe
- `"serde"` - adds implementation of `Serialize`, `Deserialize` to ids, Race, GameResult, ...

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
