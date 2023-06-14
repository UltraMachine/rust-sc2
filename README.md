<!-- markdown-toc start - Don't edit this section. Run M-x markdown-toc-refresh-toc -->
**Table of Contents**

- [rust-sc2](#rust-sc2)
- [Getting started](#getting-started)
    - [Rust](#rust)
    - [StarCraft II](#starcraft-ii)
        - [Installation](#installation)
            - [Windows and macOS](#windows-and-macos)
            - [Linux](#linux)
                - [Headfull (Lutris and Wine)](#headfull-lutris-and-wine)
                - [Headless (no graphics)](#headless-no-graphics)
- [Example](#example)
    - [Running Example](#running-example)
        - [Headfull](#headfull)
        - [Headless](#headless)
    - [Runnint the advanced examples](#running-the-advanced-examples)
    - [Optional features](#optional-features)
    - [Making bot step by step](#making-bot-step-by-step)

<!-- markdown-toc end -->


# rust-sc2
[![crates.io](https://img.shields.io/crates/v/rust-sc2.svg)](https://crates.io/crates/rust-sc2)
[![Documentation](https://docs.rs/rust-sc2/badge.svg)](https://docs.rs/rust-sc2)

Rust implementation of StarCraft II API

The library aims to be simple and easy to use, being very fast and functional at the same time. However, it provides both high and low level abstractions. This lib is inspired by [python-sc2](https://github.com/BurnySc2/python-sc2) lib, so people might find it easy to switch to rust-sc2. It was originally created because other rust libs were old, not functional and low level.

Feel free to ask questions in `#rust` channel of [Starcraft 2 AI Discord](https://discord.gg/Emm5Ztz) server

# Getting started
## Rust
[Install latest stable Rust](https://www.rust-lang.org/tools/install)
(Older versions may also work, but compatibility is not guaranteed)

Create your project

`cargo new <project_name>`

Add to dependencies in Cargo.toml:
```toml
[dependencies]
rust-sc2 = "1"
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

**NOTE:** *Version of this library on crates.io is outdated and lacks many features. Unfortunately, I can't update it yet, so it's highly recommended to use github version for now.*

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
3. Move your `.SC2Map` files up out of their `LadderXXXXSeasonX` directory to `Maps` directory. (Since there are multiple versions of the same map, there is no way of knowing which one you want.)


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
Note: The Linux client doesn't have the map `EternalEmpireLE` so you'll need to download it, or reference another map from the `LadderXXXXSeasonX` directories.

## Running Example
### Headfull
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

## Running the advanced examples
There are more advanced examples in the [`examples`](https://github.com/UltraMachine/rust-sc2/tree/master/examples) folder. To run one of these examples on your own machine, say the Reaper Rush one, clone this repository, navigate to the root folder and run the command
```
cargo run --example reaper-rush -- local
```
In addition to `local` (or `human` if you want to play against your own bot), these examples take several arguments. For a full list in either case, run the commands
```
cargo run --example reaper-rush -- local --help
cargo run --example reaper-rush -- human --help
```


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
            None, // AI Build (random here)
        ),
        "EternalEmpireLE", // Map name
        LaunchOptions::default(),
    )
}
```
