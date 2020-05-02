use crate::{
	geometry::{Point2, Rect, Size},
	pixel_map::{ByteMap, PixelMap},
	player::{AIBuild, Difficulty, PlayerType, Race},
	FromProto,
};
use sc2_proto::sc2api::ResponseGameInfo;
use std::{collections::HashMap, path::Path};

#[derive(Default, Clone)]
pub struct GameInfo {
	pub map_name: String,      // Depends on sc2 localization
	pub map_name_path: String, // Depends on file name
	pub mod_names: Vec<String>,
	pub local_map_path: String,
	pub players: HashMap<u32, PlayerInfo>,
	pub map_size: Size,
	pub pathing_grid: PixelMap,
	pub terrain_height: ByteMap,
	pub placement_grid: PixelMap,
	pub playable_area: Rect,
	pub start_locations: Vec<Point2>,
	pub map_center: Point2,
}
impl FromProto<ResponseGameInfo> for GameInfo {
	fn from_proto(game_info: ResponseGameInfo) -> Self {
		let start_raw = game_info.get_start_raw();
		let map_size = start_raw.get_map_size();
		let area = start_raw.get_playable_area();
		let area_p0 = area.get_p0();
		let area_p1 = area.get_p1();
		let area_p0_x = area_p0.get_x();
		let area_p0_y = area_p0.get_y();
		let area_p1_x = area_p1.get_x();
		let area_p1_y = area_p1.get_y();
		let local_map_path = game_info.get_local_map_path().to_string();
		Self {
			map_name: game_info.get_map_name().to_string(),
			mod_names: game_info.get_mod_names().to_vec(),
			map_name_path: Path::new(&local_map_path)
				.file_stem()
				.unwrap()
				.to_str()
				.unwrap()
				.to_string(),
			local_map_path,
			players: game_info
				.get_player_info()
				.iter()
				.map(|i| {
					let id = i.get_player_id();
					(
						id,
						PlayerInfo {
							id,
							player_type: PlayerType::from_proto(i.get_field_type()),
							race_requested: Race::from_proto(i.get_race_requested()),
							race_actual: i.race_actual.map(Race::from_proto),
							difficulty: i.difficulty.map(Difficulty::from_proto),
							ai_build: i.ai_build.map(AIBuild::from_proto),
							player_name: i.player_name.clone().into_option(),
						},
					)
				})
				.collect(),
			map_size: Size::new(map_size.get_x() as usize, map_size.get_y() as usize),
			pathing_grid: PixelMap::from_proto(start_raw.get_pathing_grid().clone()),
			terrain_height: ByteMap::from_proto(start_raw.get_terrain_height().clone()),
			placement_grid: PixelMap::from_proto(start_raw.get_placement_grid().clone()),
			playable_area: Rect::new(
				area_p0_x as usize,
				area_p0_y as usize,
				area_p1_x as usize,
				area_p1_y as usize,
			),
			start_locations: start_raw
				.get_start_locations()
				.iter()
				.map(|p| Point2::from_proto(p.clone()))
				.collect(),
			map_center: Point2::new(
				(area_p0_x + (area_p1_x - area_p0_x) / 2) as f32,
				(area_p0_y + (area_p1_y - area_p0_y) / 2) as f32,
			),
		}
	}
}

#[derive(Clone)]
pub struct PlayerInfo {
	pub id: u32,
	pub player_type: PlayerType,
	pub race_requested: Race,
	pub race_actual: Option<Race>,
	pub difficulty: Option<Difficulty>,
	pub ai_build: Option<AIBuild>,
	pub player_name: Option<String>,
}
