use crate::{
	geometry::{Point2, Point3},
	ids::UnitTypeId,
	IntoProto,
};
use num_traits::ToPrimitive;
use rustc_hash::FxHashSet;
use sc2_proto::debug::{
	DebugBox, DebugCommand as ProtoDebugCommand, DebugDraw as ProtoDebugDraw, DebugEndGame_EndResult,
	DebugGameState as ProtoDebugGameState, DebugLine, DebugSetUnitValue_UnitValue, DebugSphere, DebugText,
};

type Color = (u32, u32, u32);
type ScreenPos = (f32, f32);

#[derive(Default)]
pub struct Debugger {
	debug_commands: Vec<DebugCommand>,
	debug_drawings: Vec<DebugDraw>,
	kill_tags: FxHashSet<u64>,
}
impl Debugger {
	pub fn get_commands(&mut self) -> &[DebugCommand] {
		let commands = &mut self.debug_commands;

		if !self.debug_drawings.is_empty() {
			commands.push(DebugCommand::Draw(self.debug_drawings.drain(..).collect()));
		}
		if !self.kill_tags.is_empty() {
			commands.push(DebugCommand::KillUnit(self.kill_tags.drain().collect()));
		}

		commands
	}
	pub fn clear_commands(&mut self) {
		self.debug_commands.clear();
	}
	pub fn draw_text(&mut self, text: &str, pos: DebugPos, color: Option<Color>, size: Option<u32>) {
		self.debug_drawings
			.push(DebugDraw::Text(text.to_string(), pos, color, size));
	}
	pub fn draw_text_world(&mut self, text: &str, pos: Point3, color: Option<Color>, size: Option<u32>) {
		self.draw_text(text, DebugPos::World(pos), color, size);
	}
	pub fn draw_text_screen(
		&mut self,
		text: &str,
		pos: Option<ScreenPos>,
		color: Option<Color>,
		size: Option<u32>,
	) {
		self.draw_text(text, DebugPos::Screen(pos.unwrap_or((0.0, 0.0))), color, size);
	}
	pub fn draw_line(&mut self, p0: Point3, p1: Point3, color: Option<Color>) {
		self.debug_drawings.push(DebugDraw::Line(p0, p1, color));
	}
	pub fn draw_box(&mut self, p0: Point3, p1: Point3, color: Option<Color>) {
		self.debug_drawings.push(DebugDraw::Box(p0, p1, color));
	}
	pub fn draw_cube(&mut self, pos: Point3, half_edge: f32, color: Option<Color>) {
		let offset = Point3::new(half_edge, half_edge, half_edge);
		self.debug_drawings
			.push(DebugDraw::Box(pos - offset, pos + offset, color));
	}
	pub fn draw_sphere(&mut self, pos: Point3, radius: f32, color: Option<Color>) {
		self.debug_drawings.push(DebugDraw::Sphere(pos, radius, color));
	}
	pub fn create_units<'a, T>(&mut self, cmds: T)
	where
		T: IntoIterator<Item = &'a (UnitTypeId, Option<u32>, Point2, u32)>,
	{
		self.debug_commands.extend(
			cmds.into_iter()
				.copied()
				.map(|(type_id, owner, pos, count)| DebugCommand::CreateUnit(type_id, owner, pos, count)),
		);
	}
	pub fn kill_units<'a, T: IntoIterator<Item = &'a u64>>(&mut self, tags: T) {
		self.kill_tags.extend(tags);
	}
	pub fn set_unit_values<'a, T>(&mut self, cmds: T)
	where
		T: IntoIterator<Item = &'a (u64, DebugUnitValue, f32)>,
	{
		self.debug_commands.extend(
			cmds.into_iter()
				.copied()
				.map(|(tag, unit_value, value)| DebugCommand::SetUnitValue(tag, unit_value, value)),
		);
	}
	pub fn win_game(&mut self) {
		self.debug_commands.push(DebugCommand::EndGame(true));
	}
	pub fn end_game(&mut self) {
		self.debug_commands.push(DebugCommand::EndGame(false));
	}
	pub fn show_map(&mut self) {
		self.debug_commands
			.push(DebugCommand::GameState(DebugGameState::ShowMap));
	}
	pub fn control_enemy(&mut self) {
		self.debug_commands
			.push(DebugCommand::GameState(DebugGameState::ControlEnemy));
	}
	// Disables supply usage
	pub fn cheat_supply(&mut self) {
		self.debug_commands
			.push(DebugCommand::GameState(DebugGameState::Food));
	}
	// Makes free all units, structures and upgrades
	pub fn cheat_free_build(&mut self) {
		self.debug_commands
			.push(DebugCommand::GameState(DebugGameState::Free));
	}
	// Gives 5000 minerals and gas to the bot
	pub fn cheat_resources(&mut self) {
		self.debug_commands
			.push(DebugCommand::GameState(DebugGameState::AllResources));
	}
	// Gives 5000 minerals to the bot
	pub fn cheat_minerals(&mut self) {
		self.debug_commands
			.push(DebugCommand::GameState(DebugGameState::Minerals));
	}
	// Gives 5000 gas to the bot
	pub fn cheat_gas(&mut self) {
		self.debug_commands
			.push(DebugCommand::GameState(DebugGameState::Gas));
	}
	// Makes all bot's units invincible and significantly increases their damage
	pub fn cheat_god(&mut self) {
		self.debug_commands
			.push(DebugCommand::GameState(DebugGameState::God));
	}
	// Removes cooldown of abilities of bot's units
	pub fn cheat_cooldown(&mut self) {
		self.debug_commands
			.push(DebugCommand::GameState(DebugGameState::Cooldown));
	}
	// Removes all tech requirements for bot
	pub fn cheat_tech_tree(&mut self) {
		self.debug_commands
			.push(DebugCommand::GameState(DebugGameState::TechTree));
	}
	// First use: researches all upgrades for units and sets level 1 of damage and armor upgrades
	// Second use: sets level 2 of damage and armor upgrades
	// Third use: sets level 3 of damage and armor upgrades
	// Fourth use: disables all upgrades researched with this command
	pub fn cheat_upgrades(&mut self) {
		self.debug_commands
			.push(DebugCommand::GameState(DebugGameState::Upgrade));
	}
	// Significantly speeds up making units, structures and upgrades
	pub fn cheat_fast_build(&mut self) {
		self.debug_commands
			.push(DebugCommand::GameState(DebugGameState::FastBuild));
	}
}

