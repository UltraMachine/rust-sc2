use crate::{
	action::{ActionResult, Target},
	client::{send, SC2Result, WS},
	geometry::Point2,
	ids::AbilityId,
	FromProto, IntoProto,
};
use num_traits::ToPrimitive;
use sc2_proto::{
	query::{RequestQueryBuildingPlacement, RequestQueryPathing},
	sc2api::Request,
};

#[derive(Default, Clone)]
pub struct QueryMaster;

impl QueryMaster {
	pub fn pathing(&self, ws: &mut WS, paths: Vec<(Target, Point2)>) -> SC2Result<Vec<Option<f32>>> {
		let mut req = Request::new();
		let req_pathing = req.mut_query().mut_pathing();

		paths.iter().for_each(|(start, goal)| {
			let mut pathing = RequestQueryPathing::new();
			match start {
				Target::Tag(tag) => pathing.set_unit_tag(*tag),
				Target::Pos(pos) => pathing.set_start_pos(pos.into_proto()),
				Target::None => panic!("start pos is not specified in query pathing request"),
			}
			pathing.set_end_pos(goal.into_proto());
			req_pathing.push(pathing);
		});

		let res = send(ws, req)?;
		Ok(res
			.get_query()
			.get_pathing()
			.iter()
			.map(|result| result.distance)
			.collect())
	}
	pub fn placement(
		&self,
		ws: &mut WS,
		places: Vec<(AbilityId, Point2, Option<u64>)>,
		check_resources: bool,
	) -> SC2Result<Vec<ActionResult>> {
		let mut req = Request::new();
		let req_query = req.mut_query();
		req_query.set_ignore_resource_requirements(!check_resources);
		let req_placement = req_query.mut_placements();

		places.iter().for_each(|(ability, pos, builder)| {
			let mut placement = RequestQueryBuildingPlacement::new();
			placement.set_ability_id(ability.to_i32().unwrap());
			placement.set_target_pos(pos.into_proto());
			if let Some(tag) = builder {
				placement.set_placing_unit_tag(*tag);
			}
			req_placement.push(placement);
		});

		let res = send(ws, req)?;
		Ok(res
			.get_query()
			.get_placements()
			.iter()
			.map(|result| ActionResult::from_proto(result.get_result()))
			.collect())
	}
}
