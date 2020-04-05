use crate::{
	action::{Action, ActionError},
	geometry::Point2,
	ids::{AbilityId, EffectId, UpgradeId},
	pixel_map::{PixelMap, VisibilityMap},
	unit::Unit,
	units::Units,
	FromProto, FromProtoData, FromProtoPlayer, PlayerBox,
};
use num_traits::FromPrimitive;
use sc2_proto::{
	raw::{Alliance as ProtoAlliance, ObservationRaw, PowerSource as ProtoPowerSource},
	sc2api::{Alert as ProtoAlert, Observation as ProtoObservation, ResponseObservation},
};
use std::rc::Rc;

#[derive(Default, Clone)]
pub struct GameState {
	pub actions: Vec<Action>,
	pub action_errors: Vec<ActionError>,
	pub observation: Observation,
	// player_result,
	pub chat: Vec<ChatMessage>,
}
impl FromProtoPlayer<&ResponseObservation> for GameState {
	fn from_proto_player(player: Rc<PlayerBox>, response_observation: &ResponseObservation) -> Self {
		// let player_result = response_observation.get_player_result();
		Self {
			actions: response_observation
				.get_actions()
				.iter()
				.filter_map(|a| Option::<Action>::from_proto(a.clone()))
				.collect(),
			action_errors: response_observation
				.get_action_errors()
				.iter()
				.map(|e| ActionError::from_proto(e.clone()))
				.collect(),
			observation: Observation::from_proto_player(
				player,
				response_observation.get_observation().clone(),
			),
			chat: response_observation
				.get_chat()
				.iter()
				.map(|m| ChatMessage {
					player_id: m.get_player_id(),
					message: m.get_message().to_string(),
				})
				.collect(),
		}
	}
}

#[derive(Clone)]
pub struct ChatMessage {
	pub player_id: u32,
	pub message: String,
}

#[derive(Default, Clone)]
pub struct Observation {
	pub game_loop: u32,
	pub common: Common,
	pub alerts: Vec<Alert>,
	pub abilities: Vec<AvailableAbility>,
	// pub score: Score,
	pub raw: RawData,
}
impl FromProtoPlayer<ProtoObservation> for Observation {
	fn from_proto_player(player: Rc<PlayerBox>, obs: ProtoObservation) -> Self {
		let common = obs.get_player_common();
		Self {
			game_loop: obs.get_game_loop(),
			common: Common {
				player_id: common.get_player_id(),
				minerals: common.get_minerals(),
				vespene: common.get_vespene(),
				food_cap: common.get_food_cap(),
				food_used: common.get_food_used(),
				food_army: common.get_food_army(),
				food_workers: common.get_food_workers(),
				idle_worker_count: common.get_idle_worker_count(),
				army_count: common.get_army_count(),
				warp_gate_count: common.get_warp_gate_count(),
				larva_count: common.get_larva_count(),
			},
			alerts: obs.get_alerts().iter().map(|a| Alert::from_proto(*a)).collect(),
			abilities: obs
				.get_abilities()
				.iter()
				.map(|a| AvailableAbility {
					id: AbilityId::from_i32(a.get_ability_id()).unwrap(),
					requires_point: a.get_requires_point(),
				})
				.collect(),
			raw: RawData::from_proto_player(player, obs.get_raw_data().clone()),
		}
	}
}

#[derive(Default, Clone)]
pub struct RawData {
	pub psionic_matrix: Vec<PsionicMatrix>,
	pub camera: Point2,
	pub units: Units,
	pub upgrades: Vec<UpgradeId>,
	pub visibility: VisibilityMap,
	pub creep: PixelMap,
	pub dead_units: Vec<u64>,
	pub effects: Vec<Effect>,
	pub radars: Vec<Radar>,
}
impl FromProtoPlayer<ObservationRaw> for RawData {
	fn from_proto_player(player: Rc<PlayerBox>, raw: ObservationRaw) -> Self {
		let raw_player = raw.get_player();
		let map_state = raw.get_map_state();
		Self {
			psionic_matrix: raw_player
				.get_power_sources()
				.iter()
				.map(|ps| PsionicMatrix::from_proto(ps.clone()))
				.collect(),
			camera: Point2::from_proto(raw_player.get_camera().clone()),
			units: raw
				.get_units()
				.iter()
				.map(|u| Unit::from_proto_data(player.get_data_for_unit(), u.clone()))
				.collect(),
			upgrades: raw_player
				.get_upgrade_ids()
				.iter()
				.map(|u| UpgradeId::from_u32(*u).unwrap())
				.collect(),
			visibility: VisibilityMap::from_proto(map_state.get_visibility().clone()),
			creep: PixelMap::from_proto(map_state.get_creep().clone()),
			dead_units: raw.get_event().get_dead_units().to_vec(),
			effects: raw
				.get_effects()
				.iter()
				.map(|e| Effect {
					id: EffectId::from_u32(e.get_effect_id()).unwrap(),
					positions: e
						.get_pos()
						.iter()
						.map(|p| Point2::from_proto(p.clone()))
						.collect(),
					alliance: Alliance::from_proto(e.get_alliance()),
					owner: e.get_owner() as u32,
					radius: e.get_radius(),
				})
				.collect(),
			radars: raw
				.get_radar()
				.iter()
				.map(|r| Radar {
					pos: Point2::from_proto(r.get_pos().clone()),
					radius: r.get_radius(),
				})
				.collect(),
		}
	}
}

