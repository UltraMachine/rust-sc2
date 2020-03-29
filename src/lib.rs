#[macro_use]
extern crate num_derive;
#[macro_use]
pub extern crate sc2_macro;

pub mod action;
mod client;
pub mod game_data;
pub mod game_info;
pub mod game_state;
pub mod geometry;
pub mod ids;
mod paths;
pub mod pixel_map;
pub mod player;
pub mod unit;
pub mod units;

use action::{Action, Command};
pub use client::{run_game, run_ladder_game};
use game_data::GameData;
use game_info::GameInfo;
use game_state::GameState;
use player::{AIBuild, Difficulty, PlayerType, Race};
pub use sc2_macro::{bot, bot_impl_player, bot_new};
use std::rc::Rc;

type PlayerBox = Box<dyn Player>;

pub struct PlayerSettings {
	player_type: PlayerType,
	race: Race,
	difficulty: Option<Difficulty>,
	ai_build: Option<AIBuild>,
	name: Option<String>,
}
impl PlayerSettings {
	pub fn new(race: Race, name: Option<String>) -> Self {
		Self {
			player_type: PlayerType::Participant,
			race,
			difficulty: None,
			ai_build: None,
			name,
		}
	}
	pub fn new_human(race: Race, name: Option<String>) -> Self {
		Self {
			player_type: PlayerType::Human,
			race,
			difficulty: None,
			ai_build: None,
			name,
		}
	}
	pub fn new_computer(race: Race, difficulty: Difficulty, ai_build: Option<AIBuild>) -> Self {
		Self {
			player_type: PlayerType::Computer,
			race,
			difficulty: Some(difficulty),
			ai_build,
			name: None,
		}
	}
}

pub trait PlayerClone {
	fn clone_player(&self) -> PlayerBox;
}
impl<T: 'static + Player + Clone> PlayerClone for T {
	fn clone_player(&self) -> PlayerBox {
		Box::new(self.clone())
	}
}

pub trait Player: PlayerClone {
	fn get_player_settings(&self) -> PlayerSettings;
	fn get_step_size(&self) -> u32 {
		1
	}
	fn set_player_id(&mut self, _player_id: u32) {}
	fn set_opponent_id(&mut self, _opponent_id: String) {}
	fn set_game_info(&mut self, _game_info: GameInfo) {}
	fn set_game_data(&mut self, _game_data: GameData) {}
	fn set_state(&mut self, _state: GameState) {}
	fn group_units(&mut self) {}
	fn get_game_data(&self) -> Rc<GameData> {
		unimplemented!()
	}
	fn get_actions(&self) -> Vec<Action> {
		Vec::new()
	}
	fn clear_actions(&mut self) {}
	fn on_step(&mut self, _iteration: usize) {}
	fn command(&mut self, _cmd: Option<Command>) {}
}

impl Clone for PlayerBox {
	fn clone(&self) -> Self {
		self.clone_player()
	}
}

trait FromProto<T>
where
	Self: Sized,
{
	fn from_proto(p: T) -> Self;
}

trait TryFromProto<T>
where
	Self: Sized,
{
	fn try_from_proto(p: T) -> Option<Self>;
}

trait FromProtoPlayer<T>
where
	Self: Sized,
{
	fn from_proto_player(player: &PlayerBox, proto: T) -> Self;
}

trait FromProtoGameData<T>
where
	Self: Sized,
{
	fn from_proto_game_data(game_data: Rc<GameData>, proto: T) -> Self;
}

trait IntoProto<T> {
	fn into_proto(self) -> T;
}
