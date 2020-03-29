use crate::{
	geometry::{Point2, Rect, Size},
	pixel_map::{ByteMap, PixelMap},
	player::{AIBuild, Difficulty, PlayerType, Race},
	FromProto,
};
use sc2_proto::sc2api::ResponseGameInfo;

#[derive(Default, Clone)]
pub struct GameInfo {
	pub map_name: String,
	pub mod_names: Vec<String>,
	pub local_map_path: String,
	pub players: Vec<PlayerInfo>,
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
		/*
		println!("Pathing Grid");
		let grid = start_raw.get_pathing_grid().clone();
		println!("size: {:?}", grid.get_size());
		println!("lenght: {:?}", grid.get_data().len());
		println!("data: {:?}", grid.get_data());
		println!(
			"binary data: {:?}",
			grid.get_data()
				.iter()
				.flat_map(|n| to_binary(*n))
				.collect::<Vec<Pixel>>()
		);
		println!("Placement Grid");
		let grid = start_raw.get_placement_grid().clone();
		println!("size: {:?}", grid.get_size());
		println!("lenght: {:?}", grid.get_data().len());
		println!("data: {:?}", grid.get_data());
		println!(
			"binary data: {:?}",
			grid.get_data()
				.iter()
				.flat_map(|n| to_binary(*n))
				.collect::<Vec<Pixel>>()
		);
		*/
		let map_size = start_raw.get_map_size();
		let area = start_raw.get_playable_area();
		let area_p0 = area.get_p0();
		let area_p1 = area.get_p1();
		let area_p0_x = area_p0.get_x();
		let area_p0_y = area_p0.get_y();
		let area_p1_x = area_p1.get_x();
		let area_p1_y = area_p1.get_y();
		Self {
			map_name: game_info.get_map_name().to_string(),
			mod_names: game_info.get_mod_names().to_vec(),
			local_map_path: game_info.get_local_map_path().to_string(),
			players: game_info
				.get_player_info()
				.iter()
				.map(|i| PlayerInfo {
					id: {
						if i.has_player_id() {
							Some(i.get_player_id())
						} else {
							None
						}
					},
					player_type: {
						if i.has_field_type() {
							Some(PlayerType::from_proto(i.get_field_type()))
						} else {
							None
						}
					},
					race_requested: {
						if i.has_race_requested() {
							Some(Race::from_proto(i.get_race_requested()))
						} else {
							None
						}
					},
					race_actual: {
						if i.has_race_actual() {
							Some(Race::from_proto(i.get_race_actual()))
						} else {
							None
						}
					},
					difficulty: {
						if i.has_difficulty() {
							Some(Difficulty::from_proto(i.get_difficulty()))
						} else {
							None
						}
					},
					ai_build: {
						if i.has_ai_build() {
							Some(AIBuild::from_proto(i.get_ai_build()))
						} else {
							None
						}
					},
					player_name: {
						if i.has_player_name() {
							Some(i.get_player_name().to_string())
						} else {
							None
						}
					},
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
	pub id: Option<u32>,
	pub player_type: Option<PlayerType>,
	pub race_requested: Option<Race>,
	pub race_actual: Option<Race>,
	pub difficulty: Option<Difficulty>,
	pub ai_build: Option<AIBuild>,
	pub player_name: Option<String>,
}