#[derive(Clone)]
pub struct PsionicMatrix {
	pub pos: Point2,
	pub radius: f32,
	pub tag: u64,
}
impl FromProto<ProtoPowerSource> for PsionicMatrix {
	fn from_proto(ps: ProtoPowerSource) -> Self {
		Self {
			pos: Point2::from_proto(ps.get_pos().clone()),
			radius: ps.get_radius(),
			tag: ps.get_tag(),
		}
	}
}

#[derive(Clone)]
pub struct Effect {
	pub id: EffectId,
	pub positions: Vec<Point2>,
	pub alliance: Alliance,
	pub owner: u32,
	pub radius: f32,
}

#[derive(Clone, PartialEq, Eq)]
pub enum Alliance {
	Own,
	Ally,
	Neutral,
	Enemy,
}
impl FromProto<ProtoAlliance> for Alliance {
	fn from_proto(alliance: ProtoAlliance) -> Self {
		match alliance {
			ProtoAlliance::value_Self => Alliance::Own,
			ProtoAlliance::Ally => Alliance::Ally,
			ProtoAlliance::Neutral => Alliance::Neutral,
			ProtoAlliance::Enemy => Alliance::Enemy,
		}
	}
}

#[derive(Clone)]
pub struct Radar {
	pub pos: Point2,
	pub radius: f32,
}

#[derive(Default, Clone)]
pub struct Common {
	pub player_id: u32,
	pub minerals: u32,
	pub vespene: u32,
	pub food_cap: u32,
	pub food_used: u32,
	pub food_army: u32,
	pub food_workers: u32,
	pub idle_worker_count: u32,
	pub army_count: u32,
	pub warp_gate_count: u32,
	pub larva_count: u32,
}

#[allow(clippy::enum_variant_names)]
#[derive(Clone)]
pub enum Alert {
	AlertError,
	AddOnComplete,
	BuildingComplete,
	BuildingUnderAttack,
	LarvaHatched,
	MergeComplete,
	MineralsExhausted,
	MorphComplete,
	MothershipComplete,
	MULEExpired,
	NuclearLaunchDetected,
	NukeComplete,
	NydusWormDetected,
	ResearchComplete,
	TrainError,
	TrainUnitComplete,
	TrainWorkerComplete,
	TransformationComplete,
	UnitUnderAttack,
	UpgradeComplete,
	VespeneExhausted,
	WarpInComplete,
}
impl FromProto<ProtoAlert> for Alert {
	fn from_proto(alert: ProtoAlert) -> Self {
		match alert {
			ProtoAlert::AlertError => Alert::AlertError,
			ProtoAlert::AddOnComplete => Alert::AddOnComplete,
			ProtoAlert::BuildingComplete => Alert::BuildingComplete,
			ProtoAlert::BuildingUnderAttack => Alert::BuildingUnderAttack,
			ProtoAlert::LarvaHatched => Alert::LarvaHatched,
			ProtoAlert::MergeComplete => Alert::MergeComplete,
			ProtoAlert::MineralsExhausted => Alert::MineralsExhausted,
			ProtoAlert::MorphComplete => Alert::MorphComplete,
			ProtoAlert::MothershipComplete => Alert::MothershipComplete,
			ProtoAlert::MULEExpired => Alert::MULEExpired,
			ProtoAlert::NuclearLaunchDetected => Alert::NuclearLaunchDetected,
			ProtoAlert::NukeComplete => Alert::NukeComplete,
			ProtoAlert::NydusWormDetected => Alert::NydusWormDetected,
			ProtoAlert::ResearchComplete => Alert::ResearchComplete,
			ProtoAlert::TrainError => Alert::TrainError,
			ProtoAlert::TrainUnitComplete => Alert::TrainUnitComplete,
			ProtoAlert::TrainWorkerComplete => Alert::TrainWorkerComplete,
			ProtoAlert::TransformationComplete => Alert::TransformationComplete,
			ProtoAlert::UnitUnderAttack => Alert::UnitUnderAttack,
			ProtoAlert::UpgradeComplete => Alert::UpgradeComplete,
			ProtoAlert::VespeneExhausted => Alert::VespeneExhausted,
			ProtoAlert::WarpInComplete => Alert::WarpInComplete,
		}
	}
}

#[derive(Clone)]
pub struct AvailableAbility {
	pub id: AbilityId,
	pub requires_point: bool,
}
