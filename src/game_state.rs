//! Information updated every step stored here.

use crate::{
	action::{Action, ActionError},
	bot::{Bot, Locked, Rs, Rw},
	geometry::Point2,
	ids::*,
	pixel_map::{PixelMap, VisibilityMap},
	score::Score,
	unit::Unit,
	units::Units,
	Event, FromProto, Player, SC2Result,
};
use num_traits::FromPrimitive;
use rustc_hash::FxHashSet;
use sc2_proto::{
	query::RequestQueryAvailableAbilities,
	raw::{Alliance as ProtoAlliance, PowerSource as ProtoPowerSource},
	sc2api::{Alert as ProtoAlert, Request, ResponseObservation},
};
use std::ops::{Deref, DerefMut};

/// Information about current state on current step.
/// Can be accessed through [`state`](crate::bot::Bot::state) field.
#[derive(Default, Clone)]
pub struct GameState {
	/// Actions executed on previous step.
	pub actions: Vec<Action>,
	/// Results on actions from previous step.
	pub action_errors: Vec<ActionError>,
	pub observation: Observation,
	// player_result,
	/// Messeges in game chat.
	pub chat: Vec<ChatMessage>,
}

pub(crate) fn update_state<B>(bot: &mut B, response_observation: &ResponseObservation) -> SC2Result<()>
where
	B: Player + DerefMut<Target = Bot> + Deref<Target = Bot>,
{
	// Game state
	let state = &mut bot.state;

	// let player_result = response_observation.get_player_result();
	state.actions = response_observation
		.get_actions()
		.iter()
		.filter_map(|a| Option::<Action>::from_proto(a))
		.collect();
	state.action_errors = response_observation
		.get_action_errors()
		.iter()
		.map(|e| ActionError::from_proto(e))
		.collect();
	state.chat = response_observation
		.get_chat()
		.iter()
		.map(|m| ChatMessage {
			player_id: m.get_player_id(),
			message: m.get_message().to_string(),
		})
		.collect();

	// Observation
	let obs = &mut state.observation;
	let res_obs = response_observation.get_observation();

	obs.game_loop = res_obs.get_game_loop();
	obs.alerts = res_obs
		.get_alerts()
		.iter()
		.map(|a| Alert::from_proto(*a))
		.collect();
	obs.abilities = res_obs
		.get_abilities()
		.iter()
		.map(|a| AvailableAbility {
			id: AbilityId::from_i32(a.get_ability_id()).unwrap(),
			requires_point: a.get_requires_point(),
		})
		.collect();
	obs.score = Score::from_proto(res_obs.get_score());

	// Common
	let common = res_obs.get_player_common();
	obs.common = Common {
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
	};

	// Raw
	let raw = &mut obs.raw;
	let res_raw = res_obs.get_raw_data();

	let raw_player = res_raw.get_player();
	raw.psionic_matrix = raw_player
		.get_power_sources()
		.iter()
		.map(|ps| PsionicMatrix::from_proto(ps))
		.collect();
	raw.camera = Point2::from_proto(raw_player.get_camera());
	raw.effects = res_raw
		.get_effects()
		.iter()
		.map(|e| Effect {
			id: EffectId::from_u32(e.get_effect_id()).unwrap(),
			positions: e.get_pos().iter().map(Point2::from_proto).collect(),
			alliance: Alliance::from_proto(e.get_alliance()),
			owner: e.get_owner() as u32,
			radius: e.get_radius(),
		})
		.collect();
	raw.radars = res_raw
		.get_radar()
		.iter()
		.map(|r| Radar {
			pos: Point2::from_proto(r.get_pos()),
			radius: r.get_radius(),
		})
		.collect();

	// Dead units
	let dead_units = res_raw.get_event().get_dead_units().to_vec();

	let enemy_is_terran = bot.enemy_race.is_terran();
	for u in &dead_units {
		let cache = &mut bot.units.cached;
		cache.all.remove(*u);
		cache.units.remove(*u);
		cache.workers.remove(*u);
		if enemy_is_terran {
			cache.structures.remove(*u);
			cache.townhalls.remove(*u);
		}

		bot.saved_hallucinations.remove(u);

		bot.on_event(Event::UnitDestroyed(*u))?;
	}

	let raw = &mut bot.state.observation.raw;
	raw.dead_units = dead_units;

	// Upgrades
	*raw.upgrades.lock_write() = raw_player
		.get_upgrade_ids()
		.iter()
		.map(|u| UpgradeId::from_u32(*u).unwrap())
		.collect::<FxHashSet<_>>();

	// Map
	let map_state = res_raw.get_map_state();
	// Creep
	*raw.creep.lock_write() = PixelMap::from_proto(map_state.get_creep());

	// Available abilities
	let mut req = Request::new();
	let req_query_abilities = req.mut_query().mut_abilities();
	res_raw.get_units().iter().for_each(|u| {
		if matches!(u.get_alliance(), ProtoAlliance::value_Self) {
			let mut req_unit = RequestQueryAvailableAbilities::new();
			req_unit.set_unit_tag(u.get_tag());
			req_query_abilities.push(req_unit);
		}
	});

	let res = bot.api().send(req)?;
	*bot.abilities_units.lock_write() = res
		.get_query()
		.get_abilities()
		.iter()
		.map(|a| {
			(
				a.get_unit_tag(),
				a.get_abilities()
					.iter()
					.filter_map(|ab| AbilityId::from_i32(ab.get_ability_id()))
					.collect(),
			)
		})
		.collect();

	// Get visiblity
	let visibility = VisibilityMap::from_proto(map_state.get_visibility());
	// Get units
	let units = res_raw
		.get_units()
		.iter()
		.map(|u| Unit::from_proto(Rs::clone(&bot.data_for_unit), &visibility, u))
		.collect::<Units>();

	// Events
	for u in units.iter().filter(|u| u.is_mine()) {
		let tag = u.tag;

		if !bot.owned_tags.contains(&tag) {
			bot.owned_tags.insert(tag);
			if u.is_structure() {
				if !u.is_placeholder() && u.type_id != UnitTypeId::KD8Charge {
					if u.is_ready() {
						bot.on_event(Event::ConstructionComplete(tag))?;
					} else {
						bot.on_event(Event::ConstructionStarted(tag))?;
						bot.under_construction.insert(tag);
					}
				}
			} else {
				bot.on_event(Event::UnitCreated(tag))?;
			}
		} else if bot.under_construction.contains(&tag) && u.is_ready() {
			bot.under_construction.remove(&tag);
			bot.on_event(Event::ConstructionComplete(tag))?;
		}
	}

	// Set units
	bot.state.observation.raw.units = units;

	// Set visiblity
	bot.state.observation.raw.visibility = visibility;

	Ok(())
}

