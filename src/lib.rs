#[macro_use]
extern crate num_derive;
#[macro_use]
extern crate lazy_static;
#[macro_use]
pub extern crate sc2_macro;
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
		client::{run_ladder_game, run_vs_computer, run_vs_human, LaunchOptions, SC2Result},
		constants::{ALL_PRODUCERS, PRODUCERS, RESEARCHERS, TECH_REQUIREMENTS},
		distance::*,
		geometry::Point2,
		ids::*,
		player::{AIBuild, Computer, Difficulty, GameResult, Race},
		sc2_macro::{bot, bot_new},
		unit::Unit,
		units::Units,
		Event, Player, PlayerSettings,
	};
}

mod api;
mod client;
mod debug;
mod game_info;
mod paths;
mod ramp;
mod score;

pub mod action;
pub mod bot;
pub mod constants;
pub mod distance;
pub mod game_data;
pub mod game_state;
pub mod geometry;
pub mod ids;
pub mod pixel_map;
pub mod player;
pub mod unit;
pub mod units;
pub mod utils;

use player::{GameResult, Race};
use unit::SharedUnitData;

pub use client::{run_ladder_game, run_vs_computer, run_vs_human, SC2Result};
pub use sc2_proto::sc2api::Request;

pub type PlayerBox = Box<dyn Player>;

pub struct PlayerSettings {
	race: Race,
	name: Option<String>,
	raw_affects_selection: bool,
}
impl PlayerSettings {
	pub fn new(race: Race, name: Option<&str>) -> Self {
		Self {
			race,
			name: name.map(|n| n.to_string()),
			raw_affects_selection: false,
		}
	}
	pub fn configured(race: Race, name: Option<&str>, raw_affects_selection: bool) -> Self {
		Self {
			race,
			name: name.map(|n| n.to_string()),
			raw_affects_selection,
		}
	}
}

pub enum Event {
	UnitDestroyed(u64),
	UnitCreated(u64),
	ConstructionStarted(u64),
	ConstructionComplete(u64),
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
	fn on_event(&mut self, _event: Event) -> SC2Result<()> {
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
	fn from_proto_data(data: SharedUnitData, proto: T) -> Self;
}

trait IntoProto<T> {
	fn into_proto(self) -> T;
}

/*trait FromSC2<T> {
	fn from_sc2(s: T) -> Self;
}
impl<T, U: IntoProto<T>> FromSC2<U> for T {
	fn from_sc2(s: U) -> T {
		s.into_proto()
	}
}*/
