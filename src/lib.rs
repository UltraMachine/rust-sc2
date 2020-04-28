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

mod client;
mod paths;

pub mod action;
pub mod bot;
pub mod constants;
pub mod debug;
pub mod game_data;
pub mod game_info;
pub mod game_state;
pub mod geometry;
pub mod ids;
pub mod pixel_map;
pub mod player;
pub mod query;
pub mod unit;
pub mod units;

use player::Race;
use std::rc::Rc;
use unit::DataForUnit;

pub use client::{run_ladder_game, run_vs_computer, run_vs_human, SC2Result, WS};
pub use itertools::{iproduct, Itertools};
pub use sc2_macro::{bot, bot_new};

pub type PlayerBox = Box<dyn Player>;

pub struct PlayerSettings {
	race: Race,
	name: Option<String>,
}
impl PlayerSettings {
	pub fn new(race: Race, name: Option<String>) -> Self {
		Self { race, name }
	}
}

pub trait Player {
	fn get_player_settings(&self) -> PlayerSettings;
	fn on_start(&mut self, _ws: &mut WS) -> SC2Result<()> {
		Ok(())
	}
	fn on_step(&mut self, _ws: &mut WS, _iteration: usize) -> SC2Result<()> {
		Ok(())
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

trait FromProtoData<T>
where
	Self: Sized,
{
	fn from_proto_data(data: Rc<DataForUnit>, proto: T) -> Self;
}

trait IntoProto<T> {
	fn into_proto(self) -> T;
}