/// Messege in game chat.
#[derive(Clone)]
pub struct ChatMessage {
	/// Id of player who sent that message.
	pub player_id: u32,
	/// Actual message.
	pub message: String,
}

#[derive(Default, Clone)]
pub struct Observation {
	/// Current game tick (frame).
	pub game_loop: u32,
	pub common: Common,
	pub alerts: Vec<Alert>,
	pub abilities: Vec<AvailableAbility>,
	pub score: Score,
	pub raw: RawData,
}

#[derive(Default, Clone)]
pub struct RawData {
	pub psionic_matrix: Vec<PsionicMatrix>,
	pub camera: Point2,
	pub units: Units,
	pub upgrades: Rw<FxHashSet<UpgradeId>>,
	pub visibility: VisibilityMap,
	pub creep: Rw<PixelMap>,
	pub dead_units: Vec<u64>,
	pub effects: Vec<Effect>,
	pub radars: Vec<Radar>,
}

#[derive(Clone)]
pub struct PsionicMatrix {
	pub pos: Point2,
	pub radius: f32,
	pub tag: u64,
}
impl FromProto<&ProtoPowerSource> for PsionicMatrix {
	fn from_proto(ps: &ProtoPowerSource) -> Self {
		Self {
			pos: Point2::from_proto(ps.get_pos()),
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Alliance {
	Own,
	Ally,
	Neutral,
	Enemy,
}
impl Alliance {
	pub fn is_mine(self) -> bool {
		matches!(self, Alliance::Own)
	}
	pub fn is_enemy(self) -> bool {
		matches!(self, Alliance::Enemy)
	}
	pub fn is_neutral(self) -> bool {
		matches!(self, Alliance::Neutral)
	}
	pub fn is_ally(self) -> bool {
		matches!(self, Alliance::Ally)
	}
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
