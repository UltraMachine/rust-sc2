use crate::{geometry::Point3, IntoProto};
use sc2_proto::debug::{
	DebugBox, DebugCommand as ProtoDebugCommand, DebugDraw as ProtoDebugDraw, DebugLine, DebugSphere,
	DebugText,
};

type Color = (u32, u32, u32);
type ScreenPos = (f32, f32);

#[derive(Default, Clone)]
pub struct Debugger {
	debug_commands: Vec<DebugCommand>,
	debug_drawings: Vec<DebugDraw>,
}
impl Debugger {
	pub fn get_commands(&self) -> Vec<DebugCommand> {
		if !self.debug_drawings.is_empty() {
			let mut commands = self.debug_commands.clone();
			commands.push(DebugCommand::Draw(self.debug_drawings.clone()));
			commands
		} else {
			self.debug_commands.clone()
		}
	}
	pub fn clear_commands(&mut self) {
		self.debug_commands.clear();
		self.debug_drawings.clear();
	}
	pub fn draw_text(&mut self, text: String, pos: DebugPos, color: Option<Color>, size: Option<u32>) {
		self.debug_drawings.push(DebugDraw::Text(text, pos, color, size));
	}
	pub fn draw_text_world(&mut self, text: String, pos: Point3, color: Option<Color>, size: Option<u32>) {
		self.draw_text(text, DebugPos::World(pos), color, size);
	}
	pub fn draw_text_screen(
		&mut self,
		text: String,
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
}

#[derive(Debug, Clone)]
pub enum DebugCommand {
	Draw(Vec<DebugDraw>),
	// GameState,
	// CreateUnit,
	// KillUnit,
	// TestProcess,
	// SetScore,
	// EndGame,
	// SetUnitValue,
}
impl IntoProto<ProtoDebugCommand> for DebugCommand {
	fn into_proto(self) -> ProtoDebugCommand {
		let mut proto = ProtoDebugCommand::new();
		match self {
			DebugCommand::Draw(cmds) => proto.set_draw(cmds.into_proto()),
		}
		proto
	}
}

impl IntoProto<ProtoDebugDraw> for Vec<DebugDraw> {
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
