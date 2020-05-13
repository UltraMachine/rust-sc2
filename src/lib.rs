#[macro_use]
extern crate num_derive;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate sc2_macro;
#[macro_use]
extern crate itertools;
#[macro_use]
extern crate maplit;
#[macro_use]
extern crate log;

pub mod prelude {
	pub use crate::{
		action::Target,
		bot::PlacementOptions,
		client::{run_ladder_game, run_vs_computer, run_vs_human, SC2Result},
		constants::TECH_REQUIREMENTS,
		geometry::Point2,
		ids::*,
		player::{AIBuild, Computer, Difficulty, GameResult, Race},
		sc2_macro::{bot, bot_new},
		unit::Unit,
		units::Units,
		Player, PlayerSettings,
	};
}

mod api;
mod client;
mod debug;
mod game_info;
mod paths;

pub mod action;
pub mod bot;
pub mod constants;
pub mod game_data;
pub mod game_state;
pub mod geometry;
pub mod ids;
pub mod pixel_map;
pub mod player;
pub mod unit;
pub mod units;

use player::{GameResult, Race};
use std::rc::Rc;
use unit::DataForUnit;

pub use client::{run_ladder_game, run_vs_computer, run_vs_human, SC2Result};
pub use sc2_macro::{bot, bot_new};

pub type PlayerBox = Box<dyn Player>;

pub struct PlayerSettings {
	race: Race,
	name: Option<String>,
	raw_affects_selection: bool,
}
impl PlayerSettings {
	pub fn new(race: Race, name: Option<String>) -> Self {
		Self {
			race,
			name,
			raw_affects_selection: false,
		}
	}
	pub fn configured(race: Race, name: Option<String>, raw_affects_selection: bool) -> Self {
		Self {
			race,
			name,
			raw_affects_selection,
		}
	}
}

pub trait Player {
	fn get_player_settings(&self) -> PlayerSettings;
	fn on_start(&mut self) -> SC2Result<()> {
		Ok(())
	}
	fn on_step(&mut self, _iteration: usize) -> SC2Result<()> {
		Ok(())
	}
	fn on_end(&self, _result: GameResult) -> SC2Result<()> {
		Ok(())
	}
}

trait FromProto<T>
where
	Self: Sized,
{
	fn from_proto(p: T) -> Self;
}

trait IntoSC2<T> {
	fn into_sc2(self) -> T;
}
impl<T, U: FromProto<T>> IntoSC2<U> for T {
	fn into_sc2(self) -> U {
		U::from_proto(self)
	}
}

trait TryFromProto<T>
where
	Self: Sized,
{
	fn try_from_proto(p: T) -> Option<Self>;
}

trait FromProtoData<T>
where
	Self: Sized,
{
	fn from_proto_data(data: Rc<DataForUnit>, proto: T) -> Self;
}

trait IntoProto<T> {
	fn into_proto(self) -> T;
}