#[derive(Debug, Clone)]
pub enum DebugCommand {
	Draw(Vec<DebugDraw>),
	GameState(DebugGameState),
	CreateUnit(UnitTypeId, Option<u32>, Point2, u32),
	KillUnit(Vec<u64>),
	// TestProcess,
	// SetScore,
	EndGame(bool),
	SetUnitValue(u64, DebugUnitValue, f32),
}
impl IntoProto<ProtoDebugCommand> for &DebugCommand {
	fn into_proto(self) -> ProtoDebugCommand {
		let mut proto = ProtoDebugCommand::new();
		match self {
			DebugCommand::Draw(cmds) => proto.set_draw(cmds.into_proto()),
			DebugCommand::GameState(cmd) => proto.set_game_state(cmd.into_proto()),
			DebugCommand::CreateUnit(type_id, owner, pos, count) => {
				let unit = proto.mut_create_unit();
				unit.set_unit_type(type_id.to_u32().unwrap());
				if let Some(owner) = owner {
					unit.set_owner(*owner as i32);
				}
				unit.set_pos(pos.into_proto());
				unit.set_quantity(*count);
			}
			DebugCommand::KillUnit(tags) => proto.mut_kill_unit().set_tag(tags.to_vec()),
			DebugCommand::EndGame(win) => {
				let end_game = proto.mut_end_game();
				if *win {
					end_game.set_end_result(DebugEndGame_EndResult::DeclareVictory);
				}
			}
			DebugCommand::SetUnitValue(tag, unit_value, value) => {
				let cmd = proto.mut_unit_value();
				cmd.set_unit_tag(*tag);
				cmd.set_unit_value(unit_value.into_proto());
				cmd.set_value(*value);
			}
		}
		proto
	}
}

