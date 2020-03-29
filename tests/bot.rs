#[macro_use]
extern crate sc2_macro;

use rust_sc2::{player::Race, Player, PlayerSettings};

#[bot]
#[derive(Clone)]
struct MyBot {
	name: String,
	my_id: u32,
}

impl MyBot {
	#[bot_new]
	fn new(name: Option<String>) -> Self {
		let name = match name {
			Some(n) => n,
			None => "MyBot".to_string(),
		};
		Self {
			name,
			my_id: 12345,
			game_step: 1 + 1,
		}
	}
}

#[bot_impl_player]
impl Player for MyBot {
	fn get_player_settings(&self) -> PlayerSettings {
		PlayerSettings::new(Race::Random, None)
	}
}
