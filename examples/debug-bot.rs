use rust_sc2::{geometry::Point3, prelude::*};

mod ex_main;

#[bot]
#[derive(Default)]
struct DebugAI;

impl Player for DebugAI {
	fn on_step(&mut self, _iteration: usize) -> SC2Result<()> {
		// Debug expansion locations
		for exp in self.expansions.clone() {
			let (loc, center) = (exp.loc, exp.center);
			let z = self.get_z_height(loc) + 1.5;
			self.debug.draw_sphere(loc.to3(z), 0.6, Some((255, 128, 255)));
			let z = self.get_z_height(center) + 1.5;
			self.debug.draw_sphere(center.to3(z), 0.5, Some((255, 128, 64)));
		}

		// Debug unit types
		self.units
			.all
			.iter()
			.map(|u| (format!("{:?}", u.type_id()), u.position3d()))
			.collect::<Vec<(String, Point3)>>()
			.into_iter()
			.for_each(|(s, pos)| self.debug.draw_text_world(&s, pos, Some((255, 128, 128)), None));
		Ok(())
	}

	fn get_player_settings(&self) -> PlayerSettings {
		PlayerSettings::new(self.race)
	}
}

fn main() -> SC2Result<()> {
	ex_main::main(DebugAI::default())
}