impl IntoProto<ProtoDebugDraw> for &[DebugDraw] {
	fn into_proto(self) -> ProtoDebugDraw {
		let mut cmds = ProtoDebugDraw::new();
		self.iter().for_each(|drawing| match drawing {
			DebugDraw::Text(text, pos, color, size) => {
				let mut proto_text = DebugText::new();
				proto_text.set_text(text.clone());
				match pos {
					DebugPos::Screen((x, y)) => {
						let pos = proto_text.mut_virtual_pos();
						pos.set_x(*x);
						pos.set_y(*y);
					}
					DebugPos::World(p) => proto_text.set_world_pos(p.into_proto()),
				}
				if let Some((r, g, b)) = color {
					let proto_color = proto_text.mut_color();
					proto_color.set_r(*r);
					proto_color.set_g(*g);
					proto_color.set_b(*b);
				}
				if let Some(s) = size {
					proto_text.set_size(*s);
				}
				cmds.mut_text().push(proto_text);
			}
			DebugDraw::Line(p0, p1, color) => {
				let mut proto_line = DebugLine::new();
				let line = proto_line.mut_line();
				line.set_p0(p0.into_proto());
				line.set_p1(p1.into_proto());
				if let Some((r, g, b)) = color {
					let proto_color = proto_line.mut_color();
					proto_color.set_r(*r);
					proto_color.set_g(*g);
					proto_color.set_b(*b);
				}
				cmds.mut_lines().push(proto_line);
			}
			DebugDraw::Box(p0, p1, color) => {
				let mut proto_box = DebugBox::new();
				proto_box.set_min(p0.into_proto());
				proto_box.set_max(p1.into_proto());
				if let Some((r, g, b)) = color {
					let proto_color = proto_box.mut_color();
					proto_color.set_r(*r);
					proto_color.set_g(*g);
					proto_color.set_b(*b);
				}
				cmds.mut_boxes().push(proto_box);
			}
			DebugDraw::Sphere(pos, radius, color) => {
				let mut proto_sphere = DebugSphere::new();
				proto_sphere.set_p(pos.into_proto());
				proto_sphere.set_r(*radius);
				if let Some((r, g, b)) = color {
					let proto_color = proto_sphere.mut_color();
					proto_color.set_r(*r);
					proto_color.set_g(*g);
					proto_color.set_b(*b);
				}
				cmds.mut_spheres().push(proto_sphere);
			}
		});
		cmds
	}
}

#[derive(Debug, Clone)]
pub enum DebugPos {
	Screen(ScreenPos), // Coordinates on screen (0..1, 0..1)
	World(Point3),     // Position in game world
}

#[derive(Debug, Clone)]
pub enum DebugDraw {
	Text(String, DebugPos, Option<Color>, Option<u32>),
	Line(Point3, Point3, Option<Color>),
	Box(Point3, Point3, Option<Color>),
	Sphere(Point3, f32, Option<Color>),
}

#[derive(Debug, Clone, Copy)]
pub enum DebugUnitValue {
	Energy,
	Health,
	Shield,
}
impl IntoProto<DebugSetUnitValue_UnitValue> for DebugUnitValue {
	fn into_proto(self) -> DebugSetUnitValue_UnitValue {
		match self {
			DebugUnitValue::Energy => DebugSetUnitValue_UnitValue::Energy,
			DebugUnitValue::Health => DebugSetUnitValue_UnitValue::Life,
			DebugUnitValue::Shield => DebugSetUnitValue_UnitValue::Shields,
		}
	}
}

#[derive(Debug, Clone, Copy)]
pub enum DebugGameState {
	ShowMap,
	ControlEnemy,
	Food,
	Free,
	AllResources,
	God,
	Minerals,
	Gas,
	Cooldown,
	TechTree,
	Upgrade,
	FastBuild,
}
impl IntoProto<ProtoDebugGameState> for DebugGameState {
	fn into_proto(self) -> ProtoDebugGameState {
		match self {
			DebugGameState::ShowMap => ProtoDebugGameState::show_map,
			DebugGameState::ControlEnemy => ProtoDebugGameState::control_enemy,
			DebugGameState::Food => ProtoDebugGameState::food,
			DebugGameState::Free => ProtoDebugGameState::free,
			DebugGameState::AllResources => ProtoDebugGameState::all_resources,
			DebugGameState::God => ProtoDebugGameState::god,
			DebugGameState::Minerals => ProtoDebugGameState::minerals,
			DebugGameState::Gas => ProtoDebugGameState::gas,
			DebugGameState::Cooldown => ProtoDebugGameState::cooldown,
			DebugGameState::TechTree => ProtoDebugGameState::tech_tree,
			DebugGameState::Upgrade => ProtoDebugGameState::upgrade,
			DebugGameState::FastBuild => ProtoDebugGameState::fast_build,
		}
	}
}
